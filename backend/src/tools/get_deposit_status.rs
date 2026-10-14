use crate::availability::{Clock, SystemClock};
use crate::booking::normalize_confirmation_code;
use crate::db::models::{Booking, BookingStatus, DepositStatus, RestaurantConfig};
use crate::payments::{self, DepositPolicy, Money, RefundSummary};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for the `get_deposit_status` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetDepositStatusInput {
    /// The unique confirmation code of the reservation (e.g. 'TT-8K4MZ').
    pub confirmation_code: String,

    /// Guest name associated with the reservation. Required for identity verification.
    pub guest_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DepositStatusMoney {
    pub amount: Money,
    pub percent_bps: i64,
    pub percent_display: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetDepositStatusOutput {
    pub status: String,
    pub required: bool,
    pub deposit_status: Option<String>,
    pub amount: Option<Money>,
    pub deadline_local: Option<String>,
    pub minutes_remaining: Option<i64>,
    /// What the caller would receive if they cancelled right now.
    pub refund_if_cancelled_now: Option<DepositStatusMoney>,
    pub message: String,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

pub async fn handle_get_deposit_status(
    pool: &SqlitePool,
    input: GetDepositStatusInput,
) -> CallToolResult {
    handle_get_deposit_status_with_clock(pool, input, &SystemClock).await
}

pub async fn handle_get_deposit_status_with_clock(
    pool: &SqlitePool,
    input: GetDepositStatusInput,
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

    let mut booking: Option<Booking> = match sqlx::query_as(&format!(
        "SELECT {} FROM bookings WHERE confirmation_code = ?",
        crate::db::models::BOOKING_SELECT_COLUMNS
    ))
    .bind(&normalized)
    .fetch_optional(pool)
    .await
    {
        Ok(b) => b,
        Err(e) => {
            return tool_validation_error(format!("DB error: {}", e));
        }
    };

    let verified = booking
        .as_ref()
        .map(|b| b.guest_name.trim().eq_ignore_ascii_case(name))
        .unwrap_or(false);
    if !verified {
        let output = not_found_output();
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(
            "I couldn't find a booking with that confirmation code and name. Please check the code and the name on the booking.",
        )];
        return res;
    }

    // Lazy expiry before reporting.
    if let Some(ref b) = booking {
        if b.status == BookingStatus::Pending && b.deposit_status == DepositStatus::RequiresPayment
        {
            let _ = payments::lazy_expire_booking_by_id(pool, &b.id, clock).await;
            booking = sqlx::query_as(&format!(
                "SELECT {} FROM bookings WHERE confirmation_code = ?",
                crate::db::models::BOOKING_SELECT_COLUMNS
            ))
            .bind(&normalized)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
        }
    }
    let booking = match booking {
        Some(b) => b,
        None => {
            let output = not_found_output();
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "I couldn't find a booking with that confirmation code and name.",
            )];
            return res;
        }
    };

    let config: Result<RestaurantConfig, sqlx::Error> = sqlx::query_as(
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
    .fetch_one(pool)
    .await;
    let config = match config {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("Failed to load config: {}", e)),
    };
    let tz: Tz = config.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);
    let policy = DepositPolicy::from_config(&config);
    let required = booking.deposit_status != DepositStatus::NotRequired;

    let amount = if booking.deposit_amount_minor > 0 {
        Some(payments::money(
            booking.deposit_amount_minor,
            &config.currency,
        ))
    } else {
        None
    };

    let deadline_local = booking
        .payment_deadline_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&tz).to_rfc3339());
    let minutes = booking
        .payment_deadline_at
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| payments::minutes_remaining(d.with_timezone(&Utc), clock.now()));

    let refund_if_cancelled_now = if booking.deposit_status == DepositStatus::Paid {
        let start = DateTime::parse_from_rfc3339(&booking.start_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| clock.now());
        let minutes_until = (start - clock.now()).num_minutes();
        let quote = payments::compute_refund(booking.deposit_amount_minor, minutes_until, &policy);
        let summary = RefundSummary {
            amount: payments::money(quote.refund_minor, &config.currency),
            percent_bps: quote.bps,
            percent_display: format!("{}%", quote.bps / 100),
            explanation: if quote.refund_minor <= 0 {
                "If you cancel now, no refund applies.".to_string()
            } else {
                format!(
                    "If you cancel now, you would receive {}.",
                    payments::money(quote.refund_minor, &config.currency).display
                )
            },
        };
        Some(DepositStatusMoney {
            amount: summary.amount,
            percent_bps: summary.percent_bps,
            percent_display: summary.percent_display,
            explanation: summary.explanation,
        })
    } else {
        None
    };

    let message = match booking.deposit_status {
        DepositStatus::NotRequired => payments::voice_deposit_not_required(),
        DepositStatus::Paid => {
            payments::voice_deposit_paid(booking.deposit_amount_minor, &config.currency)
        }
        DepositStatus::RequiresPayment => payments::voice_deposit_pending(
            booking.deposit_amount_minor,
            &config.currency,
            minutes.unwrap_or(0),
        ),
        DepositStatus::Refunded | DepositStatus::PartiallyRefunded => {
            "Your deposit has already been refunded.".to_string()
        }
        DepositStatus::Forfeited => "The deposit was retained; no refund applies.".to_string(),
        DepositStatus::Expired => payments::voice_payment_expired(),
    };

    let output = GetDepositStatusOutput {
        status: "found".to_string(),
        required,
        deposit_status: Some(booking.deposit_status.as_str().to_string()),
        amount,
        deadline_local,
        minutes_remaining: minutes,
        refund_if_cancelled_now,
        message,
    };
    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(output.message.clone())];
    if res.content.is_empty() {
        res.content = vec![ContentBlock::text("Deposit status retrieved.")];
    }
    res
}

fn not_found_output() -> GetDepositStatusOutput {
    GetDepositStatusOutput {
        status: "not_found".to_string(),
        required: false,
        deposit_status: None,
        amount: None,
        deadline_local: None,
        minutes_remaining: None,
        refund_if_cancelled_now: None,
        message: "No matching booking found for the provided confirmation code and guest name."
            .to_string(),
    }
}
