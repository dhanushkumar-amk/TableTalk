use crate::{
    availability::{Clock, SystemClock},
    negotiation::{hash_token, match_waitlist_for_slot, verify_token_hash},
};
use chrono::DateTime;
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseOfferInput {
    /// Offer bearer token (prefix 'tt_offer_') associated with the active hold.
    pub offer_token: String,
    /// Full name of the primary dining guest for verification.
    pub guest_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseOfferOutput {
    pub status: String,
    pub already_released: bool,
    pub message: String,
}

fn tool_validation_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

pub async fn handle_release_offer(pool: &SqlitePool, input: ReleaseOfferInput) -> CallToolResult {
    let clock = SystemClock;
    handle_release_offer_with_clock(pool, input, &clock).await
}

pub async fn handle_release_offer_with_clock(
    pool: &SqlitePool,
    input: ReleaseOfferInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let guest_name = input.guest_name.trim();
    if guest_name.is_empty() {
        return tool_validation_error("Guest name is required.");
    }
    let token_clean = input.offer_token.trim();
    if token_clean.is_empty() {
        return tool_validation_error("Offer token is required.");
    }

    let computed_hash = hash_token(token_clean);
    let now_iso = clock.now().to_rfc3339();

    let config = crate::db::repo::restaurant::get_config(pool).await.ok();
    let tz: Tz = config
        .as_ref()
        .and_then(|c| c.timezone.parse().ok())
        .unwrap_or(chrono_tz::Asia::Kolkata);

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB connection error: {}", e)),
    };

    if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to begin transaction: {}", e));
    }

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
            let output = ReleaseOfferOutput {
                status: "not_found".to_string(),
                already_released: false,
                message: "Offer not found or invalid offer token.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text("I couldn't find that active hold.")];
            return res;
        }
    };

    if !verify_token_hash(&computed_hash, &hold.token_hash)
        || !hold.guest_name.trim().eq_ignore_ascii_case(guest_name)
    {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = ReleaseOfferOutput {
            status: "not_found".to_string(),
            already_released: false,
            message: "Offer not found or invalid offer token.".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text("I couldn't find that active hold.")];
        return res;
    }

    if hold.status == crate::db::models::HoldStatus::Released {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = ReleaseOfferOutput {
            status: "released".to_string(),
            already_released: true,
            message: "This hold was already released (idempotent replay).".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text("This hold has already been released.")];
        return res;
    }

    let hold_id = hold.id;
    let table_id = hold.table_id;
    let start_at = hold.start_at;
    let end_at = hold.end_at;
    let wl_id = hold.waitlist_entry_id;

    // Release the active hold
    let _ = sqlx::query("UPDATE slot_holds SET status = 'released', updated_at = ? WHERE id = ?")
        .bind(&now_iso)
        .bind(hold_id)
        .execute(&mut *conn)
        .await;

    // If linked to a waitlist entry, revert entry status to waiting
    if let Some(w_id) = wl_id {
        let _ = sqlx::query(
            "UPDATE waitlist_entries SET status = 'waiting' WHERE id = ? AND status = 'offered'",
        )
        .bind(w_id)
        .execute(&mut *conn)
        .await;
    }

    if let Ok(dt_utc) = DateTime::parse_from_rfc3339(&start_at) {
        let local_dt = dt_utc.with_timezone(&tz);
        let date_str = local_dt.format("%Y-%m-%d").to_string();
        let time_str = local_dt.format("%H:%M").to_string();

        let t_area: String = sqlx::query_scalar("SELECT area FROM dining_tables WHERE id = ?")
            .bind(table_id)
            .fetch_one(&mut *conn)
            .await
            .unwrap_or_else(|_| "indoor".to_string());

        let _ = match_waitlist_for_slot(
            &mut conn, table_id, &start_at, &end_at, &t_area, &date_str, &time_str, clock,
        )
        .await;
    }

    if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to commit release: {}", e));
    }

    let output = ReleaseOfferOutput {
        status: "released".to_string(),
        already_released: false,
        message: "Table hold has been successfully released.".to_string(),
    };
    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(
        "I've released your held table. Let me know if you'd like to check other times.",
    )];
    res
}
