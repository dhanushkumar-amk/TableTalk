use crate::availability::{
    check_availability_engine, generate_availability_voice_text, AreaFilter, AvailabilityConfig,
    AvailabilityQuery, Clock, SystemClock,
};
use crate::db::repo::{bookings, restaurant, tables};
use chrono::{Duration, NaiveDate, NaiveTime, TimeZone, Utc};
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

/// Helper to produce a tool execution error (isError: true) allowing the LLM to self-correct.
fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
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
    let parsed_date = match NaiveDate::parse_from_str(&input.date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return tool_validation_error(format!(
                "Invalid input: 'date' must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received '{}'.",
                input.date
            ));
        }
    };

    let parsed_time = match NaiveTime::parse_from_str(&input.time, "%H:%M") {
        Ok(t) => t,
        Err(_) => {
            return tool_validation_error(format!(
                "Invalid input: 'time' must be in 24-hour HH:MM format (e.g. '19:30' or '13:00'). Received '{}'.",
                input.time
            ));
        }
    };

    if input.party_size < 1 {
        return tool_validation_error(format!(
            "Invalid input: 'party_size' must be at least 1 (received {}).",
            input.party_size
        ));
    }

    if let Some(dur) = input.duration_minutes {
        if !(30..=240).contains(&dur) {
            return tool_validation_error(format!(
                "Invalid input: 'duration_minutes' must be between 30 and 240 minutes (received {}).",
                dur
            ));
        }
    }

    let alternatives_limit = match input.alternatives {
        Some(a) if a > 6 => {
            return tool_validation_error(format!(
                "Invalid input: 'alternatives' must be between 0 and 6 (received {}).",
                a
            ));
        }
        Some(a) => a,
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

    let output = check_availability_engine(
        query,
        &config,
        &opening_hours,
        &special_closures,
        &active_tables,
        &active_bookings,
        &engine_config,
        clock,
    );

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
