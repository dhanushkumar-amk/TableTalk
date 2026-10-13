use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{forget_me_tx, generate_forget_me_voice_text, get_token_rate_limiter},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ForgetMeInput {
    /// Opaque profile bearer token (e.g. 'tt_prof_...') to delete.
    pub profile_token: String,
    /// Explicit confirmation to delete profile and all associated preferences.
    pub confirm: bool,
}

pub async fn handle_forget_me(
    pool: &SqlitePool,
    input: ForgetMeInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_forget_me_with_clock(pool, input, &clock).await
}

pub async fn handle_forget_me_with_clock(
    pool: &SqlitePool,
    input: ForgetMeInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let token_clean = input.profile_token.trim();
    if token_clean.is_empty() {
        return rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            "Invalid input: 'profile_token' must be provided.",
        )]);
    }

    if !input.confirm {
        return rmcp::model::CallToolResult::error(vec![
            rmcp::model::ContentBlock::text(
                "Confirmation required: 'confirm: true' is required to permanently hard-delete profile data.",
            ),
        ]);
    }

    let rate_limiter = get_token_rate_limiter();
    match forget_me_tx(pool, token_clean, rate_limiter, clock).await {
        Ok(output) => {
            let voice_text = if output.status == "not_found" {
                "I couldn't find an active profile associated with that token.".to_string()
            } else {
                generate_forget_me_voice_text()
            };

            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(format!(
                            "Failed to serialize forget_me output: {}",
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
            format!("Database error processing forget_me: {}", e),
        )]),
    }
}
