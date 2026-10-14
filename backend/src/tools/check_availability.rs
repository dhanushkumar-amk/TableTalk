use crate::availability::{
    check_availability_engine, generate_availability_voice_text, AreaFilter, AvailabilityConfig,
    AvailabilityQuery, Clock, SystemClock,
};
use crate::db::repo::{bookings, restaurant, tables};
use chrono::{Duration, TimeZone, Utc};
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

pub use crate::availability::{
    AlternativeSlot, AvailableSlot, OpeningHoursRange, RequestedDetails,
};

/// Input parameters for the `check_availability` MCP tool.
/// Descriptions are tailored for LLM agent callers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CheckAvailabilityInput {
    /// Local restaurant date in ISO format YYYY-MM-DD (e.g. '2026-10-17'). \
    /// The caller must resolve relative phrases like 'tomorrow' or 'Friday' to a concrete date itself, \
    /// using the restaurant timezone. The tool result includes the restaurant's current local date/time \
    /// so the caller can verify.
    pub date: String,

    /// Local 24-hour time in HH:MM format (e.g. '19:30' or '13:00'). \
    /// Off-grid times will be reported and snapped to the nearest slot granularity (15 minutes).
    pub time: String,

    /// Party size (number of guests). Must be at least 1 and cannot exceed restaurant maximum capacity.
    pub party_size: i64,

    /// Seating area preference: 'indoor', 'patio', 'bar', or 'any'. Defaults to 'any'.
    pub area: Option<AreaFilter>,

    /// Estimated dining duration in minutes. Optional; if omitted, defaults based on party size \
    /// (75m for 1-2 guests, 90m for 3-4 guests, 105m for 5-6 guests, 120m for 7+ guests). \
    /// Must be between 30 and 240 minutes if specified.
    pub duration_minutes: Option<i64>,

    /// Number of alternative slots to return if the requested time is unavailable (0-6, default 3).
    pub alternatives: Option<usize>,
}

/// Handler for `check_availability` MCP tool execution.
pub async fn handle_check_availability(
    pool: &SqlitePool,
    input: CheckAvailabilityInput,
) -> CallToolResult {
    handle_check_availability_with_clock(pool, input, &SystemClock).await
}

