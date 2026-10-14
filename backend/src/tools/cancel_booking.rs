use crate::{
    availability::{Clock, SystemClock},
    booking::{
        cancel_booking_tx, CancelBookingOutcome, CancelBookingTxRequest, CancellationRateLimiter,
    },
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::LazyLock;

static GLOBAL_RATE_LIMITER: LazyLock<CancellationRateLimiter> =
    LazyLock::new(CancellationRateLimiter::new);

/// Input parameters for the `cancel_booking` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CancelBookingInput {
    /// The unique confirmation code received when booking was created (e.g. 'TT-8K4MZ'). \
    /// Case-insensitive; hyphens and spaces may be omitted.
    pub confirmation_code: String,

    /// Guest name associated with the reservation. Required for identity verification \
    /// to ensure reservations cannot be cancelled by unauthorized callers.
    pub guest_name: String,

    /// Optional reason for cancellation (e.g. 'change of travel plans', max 200 characters).
    pub reason: Option<String>,
}

/// Handler for `cancel_booking` tool.
pub async fn handle_cancel_booking(pool: &SqlitePool, input: CancelBookingInput) -> CallToolResult {
    let clock = SystemClock;
    handle_cancel_booking_with_clock(pool, input, &clock, &GLOBAL_RATE_LIMITER).await
}

pub async fn handle_cancel_booking_with_clock(
    pool: &SqlitePool,
    input: CancelBookingInput,
    clock: &dyn Clock,
    rate_limiter: &CancellationRateLimiter,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    // Input validation
    let trimmed_code = match crate::validation::sanitize_string(
        &input.confirmation_code,
        "confirmation_code",
        1,
        64,
    ) {
        Ok(c) => c,
        Err(err) => return err,
    };

    let trimmed_name =
        match crate::validation::sanitize_string(&input.guest_name, "guest_name", 1, 100) {
            Ok(n) => n,
            Err(err) => return err,
        };

    let sanitized_reason =
        match crate::validation::sanitize_optional_string(input.reason, "reason", 200) {
            Ok(r) => r,
            Err(err) => return err,
        };

    let tx_req = CancelBookingTxRequest {
        confirmation_code: &trimmed_code,
        guest_name: &trimmed_name,
        reason: sanitized_reason.as_deref(),
    };

    let outcome = match cancel_booking_tx(pool, tx_req, rate_limiter, clock).await {
        Ok(out) => out,
        Err(err) => {
            return crate::validation::internal_tool_error("cancelling booking", err);
        }
    };

    let (output, voice_text) = match outcome {
        CancelBookingOutcome::Cancelled { output, voice_text } => {
            tracing::info!(
                request_id = %req_id,
                code = %trimmed_code,
                "Reservation successfully cancelled"
            );
            (output, voice_text)
        }
        CancelBookingOutcome::AlreadyCancelled { output, voice_text } => (output, voice_text),
        CancelBookingOutcome::NotFound { output, voice_text } => (output, voice_text),
        CancelBookingOutcome::NotCancellable { output, voice_text } => (output, voice_text),
        CancelBookingOutcome::LimitExceeded { output, voice_text } => (output, voice_text),
    };

    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize CancelBookingOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while formatting cancellation result.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
