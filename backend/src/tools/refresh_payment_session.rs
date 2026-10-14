use crate::availability::{Clock, SystemClock};
use crate::booking::{format_spoken_code, normalize_confirmation_code};
use crate::db::models::{Booking, BookingStatus, DepositStatus, RestaurantConfig};
use crate::payments::{self, PaymentObject};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for the `refresh_payment_session` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefreshPaymentSessionInput {
    /// The unique confirmation code of the reservation (e.g. 'TT-8K4MZ').
    pub confirmation_code: String,

    /// Guest name associated with the reservation. Required for identity verification.
    pub guest_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefreshPaymentSessionOutput {
    /// One of: refreshed | not_required | not_pending | deadline_passed | not_found.
    pub status: String,
    pub payment: Option<PaymentObject>,
    pub message: String,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

pub async fn handle_refresh_payment_session(
    pool: &SqlitePool,
    input: RefreshPaymentSessionInput,
) -> CallToolResult {
    handle_refresh_payment_session_with_clock(pool, input, &SystemClock).await
}

pub async fn handle_refresh_payment_session_with_clock(
    pool: &SqlitePool,
    input: RefreshPaymentSessionInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let code = input.confirmation_code.trim();
    if code.is_empty() {
        return tool_validation_error("Confirmation code is required (e.g. 'TT-8K4MZ').");
    }
    let name = input.guest_name.trim();
    if name.is_empty() {
        return tool_validation_error("Guest name is required for booking verification.");
    }
    let normalized = normalize_confirmation_code(code);
    let now_utc = clock.now();

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB connection error: {}", e)),
    };
    if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to begin transaction: {}", e));
    }

    let result: Result<CallToolResult, sqlx::Error> = async {
        let mut booking: Option<Booking> = sqlx::query_as(&format!(
            "SELECT {} FROM bookings WHERE confirmation_code = ?",
            crate::db::models::BOOKING_SELECT_COLUMNS
        ))
        .bind(&normalized)
        .fetch_optional(&mut *conn)
        .await?;

        let verified = booking
            .as_ref()
            .map(|b| b.guest_name.trim().eq_ignore_ascii_case(name))
            .unwrap_or(false);
        if !verified {
            let output = RefreshPaymentSessionOutput {
                status: "not_found".to_string(),
                payment: None,
                message:
                    "No matching booking found for the provided confirmation code and guest name."
                        .to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "I couldn't find a booking with that confirmation code and name.",
            )];
            return Ok(res);
        }

        let config: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents,
                   tax_rate_basis_points, last_order_before_close_minutes, phone,
                   deposit_max_amount_cents, deposit_payment_deadline_minutes,
                   deposit_refund_full_basis_points, deposit_refund_partial_basis_points,
                   deposit_refund_full_hours, deposit_refund_partial_hours,
                   deposit_peak_days, deposit_peak_start_time, deposit_peak_end_time,
                   deposit_demo_mode, deposit_decline_retry_limit
            FROM restaurant WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;
        let tz: Tz = config.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);

        // Lazy expiry inside this transaction.
        if let Some(ref b) = booking {
            if b.status == BookingStatus::Pending
                && b.deposit_status == DepositStatus::RequiresPayment
            {
                let _ = payments::lazy_expire_booking_in_conn(&mut conn, b, now_utc, &tz).await;
                booking = sqlx::query_as(&format!(
                    "SELECT {} FROM bookings WHERE confirmation_code = ?",
                    crate::db::models::BOOKING_SELECT_COLUMNS
                ))
                .bind(&normalized)
                .fetch_optional(&mut *conn)
                .await?;
            }
        }
        let booking = booking.unwrap();

        if booking.deposit_status == DepositStatus::NotRequired {
            let output = RefreshPaymentSessionOutput {
                status: "not_required".to_string(),
                payment: None,
                message: "No deposit is required for this booking, so there is nothing to pay."
                    .to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text("No payment is needed for this reservation.")];
            return Ok(res);
        }
        if booking.status != BookingStatus::Pending
            || booking.deposit_status != DepositStatus::RequiresPayment
        {
            let output = RefreshPaymentSessionOutput {
                status: "not_pending".to_string(),
                payment: None,
                message: "This booking is no longer awaiting payment.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "This reservation is no longer awaiting payment, so a new payment link was not created.",
            )];
            return Ok(res);
        }

        let deadline = match booking.payment_deadline_at.as_deref() {
            Some(s) => DateTime::parse_from_rfc3339(s)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now_utc),
            None => now_utc,
        };
        if now_utc >= deadline {
            let output = RefreshPaymentSessionOutput {
                status: "deadline_passed".to_string(),
                payment: None,
                message: "The payment deadline has passed; the hold was released.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(payments::voice_payment_expired())];
            return Ok(res);
        }

        // Invalidate any prior open session and issue a fresh one. The booking's
        // payment deadline is never extended: the new session expires at the same
        // deadline (or the deadline, whichever is sooner).
        payments::close_open_sessions_in_conn(&mut conn, &booking.id)
            .await
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to close sessions: {}", e)))?;

        let payment = payments::create_payment_session_in_conn(
            &mut conn,
            &booking.id,
            booking.deposit_amount_minor,
            &config.currency,
            deadline,
            now_utc,
            &tz,
            &payments::CheckoutConfig::default(),
        )
        .await
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to create session: {}", e)))?;

        let voice_text = payments::voice_deposit_required(
            &payments::DepositPolicy::from_config(&config),
            booking.deposit_amount_minor,
            &config.currency,
            booking.party_size,
            payment.minutes_remaining,
            &booking.confirmation_code,
        );
        let output = RefreshPaymentSessionOutput {
            status: "refreshed".to_string(),
            payment: Some(payment),
            message: format!(
                "A new simulated payment link was issued for booking {}. The payment deadline was not extended.",
                format_spoken_code(&booking.confirmation_code)
            ),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(voice_text)];
        Ok(res)
    }
    .await;

    match result {
        Ok(res) => {
            if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
                return tool_validation_error(format!("Failed to commit: {}", e));
            }
            res
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            tool_validation_error(format!("Failed to refresh payment session: {}", e))
        }
    }
}
