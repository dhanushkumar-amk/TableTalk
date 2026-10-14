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
    let name_trimmed =
        match crate::validation::sanitize_string(&input.guest_name, "guest_name", 1, 80) {
            Ok(n) => n,
            Err(err) => return err,
        };

    let sanitized_phone =
        match crate::validation::sanitize_optional_string(input.phone, "phone", 30) {
            Ok(p) => p,
            Err(err) => return err,
        };

    let sanitized_email =
        match crate::validation::sanitize_optional_string(input.email, "email", 100) {
            Ok(e) => e,
            Err(err) => return err,
        };

    if !input.consent_acknowledged {
        return crate::validation::tool_arg_error(
            "consent_acknowledged",
            "Memory cannot be enabled without explicit guest consent ('consent_acknowledged: true'). Please ask the guest if they wish to opt into personalized profile memory.",
        );
    }

    let initial = input.initial_preferences.unwrap_or_default();
    if let Err(err) =
        crate::validation::validate_array_bounds(&initial, 0, 20, "initial_preferences")
    {
        return err;
    }

    match enable_memory_tx(
        pool,
        &name_trimmed,
        sanitized_phone.as_deref(),
        sanitized_email.as_deref(),
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
                    return crate::validation::internal_tool_error(
                        "serializing enable_memory output",
                        e,
                    );
                }
            };

            let mut tool_result = rmcp::model::CallToolResult::structured(structured);
            tool_result.content = vec![rmcp::model::ContentBlock::text(voice_text)];
            tool_result
        }
        Err(e) => crate::validation::internal_tool_error("enabling memory", e),
    }
}