/// Handler for `check_availability` MCP tool execution with an injectable Clock for deterministic tests.
pub async fn handle_check_availability_with_clock(
    pool: &SqlitePool,
    input: CheckAvailabilityInput,
    clock: &dyn Clock,
) -> CallToolResult {
    // 1. Argument validation (schema format & boundaries)
    let parsed_date = match crate::validation::parse_iso_date(&input.date, "date") {
        Ok(d) => d,
        Err(err) => return err,
    };

    let parsed_time = match crate::validation::parse_time_24h(&input.time, "time") {
        Ok(t) => t,
        Err(err) => return err,
    };

    if let Err(err) = crate::validation::validate_range_i64(input.party_size, 1, 100, "party_size")
    {
        return err;
    }

    if let Some(dur) = input.duration_minutes {
        if let Err(err) = crate::validation::validate_range_i64(dur, 30, 240, "duration_minutes") {
            return err;
        }
    }

    let alternatives_limit = match input.alternatives {
        Some(a) => {
            if let Err(err) = crate::validation::validate_range_i64(a as i64, 0, 6, "alternatives")
            {
                return err;
            }
            a
        }
        None => 3,
    };

    let area = input.area.unwrap_or(AreaFilter::Any);

    // 2. Fetch restaurant configuration, hours, closures, and tables from DB (read-only)
    let config = match restaurant::get_config(pool).await {
        Ok(c) => c,
        Err(err) => {
            let req_id = uuid::Uuid::new_v4();
            tracing::error!(request_id = %req_id, "Failed to load restaurant config: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while checking availability. Please try again.",
            )]);
        }
    };

    let opening_hours = match restaurant::get_opening_hours(pool).await {
        Ok(h) => h,
        Err(err) => {
            let req_id = uuid::Uuid::new_v4();
            tracing::error!(request_id = %req_id, "Failed to load opening hours: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while checking opening hours. Please try again.",
            )]);
        }
    };

    let special_closures = match restaurant::get_special_closures(pool).await {
        Ok(c) => c,
        Err(err) => {
            let req_id = uuid::Uuid::new_v4();
            tracing::error!(request_id = %req_id, "Failed to load special closures: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while checking special closures. Please try again.",
            )]);
        }
    };

    let active_tables = match tables::list_active_tables(pool).await {
        Ok(t) => t,
        Err(err) => {
            let req_id = uuid::Uuid::new_v4();
            tracing::error!(request_id = %req_id, "Failed to load dining tables: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while checking dining tables. Please try again.",
            )]);
        }
    };

    // 3. Single-query load of relevant active bookings for the search date range
    // We cover from the beginning of the requested date to +7 days to allow next-days alternatives
    let tz: Tz = match config.timezone.parse() {
        Ok(t) => t,
        Err(_) => {
            return CallToolResult::error(vec![ContentBlock::text(
                "Restaurant timezone configuration is invalid.",
            )]);
        }
    };

    let search_start_utc = match tz.from_local_datetime(&parsed_date.and_hms_opt(0, 0, 0).unwrap())
    {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(e, _) => e.with_timezone(&Utc),
        chrono::LocalResult::None => clock.now() - Duration::days(1),
    };
    let search_end_utc = search_start_utc + Duration::days(14);

    let active_bookings = match bookings::list_active_bookings_in_range(
        pool,
        &search_start_utc.to_rfc3339(),
        &search_end_utc.to_rfc3339(),
    )
    .await
    {
        Ok(b) => b,
        Err(err) => {
            let req_id = uuid::Uuid::new_v4();
            tracing::error!(request_id = %req_id, "Failed to load active bookings in range: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while checking existing reservations. Please try again.",
            )]);
        }
    };

    let now_iso = clock.now().to_rfc3339();
    let active_holds: Vec<(i64, i64, String, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT id, table_id, start_at, end_at, guest_name, party_size
        FROM slot_holds
        WHERE status = 'active'
          AND expires_at > ?
          AND start_at < ?
          AND end_at > ?
        "#,
    )
    .bind(&now_iso)
    .bind(search_end_utc.to_rfc3339())
    .bind(search_start_utc.to_rfc3339())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut combined_bookings = active_bookings.clone();
    for (hold_id, table_id, start_at, end_at, guest_name, party_size) in &active_holds {
        combined_bookings.push(crate::db::models::Booking {
            id: format!("hold_{}", hold_id),
            confirmation_code: format!("HOLD-{}", hold_id),
            customer_id: None,
            guest_name: guest_name.clone(),
            party_size: *party_size,
            start_at: start_at.clone(),
            end_at: end_at.clone(),
            status: crate::db::models::BookingStatus::Pending,
            table_id: Some(*table_id),
            special_requests: None,
            deposit_status: crate::db::models::DepositStatus::NotRequired,
            deposit_amount_minor: 0,
            payment_deadline_at: None,
            cancel_reason: None,
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            cancelled_at: None,
            guest_phone: None,
            guest_email: None,
        });
    }

    // 4. Execute pure availability engine
    let query = AvailabilityQuery {
        date: parsed_date,
        time: parsed_time,
        party_size: input.party_size,
        area,
        duration_minutes: input.duration_minutes,
        alternatives_limit,
    };

    let engine_config = AvailabilityConfig::default();

    let mut output = check_availability_engine(
        query.clone(),
        &config,
        &opening_hours,
        &special_closures,
        &active_tables,
        &combined_bookings,
        &engine_config,
        clock,
    );

    if !active_holds.is_empty() && output.status != "available" {
        let output_without_holds = check_availability_engine(
            query,
            &config,
            &opening_hours,
            &special_closures,
            &active_tables,
            &active_bookings,
            &engine_config,
            clock,
        );
        if output_without_holds.status == "available" {
            output.reason_detail = Some("temporarily held, may free up soon".to_string());
        }
    }

    // 5. Generate spoken voice text
    let voice_text = generate_availability_voice_text(&output);

    // 6. Serialize structured content
    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize CheckAvailabilityOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while formatting availability results.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
