use crate::ordering::{get_order_tx, GetOrderOutcome};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for `get_order` MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetOrderInput {
    /// Human-friendly order code assigned during order placement (e.g. 'TO-8K4MZ').
    /// Case-insensitive; hyphens and spaces tolerated.
    pub order_code: String,

    /// Full name of the order holder for verification.
    /// Case-insensitive match required for privacy.
    pub guest_name: String,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

pub async fn handle_get_order(pool: &SqlitePool, input: GetOrderInput) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    if input.order_code.trim().is_empty() {
        return tool_validation_error("Order code is required.");
    }
    if input.guest_name.trim().is_empty() {
        return tool_validation_error("Guest name is required for order lookup verification.");
    }

    match get_order_tx(pool, &input.order_code, &input.guest_name).await {
        Ok(outcome) => {
            let (output, voice_text) = match outcome {
                GetOrderOutcome::Found { output, voice_text } => {
                    let code_prefix = output
                        .order
                        .as_ref()
                        .map(|o| &o.order_code[..o.order_code.len().min(6)])
                        .unwrap_or("TO-?");
                    tracing::info!(
                        request_id = %req_id,
                        order_prefix = %code_prefix,
                        status = "found",
                        "Order summary retrieved"
                    );
                    (output, voice_text)
                }
                GetOrderOutcome::NotFound { output, voice_text } => {
                    tracing::warn!(request_id = %req_id, status = "not_found", "Order lookup verification failed");
                    (output, voice_text)
                }
            };

            let structured_val = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(err) => {
                    tracing::error!("Failed to serialize GetOrderOutput: {}", err);
                    return CallToolResult::error(vec![ContentBlock::text(
                        "An internal serialization error occurred while formatting get order result.",
                    )]);
                }
            };

            let mut tool_result = CallToolResult::structured(structured_val);
            tool_result.content = vec![ContentBlock::text(voice_text)];
            tool_result
        }
        Err(e) => {
            tracing::error!(request_id = %req_id, error = %e, "Database error retrieving order");
            tool_validation_error(format!("Internal order lookup error: {}", e))
        }
    }
}
