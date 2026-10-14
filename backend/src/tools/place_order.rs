use crate::{
    availability::{Clock, SystemClock},
    ordering::{
        place_order_tx, validate_order_request, AllergenAcknowledgement, OrderLineInput,
        PlaceOrderOutcome, PlaceOrderTxRequest,
    },
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Input parameters for `place_order` MCP tool.
/// Descriptions are tailored for LLM agent callers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlaceOrderInput {
    /// Array of ordered items (1 to 20 lines). Each line must provide either `item_id` (preferred from search_menu)
    /// or `item_name` (exact case-insensitive match), a quantity between 1 and 20, and optional special notes.
    pub items: Vec<OrderLineInput>,

    /// Optional local pickup time: either 'HH:MM' on the restaurant's current local day, or full ISO local date-time.
    /// If omitted, defaults automatically to the earliest feasible kitchen preparation time.
    /// The caller must not invent times.
    pub pickup_time: Option<String>,

    /// Full name of the primary pickup guest (1-80 characters, trimmed).
    pub guest_name: String,

    /// Optional contact phone number for order status notifications. Never logged in full.
    pub phone: Option<String>,

    /// Optional contact email address for electronic receipt. Never logged in full.
    pub email: Option<String>,

    /// Optional allergen restrictions to avoid and guest acknowledgement of verified warnings.
    /// If any ordered dish contains a listed allergen, or if allergen data is unverified, the order is blocked.
    pub allergen_acknowledgement: Option<AllergenAcknowledgement>,

    /// Optional kitchen or packaging notes (max 300 characters, stored as plain text).
    pub order_notes: Option<String>,

    /// Optional idempotency key (8-64 characters matching [a-zA-Z0-9_.-]).
    /// Strongly recommended on retries to ensure at-most-once order placement.
    pub idempotency_key: Option<String>,

    /// Optional profile bearer token (e.g. 'tt_prof_...') to link order, default guest name, and enforce stored allergies.
    pub profile_token: Option<String>,
}

/// Handler for `place_order` tool.
pub async fn handle_place_order(pool: &SqlitePool, input: PlaceOrderInput) -> CallToolResult {
    let clock = SystemClock;
    handle_place_order_with_clock(pool, input, &clock).await
}

