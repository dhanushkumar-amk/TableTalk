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
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
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

    // 1. Validation check for tool execution errors
    if let Err(val_err) = validate_order_request(
        &input.items,
        &input.guest_name,
        input.phone.as_deref(),
        input.email.as_deref(),
        input.order_notes.as_deref(),
        input.idempotency_key.as_deref(),
    ) {
        tracing::warn!(request_id = %req_id, error = %val_err, "Order validation failed");
        return tool_validation_error(val_err.to_string());
    }

    let tx_req = PlaceOrderTxRequest {
        items: &input.items,
        pickup_time: input.pickup_time.as_deref(),
        guest_name: &input.guest_name,
        phone: input.phone.as_deref(),
        email: input.email.as_deref(),
        allergen_ack: input.allergen_acknowledgement.as_ref(),
        order_notes: input.order_notes.as_deref(),
        idempotency_key: input.idempotency_key.as_deref(),
    };

    match place_order_tx(pool, tx_req, clock).await {
        Ok(outcome) => {
            let (output, voice_text) = match outcome {
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
            tool_validation_error(format!("Internal order placement error: {}", e))
        }
    }
}
