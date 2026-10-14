use crate::{
    availability::{Clock, SystemClock},
    negotiation::{
        format_spoken_waitlist_offer, generate_waitlist_offer_token, match_waitlist_for_slot,
        WaitlistOfferDetail, WaitlistSummary,
    },
};
use chrono::DateTime;
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetWaitlistStatusInput {
    /// Waitlist entry code (e.g. 'WL-8K4MZ').
    pub code: String,
    /// Full name of the dining guest for verification.
    pub guest_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetWaitlistStatusOutput {
    pub status: String,
    pub waitlist: Option<WaitlistSummary>,
    pub message: String,
}

fn tool_validation_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

pub async fn handle_get_waitlist_status(
    pool: &SqlitePool,
    input: GetWaitlistStatusInput,
) -> CallToolResult {
    let clock = SystemClock;
    handle_get_waitlist_status_with_clock(pool, input, &clock).await
}

pub async fn handle_get_waitlist_status_with_clock(
    pool: &SqlitePool,
    input: GetWaitlistStatusInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let code_clean = input.code.trim().to_uppercase();
    let guest_name = input.guest_name.trim();

    if code_clean.is_empty() {
        return tool_validation_error("Waitlist code is required.");
    }
    if guest_name.is_empty() {
        return tool_validation_error("Guest name is required.");
    }

    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();

    let config = crate::db::repo::restaurant::get_config(pool).await.ok();
    let tz: Tz = config
        .as_ref()
        .and_then(|c| c.timezone.parse().ok())
        .unwrap_or(chrono_tz::Asia::Kolkata);

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB connection error: {}", e)),
    };

    let entry: Option<crate::db::models::WaitlistEntry> = sqlx::query_as(
        r#"
        SELECT id, code, guest_name, contact, party_size, date,
               window_start, window_end, area, duration_minutes, status,
               customer_id, created_at, expires_at
        FROM waitlist_entries
        WHERE code = ?
        "#,
    )
    .bind(&code_clean)
    .fetch_optional(&mut *conn)
    .await
    .unwrap_or_default();

    let entry = match entry {
        Some(r) => r,
        None => {
            let output = GetWaitlistStatusOutput {
                status: "not_found".to_string(),
                waitlist: None,
                message: "Waitlist entry not found or invalid code.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "I couldn't find a waitlist reservation with those details.",
            )];
            return res;
        }
    };

    if !entry.guest_name.trim().eq_ignore_ascii_case(guest_name) {
        let output = GetWaitlistStatusOutput {
            status: "not_found".to_string(),
            waitlist: None,
            message: "Waitlist entry not found or invalid code.".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(
            "I couldn't find a waitlist reservation with those details.",
        )];
        return res;
    }

    let entry_id = entry.id;
    let code = entry.code;
    let party_size = entry.party_size;
    let date = entry.date;
    let win_start = entry.window_start;
    let win_end = entry.window_end;
    let area = entry.area;
    let mut status = match entry.status {
        crate::db::models::WaitlistStatus::Waiting => "waiting".to_string(),
        crate::db::models::WaitlistStatus::Offered => "offered".to_string(),
        crate::db::models::WaitlistStatus::Booked => "booked".to_string(),
        crate::db::models::WaitlistStatus::Cancelled => "cancelled".to_string(),
        crate::db::models::WaitlistStatus::Expired => "expired".to_string(),
    };
    let created_at = entry.created_at;
    let expires_at = entry.expires_at;

    // Lazy expiry check
    if status == "waiting" && expires_at <= now_iso {
        let _ = sqlx::query("UPDATE waitlist_entries SET status = 'expired' WHERE id = ?")
            .bind(entry_id)
            .execute(&mut *conn)
            .await;
        status = "expired".to_string();
    }

    let mut offer_detail = None;

    // Check if offered
    if status == "offered" {
        // Query associated hold
        let hold_row: Option<(i64, i64, String, String, String, String, String)> = sqlx::query_as(
            r#"
            SELECT id, table_id, start_at, end_at, status, expires_at, area
            FROM slot_holds
            WHERE waitlist_entry_id = ?
            ORDER BY id DESC LIMIT 1
            "#,
        )
        .bind(entry_id)
        .fetch_optional(&mut *conn)
        .await
        .unwrap_or(None);

        if let Some((h_id, h_table_id, h_start, h_end, h_status, h_expires, h_area)) = hold_row {
            if h_status == "active" && h_expires > now_iso {
                // Active hold! Generate deterministic offer token
                let (offer_token, _) = generate_waitlist_offer_token(h_id, &code, &created_at);
                let dt_start = DateTime::parse_from_rfc3339(&h_start)
                    .unwrap()
                    .with_timezone(&tz);
                let dt_end = DateTime::parse_from_rfc3339(&h_end)
                    .unwrap()
                    .with_timezone(&tz);
                let dt_exp = DateTime::parse_from_rfc3339(&h_expires)
                    .unwrap()
                    .with_timezone(&tz);

                let exp_utc = DateTime::parse_from_rfc3339(&h_expires)
                    .unwrap()
                    .with_timezone(&chrono::Utc);
                let secs_remaining = (exp_utc - now_utc).num_seconds().max(0);

                let table_label: String =
                    sqlx::query_scalar("SELECT label FROM dining_tables WHERE id = ?")
                        .bind(h_table_id)
                        .fetch_one(&mut *conn)
                        .await
                        .unwrap_or_else(|_| "T1".to_string());

                offer_detail = Some(WaitlistOfferDetail {
                    offer_token,
                    start_local: dt_start.format("%H:%M").to_string(),
                    end_local: dt_end.format("%H:%M").to_string(),
                    area: h_area,
                    table_label,
                    expires_at_local: dt_exp.format("%H:%M").to_string(),
                    seconds_remaining: secs_remaining,
                });
            } else if h_expires <= now_iso || h_status == "expired" {
                // Hold expired! Re-queue entry to waiting at original position
                let _ = sqlx::query("UPDATE waitlist_entries SET status = 'waiting' WHERE id = ?")
                    .bind(entry_id)
                    .execute(&mut *conn)
                    .await;
                status = "waiting".to_string();

                // Trigger waitlist match for next candidate
                if let Ok(dt_utc) = DateTime::parse_from_rfc3339(&h_start) {
                    let local_dt = dt_utc.with_timezone(&tz);
                    let date_str = local_dt.format("%Y-%m-%d").to_string();
                    let time_str = local_dt.format("%H:%M").to_string();
                    let _ = match_waitlist_for_slot(
                        &mut conn, h_table_id, &h_start, &h_end, &h_area, &date_str, &time_str,
                        clock,
                    )
                    .await;
                }
            }
        }
    }

    let queue_position = if status == "waiting" {
        let pos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM waitlist_entries WHERE date = ? AND status = 'waiting' AND created_at < ?"
        )
        .bind(&date)
        .bind(&created_at)
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(0) + 1;
        Some(pos)
    } else {
        None
    };

    let voice_text = match status.as_str() {
        "offered" => {
            if let Some(ref o) = offer_detail {
                let mins = (o.seconds_remaining / 60).max(1);
                format_spoken_waitlist_offer(party_size, &o.area, &o.start_local, mins)
            } else {
                "A table has opened up for you! Please call accept_offer to confirm.".to_string()
            }
        }
        "waiting" => {
            let pos = queue_position.unwrap_or(1);
            format!(
                "You are currently number {} on the waitlist for {}. No table has opened up yet. Please check back later.",
                pos, date
            )
        }
        "booked" => "Your waitlist reservation has been confirmed into a booking.".to_string(),
        "cancelled" => "Your waitlist entry has been cancelled.".to_string(),
        "expired" => "Your waitlist entry has expired for the day.".to_string(),
        _ => "Your waitlist entry status is updated.".to_string(),
    };

    let summary = WaitlistSummary {
        code,
        guest_name: guest_name.to_string(),
        party_size,
        date,
        window_start: win_start,
        window_end: win_end,
        area,
        status: status.clone(),
        queue_position,
        expires_at_local: "End of service day".to_string(),
        offer: offer_detail,
    };

    let output = GetWaitlistStatusOutput {
        status,
        waitlist: Some(summary),
        message: "Waitlist status retrieved successfully.".to_string(),
    };

    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(voice_text)];
    res
}