pub async fn handle_place_order_with_clock(
    pool: &SqlitePool,
    input: PlaceOrderInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    if let Err(err) = crate::validation::validate_array_bounds(&input.items, 1, 20, "items") {
        return err;
    }

    // Profile token resolution and allergy merging
    let (
        customer_id,
        guest_name_resolved,
        profile_applied,
        needs_reconfirmation,
        effective_allergen_ack,
    ) = if let Some(token_raw) = input
        .profile_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let rate_limiter = crate::memory::get_token_rate_limiter();
        match crate::memory::resolve_profile_token(pool, token_raw, rate_limiter, clock).await {
            Ok(Some(customer)) => {
                let cust_id = customer.id;
                let display_name = if input.guest_name.trim().is_empty() {
                    customer.display_name.clone()
                } else {
                    input.guest_name.clone()
                };

                // Fetch customer's stored allergies
                let allergies: Vec<(String, Option<String>)> = sqlx::query_as(
                        "SELECT value, confirmed_at FROM customer_preferences WHERE customer_id = ? AND kind = 'allergy'"
                    )
                    .bind(cust_id)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default();

                let mut reconf = Vec::new();
                let reconf_cutoff = clock.now()
                    - chrono::Duration::days(crate::memory::ALLERGY_RECONFIRMATION_DAYS);
                let mut avoid_filters = Vec::new();

                for (allergy_name, confirmed_at) in allergies {
                    let is_stale = match &confirmed_at {
                        Some(ts) => match chrono::DateTime::parse_from_rfc3339(ts) {
                            Ok(dt) => dt.with_timezone(&chrono::Utc) < reconf_cutoff,
                            Err(_) => true,
                        },
                        None => true,
                    };
                    if is_stale && !reconf.contains(&allergy_name) {
                        reconf.push(allergy_name.clone());
                    }

                    if let Some(filter) =
                        crate::menu_search::AllergenFilter::from_db_str(&allergy_name)
                    {
                        avoid_filters.push(filter);
                    }
                }

                // Merge guest-supplied list into profile list (guest can add, but cannot remove profile allergies)
                let mut merged_avoid = avoid_filters.clone();
                let has_profile_allergies = !avoid_filters.is_empty();
                if let Some(ref ack) = input.allergen_acknowledgement {
                    for user_avoid in &ack.allergens_to_avoid {
                        if !merged_avoid.contains(user_avoid) {
                            merged_avoid.push(*user_avoid);
                        }
                    }
                }

                // Stored profile allergies must strictly block conflicts and cannot be bypassed
                let acknowledged = if has_profile_allergies {
                    false
                } else {
                    input
                        .allergen_acknowledgement
                        .as_ref()
                        .map(|a| a.acknowledged)
                        .unwrap_or(false)
                };

                let effective_ack = Some(AllergenAcknowledgement {
                    allergens_to_avoid: merged_avoid,
                    acknowledged,
                });

                (
                    Some(cust_id),
                    display_name,
                    Some(true),
                    reconf,
                    effective_ack,
                )
            }
            Ok(None) => {
                // Profile not found / invalid token
                let output = crate::ordering::PlaceOrderOutput {
                        status: "profile_not_found".to_string(),
                        reason: Some("Profile not found or invalid profile token.".to_string()),
                        reason_detail: Some("Stored preferences and allergies could not be applied. Please verify your profile token.".to_string()),
                        order: None,
                        price_changes: vec![],
                        candidates: vec![],
                        alternatives: vec![],
                        earliest_pickup_local: None,
                        allergen_notice: crate::menu_search::ALLERGEN_NOTICE.to_string(),
                        idempotent_replay: false,
                        profile_applied: Some(false),
                        needs_reconfirmation: vec![],
                    };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text(
                    "Profile not found. Stored dining preferences and allergies were not applied.",
                )];
                return res;
            }
            Err(e) => {
                return crate::validation::internal_tool_error("resolving profile token", e);
            }
        }
    } else {
        (
            None,
            input.guest_name.clone(),
            None,
            vec![],
            input.allergen_acknowledgement.clone(),
        )
    };

    // 1. Validation check for tool execution errors
    if let Err(val_err) = validate_order_request(
        &input.items,
        &guest_name_resolved,
        input.phone.as_deref(),
        input.email.as_deref(),
        input.order_notes.as_deref(),
        input.idempotency_key.as_deref(),
    ) {
        tracing::warn!(request_id = %req_id, error = %val_err, "Order validation failed");
        return crate::validation::tool_error(val_err.to_string());
    }

    let tx_req = PlaceOrderTxRequest {
        items: &input.items,
        pickup_time: input.pickup_time.as_deref(),
        guest_name: &guest_name_resolved,
        phone: input.phone.as_deref(),
        email: input.email.as_deref(),
        allergen_ack: effective_allergen_ack.as_ref(),
        order_notes: input.order_notes.as_deref(),
        idempotency_key: input.idempotency_key.as_deref(),
        customer_id,
    };

    match place_order_tx(pool, tx_req, clock).await {
        Ok(outcome) => {
            let (mut output, mut voice_text) = match outcome {
                PlaceOrderOutcome::Placed { output, voice_text } => {
                    let code_prefix = output
                        .order
                        .as_ref()
                        .map(|o| &o.order_code[..o.order_code.len().min(6)])
                        .unwrap_or("TO-?");
                    tracing::info!(
                        request_id = %req_id,
                        order_prefix = %code_prefix,
                        status = "placed",
                        "Pickup order placed successfully"
                    );
                    (output, voice_text)
                }
                PlaceOrderOutcome::ItemUnavailable { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "item_unavailable", "Item sold out");
                    (output, voice_text)
                }
                PlaceOrderOutcome::UnknownItem { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "unknown_item", "Menu item not found");
                    (output, voice_text)
                }
                PlaceOrderOutcome::AmbiguousItem { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "ambiguous_item", "Ambiguous menu item query");
                    (output, voice_text)
                }
                PlaceOrderOutcome::PickupTooSoon { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "pickup_too_soon", "Requested time too soon for kitchen");
                    (output, voice_text)
                }
                PlaceOrderOutcome::Closed { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "closed", "Kitchen closed at requested time");
                    (output, voice_text)
                }
                PlaceOrderOutcome::AllergenConflict { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "allergen_conflict", "Allergen safety conflict detected");
                    (output, voice_text)
                }
                PlaceOrderOutcome::LimitExceeded { output, voice_text } => {
                    tracing::warn!(request_id = %req_id, status = "limit_exceeded", "Order limit exceeded");
                    (output, voice_text)
                }
                PlaceOrderOutcome::IdempotentReplay { output, voice_text } => {
                    tracing::info!(request_id = %req_id, status = "idempotent_replay", "Idempotent order replay returned");
                    (output, voice_text)
                }
                PlaceOrderOutcome::IdempotencyConflict { output, voice_text } => {
                    tracing::warn!(request_id = %req_id, status = "idempotency_conflict", "Idempotency key parameter conflict");
                    (output, voice_text)
                }
            };

            if let Some(applied) = profile_applied {
                output.profile_applied = Some(applied);
                output.needs_reconfirmation = needs_reconfirmation.clone();
                if applied {
                    if !voice_text.contains("Stored allergy preferences") {
                        voice_text
                            .push_str(" Stored allergy preferences were applied to your order.");
                    }
                    if !needs_reconfirmation.is_empty() {
                        voice_text.push_str(
                            " Please confirm if your allergy restrictions are still current.",
                        );
                    }
                }
            }

            let structured_val = match serde_json::to_value(&output) {
                Ok(v) => v,
                Err(err) => {
                    tracing::error!("Failed to serialize PlaceOrderOutput: {}", err);
                    return CallToolResult::error(vec![ContentBlock::text(
                        "An internal serialization error occurred while formatting order result.",
                    )]);
                }
            };

            let mut tool_result = CallToolResult::structured(structured_val);
            tool_result.content = vec![ContentBlock::text(voice_text)];
            tool_result
        }
        Err(e) => {
            tracing::error!(request_id = %req_id, error = %e, "Database transaction error placing order");
            crate::validation::internal_tool_error("placing order", e)
        }
    }
}
