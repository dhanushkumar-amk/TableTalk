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

pub async fn handle_get_order(pool: &SqlitePool, input: GetOrderInput) -> CallToolResult {
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

    match get_order_tx(pool, &order_code_clean, &guest_name_clean).await {
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
            crate::validation::internal_tool_error("retrieving order", e)
        }
    }
}
