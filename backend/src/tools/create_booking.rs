use crate::{
    availability::{AvailabilityConfig, Clock, SystemClock},
    booking::{
        create_booking_tx, validate_create_booking_request, CreateBookingOutcome,
        CreateBookingTxRequest,
    },
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Seating area preference for reservation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BookingAreaInput {
    #[default]
    Any,
    Indoor,
    Patio,
    Bar,
}

impl BookingAreaInput {
    pub fn as_str(&self) -> &'static str {
        match self {
            BookingAreaInput::Any => "any",
            BookingAreaInput::Indoor => "indoor",
            BookingAreaInput::Patio => "patio",
            BookingAreaInput::Bar => "bar",
        }
    }
}

/// Input parameters for the `create_booking` MCP tool.
/// Descriptions are written for LLM agent callers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateBookingInput {
    /// Local restaurant date in ISO format YYYY-MM-DD (e.g. '2026-10-17'). \
    /// The caller must resolve relative phrases like 'tomorrow' or 'Friday' to a concrete date itself, \
    /// using the restaurant timezone (Asia/Kolkata).
    pub date: String,

    /// Local 24-hour reservation time in HH:MM format (e.g. '19:30').
    pub time: String,

    /// Party size (number of guests). Must be at least 1 and cannot exceed restaurant maximum capacity.
    pub party_size: i64,

    /// Name of the primary dining guest (1 to 80 characters, trimmed).
    pub guest_name: String,

    /// Optional contact phone number for reservation notifications and SMS reminders.
    pub phone: Option<String>,

    /// Optional contact email address for booking confirmation receipts.
    pub email: Option<String>,

    /// Seating area preference: 'indoor', 'patio', 'bar', or 'any'. Defaults to 'any'.
    pub area: Option<BookingAreaInput>,

    /// Optional dining duration in minutes (30-240). If omitted, defaults by party size \
    /// (75m for 1-2 guests, 90m for 3-4 guests, 105m for 5-6 guests, 120m for 7+ guests).
    pub duration_minutes: Option<i64>,

    /// Optional dietary notes, seating requests, or celebration comments (max 300 characters).
    pub special_requests: Option<String>,

    /// Optional idempotency key (8-64 characters matching [a-zA-Z0-9_.-]). \
    /// Strongly recommended on client retries to guarantee at-most-once booking creation.
    pub idempotency_key: Option<String>,

    /// Optional returning customer identifier if known from session history.
    pub customer_id: Option<i64>,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Handler for `create_booking` tool.
pub async fn handle_create_booking(pool: &SqlitePool, input: CreateBookingInput) -> CallToolResult {
    let clock = SystemClock;
    handle_create_booking_with_clock(pool, input, &clock).await
}

pub async fn handle_create_booking_with_clock(
    pool: &SqlitePool,
    input: CreateBookingInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    // 1. Validation check for tool execution errors
    if let Err(val_err) = validate_create_booking_request(
        &input.date,
        &input.time,
        input.party_size,
        &input.guest_name,
        input.phone.as_deref(),
        input.email.as_deref(),
        input.special_requests.as_deref(),
        input.idempotency_key.as_deref(),
        12, // Max restaurant party size
    ) {
        return tool_validation_error(val_err.to_string());
    }

    let area_str = input.area.map(|a| a.as_str());

    let tx_req = CreateBookingTxRequest {
        date: &input.date,
        time: &input.time,
        party_size: input.party_size,
        guest_name: &input.guest_name,
        phone: input.phone.as_deref(),
        email: input.email.as_deref(),
        area: area_str,
        duration_minutes: input.duration_minutes,
        special_requests: input.special_requests.as_deref(),
        idempotency_key: input.idempotency_key.as_deref(),
        customer_id: input.customer_id,
    };

    let engine_config = AvailabilityConfig::default();

    // 2. Execute transactional booking creation
    let outcome = match create_booking_tx(pool, tx_req, &engine_config, clock).await {
        Ok(out) => out,
        Err(err) => {
            tracing::error!(request_id = %req_id, "Failed to create booking in database transaction: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while creating your reservation. Please try again.",
            )]);
        }
    };

    // 3. Format outcome into CallToolResult
    let (output, voice_text) = match outcome {
        CreateBookingOutcome::Created { output, voice_text } => {
            if let Some(ref b) = output.booking {
                let code_prefix = &b.confirmation_code[..b.confirmation_code.len().min(6)];
                tracing::info!(
                    request_id = %req_id,
                    code_prefix = %code_prefix,
                    party_size = b.party_size,
                    "Reservation created successfully"
                );
            }
            (output, voice_text)
        }
        CreateBookingOutcome::IdempotentReplay { output, voice_text } => {
            tracing::info!(
                request_id = %req_id,
                "Idempotent reservation replay returned"
            );
            (output, voice_text)
        }
        CreateBookingOutcome::Unavailable { output, voice_text } => (output, voice_text),
        CreateBookingOutcome::LimitExceeded { output, voice_text } => (output, voice_text),
        CreateBookingOutcome::IdempotencyConflict { output, voice_text } => (output, voice_text),
    };

    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize CreateBookingOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while formatting booking result.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
