use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{generate_usual_order_voice_text, get_token_rate_limiter, get_usual_order_tx},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GetUsualOrderInput {
    /// Opaque profile bearer token (e.g. 'tt_prof_...') issued during enable_memory.
    pub profile_token: String,
}

pub async fn handle_get_usual_order(
    pool: &SqlitePool,
    input: GetUsualOrderInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_get_usual_order_with_clock(pool, input, &clock).await
}

pub async fn handle_get_usual_order_with_clock(
    pool: &SqlitePool,
    input: GetUsualOrderInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let token_clean = input.profile_token.trim();
    if token_clean.is_empty() {
        return rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            "Invalid input: 'profile_token' must be provided.",
        )]);
    }

    let rate_limiter = get_token_rate_limiter();
    match get_usual_order_tx(pool, token_clean, rate_limiter, clock).await {
        Ok(output) => {
            let voice_text = if output.status == "not_found" {
                "I couldn't find a saved profile with that token. Please check your token or ask to enable memory.".to_string()
            } else {
                let first_name = output
                    .guest_name
                    .as_deref()
                    .and_then(|n| n.split_whitespace().next())
                    .unwrap_or("there");
                generate_usual_order_voice_text(&output, first_name)
            };

            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(format!(
                            "Failed to serialize get_usual_order output: {}",
                            e
                        )),
                    ]);
                }
            };

            let mut tool_result = rmcp::model::CallToolResult::structured(structured);
            tool_result.content = vec![rmcp::model::ContentBlock::text(voice_text)];
            tool_result
        }
        Err(e) => rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            format!("Database error retrieving usual order: {}", e),
        )]),
    }
}
