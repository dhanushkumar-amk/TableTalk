use crate::{
    availability::{Clock, SystemClock},
    booking::{format_spoken_code, BookingSummary},
    negotiation::{
        format_spoken_hold, generate_offer_token, hash_token, verify_token_hash, HoldSummary,
        StoredOfferSlot, DEFAULT_HOLD_MINUTES, MAX_ACTIVE_HOLDS_PER_GUEST,
    },
};
use chrono::{DateTime, Duration};
use chrono_tz::Tz;
use rand::Rng;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AcceptOfferInput {
    /// Negotiation ID if accepting an option from an active negotiation.
    pub negotiation_id: Option<i64>,
    /// Option identifier (e.g. 'opt_1') selected from the negotiation round.
    pub option_id: Option<String>,
    /// Offer bearer token (prefix 'tt_offer_') if confirming an active hold.
    pub offer_token: Option<String>,
    /// Full name of the primary dining guest for verification.
    pub guest_name: String,
    /// Optional contact phone number for confirmation SMS.
    pub phone: Option<String>,
    /// Optional contact email address for booking receipt.
    pub email: Option<String>,
    /// Optional special requests, dietary preferences, or celebration notes.
    pub special_requests: Option<String>,
    /// Optional client idempotency key to prevent double bookings on retries.
    pub idempotency_key: Option<String>,
    /// When true, places a short-lived hold on the table instead of immediately creating a booking (default false).
    pub hold_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AcceptOfferOutput {
    pub status: String,
    pub booking: Option<BookingSummary>,
    pub hold: Option<HoldSummary>,
    pub offer_token: Option<String>,
    pub message: String,
}

fn tool_validation_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

pub async fn handle_accept_offer(pool: &SqlitePool, input: AcceptOfferInput) -> CallToolResult {
    let clock = SystemClock;
    handle_accept_offer_with_clock(pool, input, &clock).await
}

pub async fn handle_accept_offer_with_clock(
    pool: &SqlitePool,
    input: AcceptOfferInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let guest_name = input.guest_name.trim();
    if guest_name.is_empty() {
        return tool_validation_error("Guest name is required.");
    }

    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB connection error: {}", e)),
    };

    // Load restaurant config
    let config = match crate::db::repo::restaurant::get_config(pool).await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("Failed to load config: {}", e)),
    };
    let tz: Tz = config.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);

    // --- CASE A: Converting an existing hold using offer_token ---
    if let Some(ref raw_token) = input.offer_token {
        let clean_token = raw_token.trim();
        let computed_hash = hash_token(clean_token);

        if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
            return tool_validation_error(format!("Failed to begin transaction: {}", e));
        }

        // Look up hold by hash
        let hold: Option<crate::db::models::SlotHold> = sqlx::query_as(
            "SELECT id, table_id, start_at, end_at, status, expires_at, guest_name, party_size, area, token_hash, negotiation_id, waitlist_entry_id, created_at, updated_at FROM slot_holds WHERE token_hash = ?"
        )
        .bind(&computed_hash)
        .fetch_optional(&mut *conn)
        .await
        .unwrap_or_default();

        let hold = match hold {
            Some(h) => h,
            None => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                let output = AcceptOfferOutput {
                    status: "offer_not_found".to_string(),
                    booking: None,
                    hold: None,
                    offer_token: None,
                    message: "Offer not found or invalid offer token.".to_string(),
                };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text("I couldn't find that held offer. It may have expired or already been released.")];
                return res;
            }
        };

        // Verification check: constant-time hash and guest name match
        if !verify_token_hash(&computed_hash, &hold.token_hash)
            || !hold.guest_name.trim().eq_ignore_ascii_case(guest_name)
        {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            let output = AcceptOfferOutput {
                status: "offer_not_found".to_string(),
                booking: None,
                hold: None,
                offer_token: None,
                message: "Offer not found or invalid offer token.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "I couldn't find that held offer. It may have expired or already been released.",
            )];
            return res;
        }

        // Idempotency replay check if hold was already converted
        if hold.status == crate::db::models::HoldStatus::Converted {
            // Find existing booking
            let existing_booking: Option<crate::db::models::Booking> = sqlx::query_as(
                "SELECT * FROM bookings WHERE table_id = ? AND start_at = ? AND end_at = ? AND status = 'confirmed' LIMIT 1"
            )
            .bind(hold.table_id)
            .bind(&hold.start_at)
            .bind(&hold.end_at)
            .fetch_optional(&mut *conn)
            .await
            .unwrap_or(None);

            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            if let Some(b) = existing_booking {
                let summary =
                    match crate::booking::build_booking_summary(&b, &config, &mut conn).await {
                        Ok(mut s) => {
                            s.source = Some("negotiation".to_string());
                            s
                        }
                        Err(_) => {
                            return tool_validation_error("Failed to build booking summary.");
                        }
                    };
                let spoken_code = format_spoken_code(&summary.confirmation_code);
                let voice_text = format!(
                    "Your reservation is confirmed! Your confirmation code is {}.",
                    spoken_code
                );
                let output = AcceptOfferOutput {
                    status: "confirmed".to_string(),
                    booking: Some(summary),
                    hold: None,
                    offer_token: None,
                    message: "Reservation confirmed (idempotent replay).".to_string(),
                };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text(voice_text)];
                return res;
            }
        }

        if hold.status == crate::db::models::HoldStatus::Released {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            let output = AcceptOfferOutput {
                status: "offer_released".to_string(),
                booking: None,
                hold: None,
                offer_token: None,
                message: "This held offer was previously released.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text("This hold was previously released. Would you like to search for fresh availability?")];
            return res;
        }

        let is_expired =
            hold.expires_at <= now_iso || hold.status == crate::db::models::HoldStatus::Expired;
        if is_expired {
            // Re-check if table is still free
            let overlap: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM bookings WHERE table_id = ? AND status IN ('pending', 'confirmed') AND start_at < ? AND end_at > ?"
            )
            .bind(hold.table_id)
            .bind(&hold.end_at)
            .bind(&hold.start_at)
            .fetch_one(&mut *conn)
            .await
            .unwrap_or(1);

            if overlap > 0 {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                let output = AcceptOfferOutput {
                    status: "offer_expired".to_string(),
                    booking: None,
                    hold: None,
                    offer_token: None,
                    message: "This hold has expired and the table is no longer available. Please search for fresh options.".to_string(),
                };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text("Your hold has expired and that table is no longer available. Would you like to check other times?")];
                return res;
            }
        }

        let hold_id = hold.id;
        let table_id = hold.table_id;
        let start_at = hold.start_at;
        let end_at = hold.end_at;
        let party_size = hold.party_size;
        let area = hold.area;
        let neg_id = hold.negotiation_id;
        let wl_id = hold.waitlist_entry_id;

        // Convert hold to confirmed booking!
        let code_suffix: String = (0..5)
            .map(|_| {
                let charset = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
                let idx = rand::thread_rng().gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();
        let confirmation_code = format!("BK-{}", code_suffix);
        let booking_id = uuid::Uuid::new_v4().to_string();

        let insert_res = sqlx::query(
            r#"
            INSERT INTO bookings (
                id, confirmation_code, guest_name, party_size,
                start_at, end_at, status, table_id, special_requests,
                deposit_status, deposit_amount_cents, created_at, updated_at,
                guest_phone, guest_email
            ) VALUES (?, ?, ?, ?, ?, ?, 'confirmed', ?, ?, 'none', 0, ?, ?, ?, ?)
            "#,
        )
        .bind(&booking_id)
        .bind(&confirmation_code)
        .bind(guest_name)
        .bind(party_size)
        .bind(&start_at)
        .bind(&end_at)
        .bind(table_id)
        .bind(input.special_requests.as_deref())
        .bind(&now_iso)
        .bind(&now_iso)
        .bind(input.phone.as_deref())
        .bind(input.email.as_deref())
        .execute(&mut *conn)
        .await;

        if let Err(e) = insert_res {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return tool_validation_error(format!("Failed to insert booking: {}", e));
        }

        // Update hold to converted
        let _ =
            sqlx::query("UPDATE slot_holds SET status = 'converted', updated_at = ? WHERE id = ?")
                .bind(&now_iso)
                .bind(hold_id)
                .execute(&mut *conn)
                .await;

        if let Some(n_id) = neg_id {
            let _ = sqlx::query("UPDATE negotiations SET status = 'accepted' WHERE id = ?")
                .bind(n_id)
                .execute(&mut *conn)
                .await;
        }

        if let Some(w_id) = wl_id {
            let _ = sqlx::query("UPDATE waitlist_entries SET status = 'booked' WHERE id = ?")
                .bind(w_id)
                .execute(&mut *conn)
                .await;
        }

        if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
            return tool_validation_error(format!("Failed to commit booking: {}", e));
        }

        // Fetch table label
        let table_label: String =
            sqlx::query_scalar("SELECT label FROM dining_tables WHERE id = ?")
                .bind(table_id)
                .fetch_one(pool)
                .await
                .unwrap_or_else(|_| "T1".to_string());

        let dt_start = DateTime::parse_from_rfc3339(&start_at)
            .unwrap()
            .with_timezone(&tz);
        let dt_end = DateTime::parse_from_rfc3339(&end_at)
            .unwrap()
            .with_timezone(&tz);

        let summary = BookingSummary {
            confirmation_code: confirmation_code.clone(),
            guest_name: guest_name.to_string(),
            party_size,
            start_local: dt_start.format("%H:%M").to_string(),
            end_local: dt_end.format("%H:%M").to_string(),
            area,
            table_label,
            status: "confirmed".to_string(),
            special_requests: input.special_requests,
            created_at: now_iso,
            source: Some("negotiation".to_string()),
        };

        let spoken_code = format_spoken_code(&confirmation_code);
        let voice_text = format!(
            "Your reservation is confirmed! Your confirmation code is {}.",
            spoken_code
        );
        let output = AcceptOfferOutput {
            status: "confirmed".to_string(),
            booking: Some(summary),
            hold: None,
            offer_token: None,
            message: "Reservation confirmed successfully from hold.".to_string(),
        };

        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(voice_text)];
        return res;
    }

    // --- CASE B: Negotiation option specified (direct booking or hold_only) ---
    let neg_id = match input.negotiation_id {
        Some(id) => id,
        None => {
            return tool_validation_error(
                "Either 'offer_token' or 'negotiation_id' + 'option_id' is required.",
            )
        }
    };
    let opt_id = match input.option_id {
        Some(ref id) => id.trim(),
        None => {
            return tool_validation_error(
                "'option_id' is required when accepting a negotiation offer.",
            )
        }
    };

    if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to begin transaction: {}", e));
    }

    // Load negotiation
    let neg: Option<crate::db::models::Negotiation> = sqlx::query_as(
        "SELECT id, guest_name, date, time, party_size, area, duration_minutes, round, offered_slots, status, created_at, expires_at FROM negotiations WHERE id = ?"
    )
    .bind(neg_id)
    .fetch_optional(&mut *conn)
    .await
    .unwrap_or(None);

    let neg = match neg {
        Some(n) => n,
        None => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return tool_validation_error("Negotiation session not found.");
        }
    };

    if !neg.guest_name.trim().eq_ignore_ascii_case(guest_name) {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        return tool_validation_error("Guest name does not match the negotiation record.");
    }

    if neg.expires_at <= now_iso || neg.status != crate::db::models::NegotiationStatus::Open {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = AcceptOfferOutput {
            status: "negotiation_expired".to_string(),
            booking: None,
            hold: None,
            offer_token: None,
            message: "Negotiation session has expired. Please perform a fresh reservation check."
                .to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(
            "That negotiation session has expired. Would you like to check fresh availability?",
        )];
        return res;
    }

    let offered: Vec<StoredOfferSlot> =
        serde_json::from_str(&neg.offered_slots).unwrap_or_default();
    let slot = match offered.iter().find(|s| s.option_id == opt_id) {
        Some(s) => s.clone(),
        None => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return tool_validation_error(format!(
                "Option '{}' not found in this negotiation session.",
                opt_id
            ));
        }
    };

    // Re-check table availability for this slot inside the lock
    let overlap_count: i64 = sqlx::query_scalar(
        r#"
        SELECT (
            (SELECT COUNT(*) FROM bookings WHERE table_id = ? AND status IN ('pending', 'confirmed') AND start_at < ? AND end_at > ?)
            +
            (SELECT COUNT(*) FROM slot_holds WHERE table_id = ? AND status = 'active' AND expires_at > ? AND start_at < ? AND end_at > ?)
        )
        "#,
    )
    .bind(slot.table_id)
    .bind(&slot.end_at)
    .bind(&slot.start_at)
    .bind(slot.table_id)
    .bind(&now_iso)
    .bind(&slot.end_at)
    .bind(&slot.start_at)
    .fetch_one(&mut *conn)
    .await
    .unwrap_or(1);

    if overlap_count > 0 {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = AcceptOfferOutput {
            status: "unavailable".to_string(),
            booking: None,
            hold: None,
            offer_token: None,
            message: "The selected dining option is no longer available.".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text("That table was just taken by another guest. Would you like me to check other available times?")];
        return res;
    }

    let hold_only = input.hold_only.unwrap_or(false);

    if hold_only {
        // Check active holds cap for guest
        let active_holds: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM slot_holds WHERE guest_name = ? AND status = 'active' AND expires_at > ?"
        )
        .bind(guest_name)
        .bind(&now_iso)
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(0);

        if active_holds >= MAX_ACTIVE_HOLDS_PER_GUEST {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            return tool_validation_error("Maximum active holds reached for this guest. Please confirm or release existing holds.");
        }

        let (plain_token, token_hash) = generate_offer_token();
        let hold_expires = now_utc + Duration::minutes(DEFAULT_HOLD_MINUTES);
        let hold_expires_iso = hold_expires.to_rfc3339();
        let hold_expires_local = hold_expires.with_timezone(&tz).format("%H:%M").to_string();

        let hold_id = match sqlx::query(
            r#"
            INSERT INTO slot_holds (
                table_id, start_at, end_at, status, expires_at,
                guest_name, party_size, area, token_hash,
                negotiation_id, created_at, updated_at
            ) VALUES (?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(slot.table_id)
        .bind(&slot.start_at)
        .bind(&slot.end_at)
        .bind(&hold_expires_iso)
        .bind(guest_name)
        .bind(slot.party_size)
        .bind(&slot.area)
        .bind(&token_hash)
        .bind(neg_id)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(&mut *conn)
        .await
        {
            Ok(r) => r.last_insert_rowid(),
            Err(e) => {
                let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
                return tool_validation_error(format!("Failed to place hold: {}", e));
            }
        };

        if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
            return tool_validation_error(format!("Failed to commit hold: {}", e));
        }

        let hold_summary = HoldSummary {
            hold_id,
            expires_at_local: hold_expires_local,
            seconds_remaining: DEFAULT_HOLD_MINUTES * 60,
            area: slot.area.clone(),
            start_local: slot.start_local.clone(),
            end_local: slot.end_local.clone(),
            table_label: slot.table_label.clone(),
            party_size: slot.party_size,
        };

        let voice_text = format_spoken_hold(&slot.area, &slot.start_local, DEFAULT_HOLD_MINUTES);
        let output = AcceptOfferOutput {
            status: "hold_placed".to_string(),
            booking: None,
            hold: Some(hold_summary),
            offer_token: Some(plain_token),
            message: format!(
                "Table hold successfully placed for {} minutes. Offer token returned.",
                DEFAULT_HOLD_MINUTES
            ),
        };

        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(voice_text)];
        return res;
    }

    // Direct booking without prior hold
    let code_suffix: String = (0..5)
        .map(|_| {
            let charset = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
            let idx = rand::thread_rng().gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();
    let confirmation_code = format!("BK-{}", code_suffix);
    let booking_id = uuid::Uuid::new_v4().to_string();

    let insert_res = sqlx::query(
        r#"
        INSERT INTO bookings (
            id, confirmation_code, guest_name, party_size,
            start_at, end_at, status, table_id, special_requests,
            deposit_status, deposit_amount_cents, created_at, updated_at,
            guest_phone, guest_email
        ) VALUES (?, ?, ?, ?, ?, ?, 'confirmed', ?, ?, 'none', 0, ?, ?, ?, ?)
        "#,
    )
    .bind(&booking_id)
    .bind(&confirmation_code)
    .bind(guest_name)
    .bind(slot.party_size)
    .bind(&slot.start_at)
    .bind(&slot.end_at)
    .bind(slot.table_id)
    .bind(input.special_requests.as_deref())
    .bind(&now_iso)
    .bind(&now_iso)
    .bind(input.phone.as_deref())
    .bind(input.email.as_deref())
    .execute(&mut *conn)
    .await;

    if let Err(e) = insert_res {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        return tool_validation_error(format!("Failed to insert booking: {}", e));
    }

    let _ = sqlx::query("UPDATE negotiations SET status = 'accepted' WHERE id = ?")
        .bind(neg_id)
        .execute(&mut *conn)
        .await;

    if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to commit booking: {}", e));
    }

    let summary = BookingSummary {
        confirmation_code: confirmation_code.clone(),
        guest_name: guest_name.to_string(),
        party_size: slot.party_size,
        start_local: slot.start_local,
        end_local: slot.end_local,
        area: slot.area,
        table_label: slot.table_label,
        status: "confirmed".to_string(),
        special_requests: input.special_requests,
        created_at: now_iso,
        source: Some("negotiation".to_string()),
    };

    let spoken_code = format_spoken_code(&confirmation_code);
    let voice_text = format!(
        "Your reservation is confirmed! Your confirmation code is {}.",
        spoken_code
    );
    let output = AcceptOfferOutput {
        status: "confirmed".to_string(),
        booking: Some(summary),
        hold: None,
        offer_token: None,
        message: "Reservation confirmed successfully from negotiation.".to_string(),
    };

    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(voice_text)];
    res
}
