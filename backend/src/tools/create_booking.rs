use crate::{
    availability::{AvailabilityConfig, Clock, SystemClock},
    booking::{
        create_booking_tx, validate_create_booking_request, CreateBookingOutcome,
        CreateBookingTxRequest,
    },
};
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Seating area preference for reservation requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum BookingAreaInput {
    #[default]
    Any,
    Indoor,
    Patio,
    Bar,
}

impl BookingAreaInput {
    pub fn as_str(&self) -> &'static str {
        match self {
            BookingAreaInput::Any => "any",
            BookingAreaInput::Indoor => "indoor",
            BookingAreaInput::Patio => "patio",
            BookingAreaInput::Bar => "bar",
        }
    }
}

/// Input parameters for the `create_booking` MCP tool.
/// Descriptions are written for LLM agent callers.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateBookingInput {
    /// Local restaurant date in ISO format YYYY-MM-DD (e.g. '2026-10-17'). \
    /// The caller must resolve relative phrases like 'tomorrow' or 'Friday' to a concrete date itself, \
    /// using the restaurant timezone (Asia/Kolkata).
    pub date: String,

    /// Local 24-hour reservation time in HH:MM format (e.g. '19:30').
    pub time: String,

    /// Party size (number of guests). Must be at least 1 and cannot exceed restaurant maximum capacity.
    pub party_size: i64,

    /// Name of the primary dining guest (1 to 80 characters, trimmed).
    pub guest_name: String,

    /// Optional contact phone number for reservation notifications and SMS reminders.
    pub phone: Option<String>,

    /// Optional contact email address for booking confirmation receipts.
    pub email: Option<String>,

    /// Seating area preference: 'indoor', 'patio', 'bar', or 'any'. Defaults to 'any'.
    pub area: Option<BookingAreaInput>,

    /// Optional dining duration in minutes (30-240). If omitted, defaults by party size \
    /// (75m for 1-2 guests, 90m for 3-4 guests, 105m for 5-6 guests, 120m for 7+ guests).
    pub duration_minutes: Option<i64>,

    /// Optional dietary notes, seating requests, or celebration comments (max 300 characters).
    pub special_requests: Option<String>,

    /// Optional idempotency key (8-64 characters matching [a-zA-Z0-9_.-]). \
    /// Strongly recommended on client retries to guarantee at-most-once booking creation.
    pub idempotency_key: Option<String>,

    /// Optional returning customer identifier if known from session history.
    pub customer_id: Option<i64>,

    /// Optional profile bearer token to link booking, default guest name, and honor seating preferences.
    pub profile_token: Option<String>,

    /// Optional negotiate flag. When true and requested reservation is unavailable, returns negotiation_id and ranked alternative options.
    pub negotiate: Option<bool>,
}

fn tool_validation_error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// Handler for `create_booking` tool.
pub async fn handle_create_booking(pool: &SqlitePool, input: CreateBookingInput) -> CallToolResult {
    let clock = SystemClock;
    handle_create_booking_with_clock(pool, input, &clock).await
}

