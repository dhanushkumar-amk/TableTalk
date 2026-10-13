use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{get_token_rate_limiter, remember_preference_tx},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RememberPreferenceInput {
    /// Opaque profile bearer token (e.g. 'tt_prof_...').
    pub profile_token: String,
    /// Preference category: 'allergy', 'dietary', 'seat_pref', 'favorite_item', or 'note'.
    pub kind: String,
    /// Preference value (e.g. 'peanuts', 'vegetarian', 'patio', 'Paneer Tikka Angaare', or note text).
    pub value: String,
    /// Optional allergy severity: 'mild', 'severe', or 'unknown'. Only applicable when kind is 'allergy'.
    pub severity: Option<String>,
}

pub async fn handle_remember_preference(
    pool: &SqlitePool,
    input: RememberPreferenceInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_remember_preference_with_clock(pool, input, &clock).await
}

pub async fn handle_remember_preference_with_clock(
    pool: &SqlitePool,
    input: RememberPreferenceInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let token_clean = input.profile_token.trim();
    if token_clean.is_empty() {
        return rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            "Invalid input: 'profile_token' must be provided.",
        )]);
    }

    let rate_limiter = get_token_rate_limiter();

    match remember_preference_tx(
        pool,
        token_clean,
        &input.kind,
        &input.value,
        input.severity.as_deref(),
        rate_limiter,
        clock,
    )
    .await
    {
        Ok(Ok(output)) => {
            let voice_text = if output.status == "not_found" {
                "I couldn't find a saved profile matching that token.".to_string()
            } else {
                match input.kind.to_lowercase().as_str() {
                    "allergy" => format!(
                        "I've noted that you would like to avoid {}.",
                        input.value.trim()
                    ),
                    "dietary" => format!(
                        "I've saved {} as your dietary preference.",
                        input.value.trim()
                    ),
                    "seat_pref" => format!(
                        "I've noted your seating preference for {}.",
                        input.value.trim()
                    ),
                    "favorite_item" => format!(
                        "I've saved {} as one of your favorite dishes.",
                        input.value.trim()
                    ),
                    "note" => "I've saved that note to your profile.".to_string(),
                    _ => "I've saved that preference to your profile.".to_string(),
                }
            };

            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return rmcp::model::CallToolResult::error(vec![
                        rmcp::model::ContentBlock::text(format!(
                            "Failed to serialize remember_preference output: {}",
                            e
                        )),
                    ]);
                }
            };

            let mut tool_result = rmcp::model::CallToolResult::structured(structured);
            tool_result.content = vec![rmcp::model::ContentBlock::text(voice_text)];
            tool_result
        }
        Ok(Err(validation_err)) => {
            rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
                validation_err,
            )])
        }
        Err(e) => rmcp::model::CallToolResult::error(vec![rmcp::model::ContentBlock::text(
            format!("Database error saving preference: {}", e),
        )]),
    }
}
