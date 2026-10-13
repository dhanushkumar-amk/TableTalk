use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{get_token_rate_limiter, rotate_profile_token_tx},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RotateProfileTokenInput {
    /// Opaque profile bearer token (e.g. 'tt_prof_...') to rotate and revoke.
    pub profile_token: String,
}

pub async fn handle_rotate_profile_token(
    pool: &SqlitePool,
    input: RotateProfileTokenInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_rotate_profile_token_with_clock(pool, input, &clock).await
}

pub async fn handle_rotate_profile_token_with_clock(
    pool: &SqlitePool,
    input: RotateProfileTokenInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let token_clean = input.profile_token.trim();
    if token_clean.is_empty() {
        return rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            "Invalid input: 'profile_token' must be provided.",
        )]);
    }

    let rate_limiter = get_token_rate_limiter();
    match rotate_profile_token_tx(pool, token_clean, rate_limiter, clock).await {
        Ok(output) => {
            let voice_text = if output.status == "not_found" {
                "I couldn't find a saved profile with that token. Please check your token or ask to enable memory.".to_string()
            } else {
                "I have rotated your profile token. Please make sure to save your new token securely, as your previous token is now invalid.".to_string()
            };

            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(format!(
                            "Failed to serialize rotate_profile_token output: {}",
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
            format!("Database error rotating profile token: {}", e),
        )]),
    }
}
