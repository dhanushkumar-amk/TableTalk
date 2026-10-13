use crate::{
    availability::{Clock, SystemClock},
    negotiation::{format_spoken_waitlist_code, MAX_ACTIVE_WAITLIST_PER_GUEST},
};
use chrono::{Datelike, NaiveDate, NaiveTime};
use rand::Rng;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JoinWaitlistInput {
    /// Full name of the dining guest.
    pub guest_name: String,
    /// Party size (number of guests).
    pub party_size: i64,
    /// Local restaurant date in YYYY-MM-DD format (e.g. '2026-10-16').
    pub date: String,
    /// Earliest acceptable arrival time in HH:MM format (e.g. '18:30').
    pub window_start: String,
    /// Latest acceptable arrival time in HH:MM format (e.g. '20:30').
    pub window_end: String,
    /// Seating area preference: 'indoor', 'patio', 'bar', or 'any'. Defaults to 'any'.
    pub area: Option<String>,
    /// Optional contact phone number or email for notifications.
    pub contact: Option<String>,
    /// Optional profile bearer token to link waitlist entry to saved profile.
    pub profile_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JoinWaitlistOutput {
    pub status: String,
    pub code: String,
    pub guest_name: String,
    pub party_size: i64,
    pub date: String,
    pub window_start: String,
    pub window_end: String,
    pub area: String,
    pub queue_position: i64,
    pub expires_at_local: String,
    pub message: String,
}

fn tool_validation_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

pub async fn handle_join_waitlist(pool: &SqlitePool, input: JoinWaitlistInput) -> CallToolResult {
    let clock = SystemClock;
    handle_join_waitlist_with_clock(pool, input, &clock).await
}

pub async fn handle_join_waitlist_with_clock(
    pool: &SqlitePool,
    input: JoinWaitlistInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let guest_name = input.guest_name.trim();
    if guest_name.is_empty() {
        return tool_validation_error("Guest name is required.");
    }
    if input.party_size <= 0 {
        return tool_validation_error("Party size must be at least 1.");
    }

    let parsed_date = match NaiveDate::parse_from_str(input.date.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return tool_validation_error("Invalid date format. Expected YYYY-MM-DD."),
    };

    let start_time = match NaiveTime::parse_from_str(input.window_start.trim(), "%H:%M") {
        Ok(t) => t,
        Err(_) => return tool_validation_error("Invalid window_start format. Expected HH:MM."),
    };

    let end_time = match NaiveTime::parse_from_str(input.window_end.trim(), "%H:%M") {
        Ok(t) => t,
        Err(_) => return tool_validation_error("Invalid window_end format. Expected HH:MM."),
    };

    if start_time >= end_time {
        return tool_validation_error("window_start must be earlier than window_end.");
    }

    let area_str = match input
        .area
        .as_deref()
        .unwrap_or("any")
        .to_lowercase()
        .as_str()
    {
        "indoor" => "indoor",
        "patio" => "patio",
        "bar" => "bar",
        "any" | "" => "any",
        other => {
            return tool_validation_error(format!(
                "Invalid area '{}'. Expected 'indoor', 'patio', 'bar', or 'any'.",
                other
            ))
        }
    };

    // Load restaurant config
    let config = match crate::db::repo::restaurant::get_config(pool).await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("Failed to load config: {}", e)),
    };

    if input.party_size > config.max_party_size {
        return tool_validation_error(format!(
            "Party size cannot exceed restaurant limit of {} guests.",
            config.max_party_size
        ));
    }

    let now_utc = clock.now();
    let today = now_utc.date_naive();
    if parsed_date < today {
        return tool_validation_error("Waitlist requests cannot be made for dates in the past.");
    }
    if (parsed_date - today).num_days() > config.booking_horizon_days {
        return tool_validation_error(format!(
            "Date is beyond the {}-day booking horizon.",
            config.booking_horizon_days
        ));
    }

    // Check operating hours for day of week
    let dow = parsed_date.weekday().num_days_from_sunday() as i64;
    let open_periods: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM opening_hours WHERE day_of_week = ? AND is_closed = 0",
    )
    .bind(dow)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if open_periods == 0 {
        return tool_validation_error("The restaurant is closed on the selected date.");
    }

    // Resolve customer_id if profile_token provided
    let customer_id = if let Some(ref tok) = input.profile_token {
        let rate_limiter = crate::memory::get_token_rate_limiter();
        crate::memory::resolve_profile_token(pool, tok, rate_limiter, clock)
            .await
            .ok()
            .flatten()
            .map(|c| c.id)
    } else {
        None
    };

    let now_iso = now_utc.to_rfc3339();
    let date_str = parsed_date.to_string();
    let start_str = start_time.format("%H:%M").to_string();
    let end_str = end_time.format("%H:%M").to_string();

    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB connection error: {}", e)),
    };

    if let Err(e) = sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to begin transaction: {}", e));
    }

    // 1. Check active entries cap for guest
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM waitlist_entries WHERE guest_name = ? AND status = 'waiting' AND expires_at > ?"
    )
    .bind(guest_name)
    .bind(&now_iso)
    .fetch_one(&mut *conn)
    .await
    .unwrap_or(0);

    if active_count >= MAX_ACTIVE_WAITLIST_PER_GUEST {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        return tool_validation_error("Maximum active waitlist entries reached for this guest. Please wait or leave an existing entry.");
    }

    // 2. Check for duplicate entry (same guest, date, overlapping window) -> idempotent return!
    let existing: Option<(String, i64, String, String, String)> = sqlx::query_as(
        r#"
        SELECT code, party_size, window_start, window_end, area
        FROM waitlist_entries
        WHERE guest_name = ? AND date = ? AND status = 'waiting' AND expires_at > ?
          AND window_start < ? AND window_end > ?
        LIMIT 1
        "#,
    )
    .bind(guest_name)
    .bind(&date_str)
    .bind(&now_iso)
    .bind(&end_str)
    .bind(&start_str)
    .fetch_optional(&mut *conn)
    .await
    .unwrap_or(None);

    if let Some((code, ex_party, ex_start, ex_end, ex_area)) = existing {
        // Calculate position
        let pos: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM waitlist_entries WHERE date = ? AND status = 'waiting' AND created_at < (SELECT created_at FROM waitlist_entries WHERE code = ?)"
        )
        .bind(&date_str)
        .bind(&code)
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(0) + 1;

        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        let spaced = format_spoken_waitlist_code(&code);
        let voice_text = format!("You are already on the waitlist at position {}. Your code is {}. Since we don't send notifications, please check back for updates.", pos, spaced);

        let output = JoinWaitlistOutput {
            status: "waiting".to_string(),
            code,
            guest_name: guest_name.to_string(),
            party_size: ex_party,
            date: date_str,
            window_start: ex_start,
            window_end: ex_end,
            area: ex_area,
            queue_position: pos,
            expires_at_local: "End of service day".to_string(),
            message: "Existing waitlist entry returned (idempotent duplicate request).".to_string(),
        };

        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text(voice_text)];
        return res;
    }

    // 3. Generate waitlist code
    let code_suffix: String = (0..5)
        .map(|_| {
            let charset = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
            let idx = rand::thread_rng().gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect();
    let waitlist_code = format!("WL-{}", code_suffix);

    // Waitlist expires at end of requested date
    let expires_at_dt = parsed_date.and_hms_opt(23, 59, 59).unwrap().and_utc();
    let expires_at_iso = expires_at_dt.to_rfc3339();

    let insert_res = sqlx::query(
        r#"
        INSERT INTO waitlist_entries (
            code, guest_name, contact, party_size, date,
            window_start, window_end, area, duration_minutes,
            status, customer_id, created_at, expires_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'waiting', ?, ?, ?)
        "#,
    )
    .bind(&waitlist_code)
    .bind(guest_name)
    .bind(input.contact.as_deref())
    .bind(input.party_size)
    .bind(&date_str)
    .bind(&start_str)
    .bind(&end_str)
    .bind(area_str)
    .bind(config.dining_duration_minutes)
    .bind(customer_id)
    .bind(&now_iso)
    .bind(&expires_at_iso)
    .execute(&mut *conn)
    .await;

    if let Err(e) = insert_res {
        let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        return tool_validation_error(format!("Failed to insert waitlist entry: {}", e));
    }

    // Calculate queue position
    let pos: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM waitlist_entries WHERE date = ? AND status = 'waiting' AND expires_at > ?"
    )
    .bind(&date_str)
    .bind(&now_iso)
    .fetch_one(&mut *conn)
    .await
    .unwrap_or(1);

    if let Err(e) = sqlx::query("COMMIT").execute(&mut *conn).await {
        return tool_validation_error(format!("Failed to commit waitlist entry: {}", e));
    }

    let spaced = format_spoken_waitlist_code(&waitlist_code);
    let voice_text = format!(
        "You're on the waitlist at position {} for {} between {} and {}. Your code is {}. Since we don't send notifications, please check back for updates.",
        pos,
        parsed_date.format("%A, %B %e"),
        start_time.format("%l:%M %P").to_string().trim(),
        end_time.format("%l:%M %P").to_string().trim(),
        spaced
    );

    let output = JoinWaitlistOutput {
        status: "waiting".to_string(),
        code: waitlist_code,
        guest_name: guest_name.to_string(),
        party_size: input.party_size,
        date: date_str,
        window_start: start_str,
        window_end: end_str,
        area: area_str.to_string(),
        queue_position: pos,
        expires_at_local: "End of service day".to_string(),
        message: "Successfully joined waitlist.".to_string(),
    };

    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(voice_text)];
    res
}
