use chrono::{NaiveDate, NaiveTime};
use rmcp::model::{CallToolResult, ContentBlock};

/// Helper to produce a standardized tool execution error (`isError: true`) for an invalid argument.
/// Formats the message consistently naming the field and the expected constraints or format,
/// allowing the LLM caller to self-correct.
pub fn tool_arg_error(field: &str, message: impl std::fmt::Display) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!(
        "Invalid argument for '{}': {}",
        field, message
    ))])
}

/// Helper to produce a tool execution error with a custom message (`isError: true`).
pub fn tool_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Helper to handle internal server and database errors safely without leaking SQL queries,
/// table structures, file paths, or stack traces.
///
/// Logs the full error detail to `tracing::error!` with a generated Request ID,
/// and returns a clean, safe tool error containing only the operation context and Request ID.
pub fn internal_tool_error(context: &str, err: impl std::fmt::Display) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();
    tracing::error!(request_id = %req_id, "Internal error in {}: {}", context, err);
    CallToolResult::error(vec![ContentBlock::text(format!(
        "An internal error occurred while processing {}. Please try again (Request ID: {}).",
        context, req_id
    ))])
}

/// Sanitizes a string:
/// 1. Strips dangerous ASCII control characters (preserving standard tab `\t` and newline `\n`).
/// 2. Trims leading and trailing whitespace.
/// 3. Validates Unicode character length against `[min_chars, max_chars]`.
pub fn sanitize_string(
    s: &str,
    field: &str,
    min_chars: usize,
    max_chars: usize,
) -> Result<String, CallToolResult> {
    let cleaned: String = s
        .chars()
        .filter(|c| !c.is_control() || *c == '\t' || *c == '\n')
        .collect();
    let trimmed = cleaned.trim();
    let count = trimmed.chars().count();

    if count < min_chars {
        if min_chars == 1 {
            return Err(tool_arg_error(field, "cannot be empty."));
        } else {
            return Err(tool_arg_error(
                field,
                format!(
                    "must be at least {} characters (received {}).",
                    min_chars, count
                ),
            ));
        }
    }

    if count > max_chars {
        return Err(tool_arg_error(
            field,
            format!(
                "exceeds maximum allowed length of {} characters (received {}).",
                max_chars, count
            ),
        ));
    }

    Ok(trimmed.to_string())
}

/// Sanitizes an optional string: if `Some`, strips control characters, trims, and validates max length.
/// If empty after trimming, returns `None`.
pub fn sanitize_optional_string(
    s: Option<String>,
    field: &str,
    max_chars: usize,
) -> Result<Option<String>, CallToolResult> {
    match s {
        None => Ok(None),
        Some(val) => {
            let cleaned = sanitize_string(&val, field, 0, max_chars)?;
            if cleaned.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cleaned))
            }
        }
    }
}

/// Parses an ISO 8601 date string strictly formatted as `YYYY-MM-DD`.
pub fn parse_iso_date(date_str: &str, field: &str) -> Result<NaiveDate, CallToolResult> {
    let cleaned = sanitize_string(date_str, field, 1, 30)?;
    match NaiveDate::parse_from_str(&cleaned, "%Y-%m-%d") {
        Ok(d) => Ok(d),
        Err(_) => Err(tool_arg_error(
            field,
            format!(
                "must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received '{}'.",
                cleaned
            ),
        )),
    }
}

/// Parses a 24-hour time string strictly formatted as `HH:MM`.
pub fn parse_time_24h(time_str: &str, field: &str) -> Result<NaiveTime, CallToolResult> {
    let cleaned = sanitize_string(time_str, field, 1, 30)?;
    match NaiveTime::parse_from_str(&cleaned, "%H:%M") {
        Ok(t) => Ok(t),
        Err(_) => Err(tool_arg_error(
            field,
            format!(
                "must be in 24-hour format HH:MM (e.g. '19:30'). Received '{}'.",
                cleaned
            ),
        )),
    }
}

/// Validates that an `i64` integer lies within `[min, max]`.
pub fn validate_range_i64(
    val: i64,
    min: i64,
    max: i64,
    field: &str,
) -> Result<i64, CallToolResult> {
    if val < min || val > max {
        return Err(tool_arg_error(
            field,
            format!("must be between {} and {} (received {}).", min, max, val),
        ));
    }
    Ok(val)
}

/// Validates that an `f64` float is non-negative and finite within `[min, max]`.
pub fn validate_range_f64(
    val: f64,
    min: f64,
    max: f64,
    field: &str,
) -> Result<f64, CallToolResult> {
    if !val.is_finite() || val < min || val > max {
        return Err(tool_arg_error(
            field,
            format!(
                "must be a valid number between {} and {} (received {}).",
                min, max, val
            ),
        ));
    }
    Ok(val)
}

/// Validates that an array/slice contains between `[min, max]` items.
pub fn validate_array_bounds<T>(
    arr: &[T],
    min: usize,
    max: usize,
    field: &str,
) -> Result<(), CallToolResult> {
    let count = arr.len();
    if count < min {
        return Err(tool_arg_error(
            field,
            format!(
                "must contain at least {} item(s) (received {}).",
                min, count
            ),
        ));
    }
    if count > max {
        return Err(tool_arg_error(
            field,
            format!("cannot exceed {} item(s) (received {}).", max, count),
        ));
    }
    Ok(())
}

/// Normalizes a booking or order alphanumeric code: strips spaces and hyphens, converts to uppercase.
pub fn normalize_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}