pub async fn handle_create_booking_with_clock(
    pool: &SqlitePool,
    input: CreateBookingInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let req_id = uuid::Uuid::new_v4();

    // Profile token resolution and seat preference extraction
    let (customer_id_resolved, guest_name_resolved, profile_applied, preferred_area) = if let Some(
        token_raw,
    ) = input
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

                // Look up seat preference
                let seat_pref: Option<String> = sqlx::query_scalar(
                        "SELECT value FROM customer_preferences WHERE customer_id = ? AND kind = 'seat_pref' LIMIT 1"
                    )
                    .bind(cust_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or_default();

                let area_hint = seat_pref.and_then(|val| {
                    let lower = val.to_lowercase();
                    if lower.contains("patio") {
                        Some("patio")
                    } else if lower.contains("bar") {
                        Some("bar")
                    } else if lower.contains("indoor") || lower.contains("booth") {
                        Some("indoor")
                    } else {
                        None
                    }
                });

                (Some(cust_id), display_name, Some(true), area_hint)
            }
            Ok(None) => {
                let output = crate::booking::CreateBookingOutput {
                        status: "profile_not_found".to_string(),
                        reason: Some("Profile not found or invalid profile token.".to_string()),
                        reason_detail: Some("Stored preferences could not be applied. Please verify your profile token.".to_string()),
                        booking: None,
                        idempotent_replay: false,
                        alternatives: vec![],
                        cancellation_policy: None,
                        note: "availability is a snapshot, not a reservation".to_string(),
                        profile_applied: Some(false),
                        seat_preference_honored: None,
                    };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text(
                    "Profile not found. Stored dining preferences were not applied.",
                )];
                return res;
            }
            Err(e) => {
                return CallToolResult::error(vec![ContentBlock::text(format!(
                    "Database error resolving profile token: {}",
                    e
                ))]);
            }
        }
    } else {
        (input.customer_id, input.guest_name.clone(), None, None)
    };

    // 1. Validation check for tool execution errors
    if let Err(val_err) = validate_create_booking_request(
        &input.date,
        &input.time,
        input.party_size,
        &guest_name_resolved,
        input.phone.as_deref(),
        input.email.as_deref(),
        input.special_requests.as_deref(),
        input.idempotency_key.as_deref(),
        12, // Max restaurant party size
    ) {
        return tool_validation_error(val_err.to_string());
    }

    let explicit_area = input.area.map(|a| a.as_str());
    let engine_config = AvailabilityConfig::default();

    // Soft seating preference tie-break: if user didn't specify an area (or specified any) and profile has an area preference
    let (outcome, seat_preference_honored) = if explicit_area.is_none()
        || explicit_area == Some("any")
    {
        if let Some(pref_area) = preferred_area {
            let tx_req_pref = CreateBookingTxRequest {
                date: &input.date,
                time: &input.time,
                party_size: input.party_size,
                guest_name: &guest_name_resolved,
                phone: input.phone.as_deref(),
                email: input.email.as_deref(),
                area: Some(pref_area),
                duration_minutes: input.duration_minutes,
                special_requests: input.special_requests.as_deref(),
                idempotency_key: input.idempotency_key.as_deref(),
                customer_id: customer_id_resolved,
                negotiate: input.negotiate.unwrap_or(false),
            };

            match create_booking_tx(pool, tx_req_pref, &engine_config, clock).await {
                Ok(CreateBookingOutcome::Created { output, voice_text }) => (
                    CreateBookingOutcome::Created { output, voice_text },
                    Some(true),
                ),
                _ => {
                    // Fall back to any so soft preference never causes booking to fail
                    let tx_req_any = CreateBookingTxRequest {
                        date: &input.date,
                        time: &input.time,
                        party_size: input.party_size,
                        guest_name: &guest_name_resolved,
                        phone: input.phone.as_deref(),
                        email: input.email.as_deref(),
                        area: explicit_area,
                        duration_minutes: input.duration_minutes,
                        special_requests: input.special_requests.as_deref(),
                        idempotency_key: input.idempotency_key.as_deref(),
                        customer_id: customer_id_resolved,
                        negotiate: input.negotiate.unwrap_or(false),
                    };
                    let outcome_any = match create_booking_tx(
                        pool,
                        tx_req_any,
                        &engine_config,
                        clock,
                    )
                    .await
                    {
                        Ok(o) => o,
                        Err(err) => {
                            tracing::error!(request_id = %req_id, "Failed to create booking in database transaction: {}", err);
                            return CallToolResult::error(vec![ContentBlock::text(
                                "An internal database error occurred while creating your reservation. Please try again.",
                            )]);
                        }
                    };
                    (outcome_any, Some(false))
                }
            }
        } else {
            let tx_req = CreateBookingTxRequest {
                date: &input.date,
                time: &input.time,
                party_size: input.party_size,
                guest_name: &guest_name_resolved,
                phone: input.phone.as_deref(),
                email: input.email.as_deref(),
                area: explicit_area,
                duration_minutes: input.duration_minutes,
                special_requests: input.special_requests.as_deref(),
                idempotency_key: input.idempotency_key.as_deref(),
                customer_id: customer_id_resolved,
                negotiate: input.negotiate.unwrap_or(false),
            };
            let out = match create_booking_tx(pool, tx_req, &engine_config, clock).await {
                Ok(o) => o,
                Err(err) => {
                    tracing::error!(request_id = %req_id, "Failed to create booking in database transaction: {}", err);
                    return CallToolResult::error(vec![ContentBlock::text(
                        "An internal database error occurred while creating your reservation. Please try again.",
                    )]);
                }
            };
            (out, None)
        }
    } else {
        let tx_req = CreateBookingTxRequest {
            date: &input.date,
            time: &input.time,
            party_size: input.party_size,
            guest_name: &guest_name_resolved,
            phone: input.phone.as_deref(),
            email: input.email.as_deref(),
            area: explicit_area,
            duration_minutes: input.duration_minutes,
            special_requests: input.special_requests.as_deref(),
            idempotency_key: input.idempotency_key.as_deref(),
            customer_id: customer_id_resolved,
            negotiate: input.negotiate.unwrap_or(false),
        };
        let out = match create_booking_tx(pool, tx_req, &engine_config, clock).await {
            Ok(o) => o,
            Err(err) => {
                tracing::error!(request_id = %req_id, "Failed to create booking in database transaction: {}", err);
                return CallToolResult::error(vec![ContentBlock::text(
                    "An internal database error occurred while creating your reservation. Please try again.",
                )]);
            }
        };
        (out, None)
    };

    // 3. Format outcome into CallToolResult
    let (mut output, mut voice_text) = match outcome {
        CreateBookingOutcome::Created { output, voice_text } => {
            if let Some(ref b) = output.booking {
                let code_prefix = &b.confirmation_code[..b.confirmation_code.len().min(6)];
                tracing::info!(
                    request_id = %req_id,
                    code_prefix = %code_prefix,
                    party_size = b.party_size,
                    "Reservation created successfully"
                );
            }
            (output, voice_text)
        }
        CreateBookingOutcome::IdempotentReplay { output, voice_text } => {
            tracing::info!(
                request_id = %req_id,
                "Idempotent reservation replay returned"
            );
            (output, voice_text)
        }
        CreateBookingOutcome::Unavailable { output, voice_text } => (output, voice_text),
        CreateBookingOutcome::LimitExceeded { output, voice_text } => (output, voice_text),
        CreateBookingOutcome::IdempotencyConflict { output, voice_text } => (output, voice_text),
    };

    if let Some(applied) = profile_applied {
        output.profile_applied = Some(applied);
        output.seat_preference_honored = seat_preference_honored;
        if seat_preference_honored == Some(true) {
            voice_text.push_str(" Your preferred seating area has been reserved.");
        }
    }

    let structured_val = match serde_json::to_value(&output) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("Failed to serialize CreateBookingOutput: {}", err);
            return CallToolResult::error(vec![ContentBlock::text(
                "An internal serialization error occurred while formatting booking result.",
            )]);
        }
    };

    let mut tool_result = CallToolResult::structured(structured_val);
    tool_result.content = vec![ContentBlock::text(voice_text)];
    tool_result
}
