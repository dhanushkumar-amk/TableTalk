use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{Clock, SystemClock},
    memory::{forget_preference_tx, get_token_rate_limiter},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ForgetPreferenceInput {
    /// Opaque profile bearer token (e.g. 'tt_prof_...').
    pub profile_token: String,
    /// Preference category: 'allergy', 'dietary', 'seat_pref', 'favorite_item', or 'note'.
    pub kind: String,
    /// Exact preference value to remove.
    pub value: String,
    /// Required confirmation when removing an allergy (must be true).
    pub confirm_remove_allergy: Option<bool>,
}

pub async fn handle_forget_preference(
    pool: &SqlitePool,
    input: ForgetPreferenceInput,
) -> rmcp::model::CallToolResult {
    let clock = SystemClock;
    handle_forget_preference_with_clock(pool, input, &clock).await
}

pub async fn handle_forget_preference_with_clock(
    pool: &SqlitePool,
    input: ForgetPreferenceInput,
    clock: &dyn Clock,
) -> rmcp::model::CallToolResult {
    let token_clean =
        match crate::validation::sanitize_string(&input.profile_token, "profile_token", 1, 128) {
            Ok(t) => t,
            Err(err) => return err,
        };

    let kind_clean = match crate::validation::sanitize_string(&input.kind, "kind", 1, 32) {
        Ok(k) => k,
        Err(err) => return err,
    };

    let value_clean = match crate::validation::sanitize_string(&input.value, "value", 1, 200) {
        Ok(v) => v,
        Err(err) => return err,
    };

    let rate_limiter = get_token_rate_limiter();
    let confirm_allergy = input.confirm_remove_allergy.unwrap_or(false);

    match forget_preference_tx(
        pool,
        &token_clean,
        &kind_clean,
        &value_clean,
        confirm_allergy,
        rate_limiter,
        clock,
    )
    .await
    {
        Ok(Ok(output)) => {
            let voice_text = if output.status == "confirmation_required" {
                "Removing a saved allergy from your profile requires your explicit confirmation. Please confirm if you wish to remove this allergy.".to_string()
            } else if output.status == "not_found" {
                format!(
                    "I couldn't find '{}' in your saved profile preferences.",
                    value_clean
                )
            } else if kind_clean.to_lowercase() == "allergy" {
                format!(
                    "I've removed {} from your saved profile preferences. Please remember to inform restaurant staff of any active allergies when ordering.",
                    value_clean
                )
            } else {
                format!("I've removed {} from your saved preferences.", value_clean)
            };

            let structured = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(e) => {
                    return crate::validation::internal_tool_error(
                        "serializing forget_preference output",
                        e,
                    );
                }
            };

            let mut tool_result = rmcp::model::CallToolResult::structured(structured);
            tool_result.content = vec![rmcp::model::ContentBlock::text(voice_text)];
            tool_result
        }
        Ok(Err(validation_err)) => crate::validation::tool_error(validation_err),
        Err(e) => crate::validation::internal_tool_error("removing preference", e),
    }
}
