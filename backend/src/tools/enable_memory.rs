use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{enable_memory_tx, generate_enable_memory_voice_text, InitialPreferenceInput},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EnableMemoryInput {
    /// Full name of the dining guest (1-80 characters).
    pub guest_name: String,
    /// Optional mobile telephone number for SMS booking updates.
    pub phone: Option<String>,
    /// Optional email address for confirmation notifications.
    pub email: Option<String>,
    /// Required explicit consent acknowledgement. Must be true to enable profile memory.
    pub consent_acknowledged: bool,
    /// Optional initial dining preferences (allergens, dietary tags, seating, favorites).
    pub initial_preferences: Option<Vec<InitialPreferenceInput>>,
}

pub async fn handle_enable_memory(
    pool: &SqlitePool,
    input: EnableMemoryInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_enable_memory_with_clock(pool, input, &clock).await
}

pub async fn handle_enable_memory_with_clock(
    pool: &SqlitePool,
    input: EnableMemoryInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let name_trimmed = input.guest_name.trim();
    if name_trimmed.is_empty() || name_trimmed.len() > 80 {
        return rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            "Invalid input: 'guest_name' must be between 1 and 80 characters.",
        )]);
    }

    if !input.consent_acknowledged {
        return rmcp::model::CallToolResult::error(vec![
            rmcp::model::ContentBlock::text(
                "Consent required: Memory cannot be enabled without explicit guest consent ('consent_acknowledged: true'). Please ask the guest if they wish to opt into personalized profile memory.",
            ),
        ]);
    }

    let initial = input.initial_preferences.unwrap_or_default();

    match enable_memory_tx(
        pool,
        name_trimmed,
        input.phone.as_deref(),
        input.email.as_deref(),
        &initial,
        clock,
    )
    .await
    {
        Ok(output) => {
            let voice_text = generate_enable_memory_voice_text(&output.guest_name);
            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(format!(
                            "Failed to serialize enable_memory output: {}",
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
            format!("Database error enabling memory: {}", e),
        )]),
    }
}
