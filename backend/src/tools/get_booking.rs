use crate::availability::{Clock, SystemClock};
use crate::booking::{get_booking_tx, GetBookingOutcome};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for the `get_booking` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetBookingInput {
    /// The unique confirmation code of the reservation (e.g. 'TT-8K4MZ').
    pub confirmation_code: String,

    /// Guest name associated with the reservation. Required for identity verification.
    pub guest_name: String,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Handler for `get_booking` tool.
pub async fn handle_get_booking(pool: &SqlitePool, input: GetBookingInput) -> CallToolResult {
    handle_get_booking_with_clock(pool, input, &SystemClock).await
}

/// Handler for `get_booking` tool with an injectable clock.
pub async fn handle_get_booking_with_clock(
    pool: &SqlitePool,
    input: GetBookingInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    let trimmed_code = input.confirmation_code.trim();
    if trimmed_code.is_empty() {
        return tool_validation_error(
            "Confirmation code is required and cannot be empty (e.g. 'TT-8K4MZ').",
        );
    }

    let trimmed_name = input.guest_name.trim();
    if trimmed_name.is_empty() {
        return tool_validation_error("Guest name is required for booking verification.");
    }

    let outcome = match get_booking_tx(pool, trimmed_code, trimmed_name, clock).await {
        Ok(out) => out,
        Err(err) => {
            tracing::error!(request_id = %req_id, "Failed to retrieve booking from database: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal database error occurred while looking up your reservation. Please try again.",
            )]);
        }
    };

    let (output, voice_text) = match outcome {
        GetBookingOutcome::Found { output, voice_text } => (output, voice_text),
        GetBookingOutcome::NotFound { output, voice_text } => (output, voice_text),
    };

    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize GetBookingOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while formatting booking result.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
