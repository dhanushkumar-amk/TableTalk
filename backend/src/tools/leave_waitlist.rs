use crate::{
    availability::{Clock, SystemClock},
    negotiation::match_waitlist_for_slot,
};
use chrono::DateTime;
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LeaveWaitlistInput {
    /// Waitlist entry code (e.g. 'WL-8K4MZ').
    pub code: String,
    /// Full name of the dining guest for verification.
    pub guest_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LeaveWaitlistOutput {
    pub status: String,
    pub code: String,
    pub already_cancelled: bool,
    pub message: String,
}

pub async fn handle_leave_waitlist(pool: &SqlitePool, input: LeaveWaitlistInput) -> CallToolResult {
    let clock = SystemClock;
    handle_leave_waitlist_with_clock(pool, input, &clock).await
}

pub async fn handle_leave_waitlist_with_clock(
    pool: &SqlitePool,
    input: LeaveWaitlistInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let code_clean = match crate::validation::sanitize_string(&input.code, "code", 1, 64) {
        Ok(c) => c.to_uppercase(),
        Err(err) => return err,
    };
    let guest_name =
        match crate::validation::sanitize_string(&input.guest_name, "guest_name", 1, 100) {
            Ok(n) => n,
            Err(err) => return err,
        };

    let config = crate::db::repo::restaurant::get_config(pool).await.ok();
    let tz: Tz = config
        .as_ref()
        .and_then(|c| c.timezone.parse().ok())
        .unwrap_or(chrono_tz::Asia::Kolkata);

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return crate::validation::internal_tool_error("acquiring connection", e),
    };

    if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
        return crate::validation::internal_tool_error("beginning transaction", e);
    }

    let entry_row: Option<(i64, String, String, String)> =
        sqlx::query_as("SELECT id, code, guest_name, status FROM waitlist_entries WHERE code = ?")
            .bind(&code_clean)
            .fetch_optional(&mut *conn)
            .await
            .unwrap_or_default();

    let (entry_id, code, e_guest, status) = match entry_row {
        Some(r) => r,
        None => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            let output = LeaveWaitlistOutput {
                status: "not_found".to_string(),
                code: code_clean,
                already_cancelled: false,
                message: "Waitlist entry not found or invalid code.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "I couldn't find a waitlist reservation with those details.",
            )];
            return res;
        }
    };

    if !e_guest.trim().eq_ignore_ascii_case(&guest_name) {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = LeaveWaitlistOutput {
            status: "not_found".to_string(),
            code: code_clean,
            already_cancelled: false,
            message: "Waitlist entry not found or invalid code.".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(
            "I couldn't find a waitlist reservation with those details.",
        )];
        return res;
    }

    if status == "cancelled" {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let output = LeaveWaitlistOutput {
            status: "cancelled".to_string(),
            code,
            already_cancelled: true,
            message: "This waitlist entry was already cancelled (idempotent replay).".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(
            "Your waitlist entry has already been cancelled.",
        )];
        return res;
    }

    let now_iso = clock.now().to_rfc3339();

    // If entry had an active hold, release it and run matcher
    let hold_row: Option<(i64, i64, String, String, String)> = sqlx::query_as(
        "SELECT id, table_id, start_at, end_at, area FROM slot_holds WHERE waitlist_entry_id = ? AND status = 'active'"
    )
    .bind(entry_id)
    .fetch_optional(&mut *conn)
    .await
    .unwrap_or(None);

    if let Some((hold_id, table_id, start_at, end_at, area)) = hold_row {
        let _ =
            sqlx::query("UPDATE slot_holds SET status = 'released', updated_at = ? WHERE id = ?")
                .bind(&now_iso)
                .bind(hold_id)
                .execute(&mut *conn)
                .await;

        if let Ok(dt_utc) = DateTime::parse_from_rfc3339(&start_at) {
            let local_dt = dt_utc.with_timezone(&tz);
            let date_str = local_dt.format("%Y-%m-%d").to_string();
            let time_str = local_dt.format("%H:%M").to_string();
            let _ = match_waitlist_for_slot(
                &mut conn, table_id, &start_at, &end_at, &area, &date_str, &time_str, clock,
            )
            .await;
        }
    }

    let _ = sqlx::query("UPDATE waitlist_entries SET status = 'cancelled' WHERE id = ?")
        .bind(entry_id)
        .execute(&mut *conn)
        .await;

    if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
        return crate::validation::internal_tool_error("committing cancellation", e);
    }

    let output = LeaveWaitlistOutput {
        status: "cancelled".to_string(),
        code,
        already_cancelled: false,
        message: "You have been successfully removed from the waitlist.".to_string(),
    };

    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text("I've removed you from the waitlist.")];
    res
}
