use crate::{
    availability::{Clock, SystemClock},
    ordering::{
        cancel_order_tx, CancelOrderOutcome, CancelOrderTxRequest, OrderCancellationRateLimiter,
    },
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for `cancel_order` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CancelOrderInput {
    /// Human-friendly order code assigned during order placement (e.g. 'TO-8K4MZ').
    /// Case-insensitive; hyphens and spaces tolerated.
    pub order_code: String,

    /// Full name of the order holder for verification.
    /// Case-insensitive match required to prevent code guessing attacks.
    pub guest_name: String,

    /// Optional reason for cancellation (max 200 characters).
    pub reason: Option<String>,
}

static GLOBAL_ORDER_RATE_LIMITER: std::sync::LazyLock<OrderCancellationRateLimiter> =
    std::sync::LazyLock::new(OrderCancellationRateLimiter::new);

pub async fn handle_cancel_order(pool: &SqlitePool, input: CancelOrderInput) -> CallToolResult {
    let clock = SystemClock;
    handle_cancel_order_with_deps(pool, input, &GLOBAL_ORDER_RATE_LIMITER, &clock).await
}

pub async fn handle_cancel_order_with_deps(
    pool: &SqlitePool,
    input: CancelOrderInput,
    rate_limiter: &OrderCancellationRateLimiter,
    clock: &dyn Clock,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    let order_code_clean =
        match crate::validation::sanitize_string(&input.order_code, "order_code", 1, 64) {
            Ok(c) => c,
            Err(err) => return err,
        };

    let guest_name_clean =
        match crate::validation::sanitize_string(&input.guest_name, "guest_name", 1, 100) {
            Ok(n) => n,
            Err(err) => return err,
        };

    let sanitized_reason =
        match crate::validation::sanitize_optional_string(input.reason, "reason", 200) {
            Ok(r) => r,
            Err(err) => return err,
        };

    let tx_req = CancelOrderTxRequest {
        order_code: &order_code_clean,
        guest_name: &guest_name_clean,
        reason: sanitized_reason.as_deref(),
    };

    match cancel_order_tx(pool, tx_req, rate_limiter, clock).await {
        Ok(outcome) => {
            let (output, voice_text) = match outcome {
                CancelOrderOutcome::Cancelled { output, voice_text } => {
                    let code_prefix = output
                        .order
                        .as_ref()
                        .map(|o| &o.order_code[..o.order_code.len().min(6)])
                        .unwrap_or("TO-?");
                    tracing::info!(
                        request_id = %req_id,
                        order_prefix = %code_prefix,
                        status = "cancelled",
                        "Order cancelled successfully"
                    );
                    (output, voice_text)
                }
                CancelOrderOutcome::AlreadyCancelled { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "already_cancelled", "Order already cancelled");
                    (output, voice_text)
                }
                CancelOrderOutcome::NotCancellable { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "not_cancellable", "Order already in kitchen prep");
                    (output, voice_text)
                }
                CancelOrderOutcome::NotFound { output, voice_text } => {
                    tracing::warn!(request_id = %req_id, status = "not_found", "Order cancellation verification failed");
                    (output, voice_text)
                }
                CancelOrderOutcome::RateLimited { output, voice_text } => {
                    tracing::warn!(request_id = %req_id, status = "limit_exceeded", "Cancellation rate limit reached");
                    (output, voice_text)
                }
            };

            let structured_val = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(err) => {
                    tracing::error!("Failed to serialize CancelOrderOutput: {}", err);
                    return CallToolResult::error(vec![ContentBlock::text(
                        "An internal serialization error occurred while formatting cancel order result.",
                    )]);
                }
            };

            let mut tool_result = CallToolResult::structured(structured_val);
            tool_result.content = vec![ContentBlock::text(voice_text)];
            tool_result
        }
        Err(e) => {
            tracing::error!(request_id = %req_id, error = %e, "Database error during order cancellation");
            crate::validation::internal_tool_error("cancelling order", e)
        }
    }
}
