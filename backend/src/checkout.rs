//! Mock checkout HTTP endpoints (Phase 12).
//!
//! These endpoints are deliberately separate from `/mcp`: no MCP tool can
//! complete a payment. They accept only an `outcome` selector, never card data.
//! The same Origin allowlist and CORS policy as `/mcp` apply (the routes are
//! mounted on the same router).

use crate::availability::Clock;
use crate::payments::{
    self, body_contains_card_data, CheckoutOutcome, CheckoutResult, PaymentError,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Per-token completion limit within the sliding window.
pub const MAX_COMPLETE_PER_TOKEN_PER_MIN: usize = 10;
/// Per-IP completion limit within the sliding window.
pub const MAX_COMPLETE_PER_IP_PER_MIN: usize = 60;
const RATE_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct CheckoutState {
    pub pool: sqlx::SqlitePool,
    pub clock: Arc<dyn Clock>,
    pub rate_limiter: Arc<CheckoutRateLimiter>,
}

/// Simple in-memory sliding-window limiter keyed by token and by client IP.
#[derive(Debug, Default)]
pub struct CheckoutRateLimiter {
    per_token: Mutex<HashMap<String, VecDeque<Instant>>>,
    per_ip: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl CheckoutRateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    fn check(map: &Mutex<HashMap<String, VecDeque<Instant>>>, key: &str, limit: usize) -> bool {
        let now = Instant::now();
        let mut guard = map.lock().unwrap();
        let entry = guard.entry(key.to_string()).or_default();
        while let Some(front) = entry.front() {
            if now.duration_since(*front) > RATE_WINDOW {
                entry.pop_front();
            } else {
                break;
            }
        }
        if entry.len() >= limit {
            return false;
        }
        entry.push_back(now);
        true
    }

    /// Returns true when the request is allowed.
    pub fn allow(&self, token_hash: &str, ip: &str) -> bool {
        // Check IP first (cheaper blast-radius), then token.
        if !Self::check(&self.per_ip, ip, MAX_COMPLETE_PER_IP_PER_MIN) {
            return false;
        }
        Self::check(&self.per_token, token_hash, MAX_COMPLETE_PER_TOKEN_PER_MIN)
    }
}

#[derive(Debug, Deserialize)]
pub struct CompleteBody {
    pub outcome: Option<String>,
}

/// Build the checkout router. Merged into the main router so it inherits CORS,
/// Origin validation, body-size limits and the request-id middleware.
pub fn checkout_router(state: CheckoutState) -> Router {
    Router::new()
        .route("/mock-checkout/{token}", get(get_session))
        .route("/mock-checkout/{token}/complete", post(complete_session))
        .with_state(state)
}

fn generic_not_found() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "status": "not_found",
            "simulated": true,
            "message": "That payment link is no longer valid."
        })),
    )
        .into_response()
}

fn generic_bad_request() -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "invalid_request",
            "simulated": true,
            "message": "The request could not be processed."
        })),
    )
        .into_response()
}

/// GET /mock-checkout/{token}
async fn get_session(State(state): State<CheckoutState>, Path(token): Path<String>) -> Response {
    let request_id = uuid::Uuid::new_v4();
    let token_hash = payments::hash_payment_token(&token);

    let session: Result<Option<crate::db::models::PaymentSession>, sqlx::Error> = sqlx::query_as(
        "SELECT id, booking_id, token_hash, amount_minor, currency, status, expires_at, decline_count, created_at
         FROM payment_sessions WHERE token_hash = ?",
    )
    .bind(&token_hash)
    .fetch_optional(&state.pool)
    .await;

    let session = match session {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::info!(request_id = %request_id, "checkout session lookup: not found");
            return generic_not_found();
        }
        Err(e) => {
            tracing::error!(request_id = %request_id, "checkout session lookup failed: {}", e);
            return generic_not_found();
        }
    };

    if session.status != crate::db::models::SessionStatus::Open {
        return generic_not_found();
    }

    let restaurant: Option<(String, String)> =
        sqlx::query_as("SELECT name, timezone FROM restaurant WHERE id = 1")
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();
    let (restaurant_name, timezone) = restaurant.unwrap_or_else(|| {
        (
            "The Saffron Nebula Bistro".to_string(),
            "Asia/Kolkata".to_string(),
        )
    });
    let tz: chrono_tz::Tz = timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);

    let deadline = chrono::DateTime::parse_from_rfc3339(&session.expires_at)
        .map(|d| d.with_timezone(&tz).to_rfc3339())
        .unwrap_or_else(|_| session.expires_at.clone());

    let booking_info: Option<(i64, String)> =
        sqlx::query_as("SELECT party_size, start_at FROM bookings WHERE id = ?")
            .bind(&session.booking_id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

    let (booking_party_size, booking_start_at) = match booking_info {
        Some((ps, sa)) => {
            let local_start = chrono::DateTime::parse_from_rfc3339(&sa)
                .map(|d| d.with_timezone(&tz).to_rfc3339())
                .unwrap_or(sa);
            (Some(ps), Some(local_start))
        }
        None => (None, None),
    };

    tracing::info!(
        request_id = %request_id,
        session_id = %session.id,
        "checkout session info returned (simulated)"
    );

    Json(json!({
        "status": "open",
        "simulated": true,
        "provider": payments::PROVIDER_NAME,
        "restaurant_name": restaurant_name,
        "amount": payments::money(session.amount_minor, &session.currency),
        "deadline_local": deadline,
        "party_size": booking_party_size,
        "start_at_local": booking_start_at,
        "message": "SIMULATED PAYMENT - no real money will move."
    }))
    .into_response()
}

/// POST /mock-checkout/{token}/complete
async fn complete_session(
    State(state): State<CheckoutState>,
    headers: HeaderMap,
    Path(token): Path<String>,
    raw_body: String,
) -> Response {
    let request_id = uuid::Uuid::new_v4();
    let token_hash = payments::hash_payment_token(&token);

    // Card-data safeguard, applied before anything is logged or parsed further.
    let parsed: serde_json::Value = match serde_json::from_str(&raw_body) {
        Ok(v) => v,
        Err(_) => return generic_bad_request(),
    };
    if body_contains_card_data(&parsed) {
        // Generic 400; the body is never logged.
        tracing::warn!(request_id = %request_id, "checkout request rejected: card-like payload");
        return generic_bad_request();
    }

    // Client identity for per-IP rate limiting: X-Forwarded-For / X-Real-IP when
    // behind a proxy, else a shared bucket. Per-token limits give the primary
    // protection regardless.
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    if !state.rate_limiter.allow(&token_hash, &ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "status": "rate_limited",
                "simulated": true,
                "message": "Too many attempts. Please wait a moment and try again."
            })),
        )
            .into_response();
    }

    let body: CompleteBody = match serde_json::from_value(parsed) {
        Ok(b) => b,
        Err(_) => return generic_bad_request(),
    };
    let outcome = match body.outcome.as_deref().and_then(CheckoutOutcome::parse) {
        Some(o) => o,
        None => return generic_bad_request(),
    };

    match payments::complete_checkout(&state.pool, &token, outcome, state.clock.as_ref()).await {
        Ok(result) => {
            tracing::info!(
                request_id = %request_id,
                session_id = %token_hash.chars().take(12).collect::<String>(),
                outcome = %result.status,
                "checkout completion processed (simulated)"
            );
            if result.status == "not_found" {
                return generic_not_found();
            }
            (StatusCode::OK, Json(result)).into_response()
        }
        Err(PaymentError::Db(e)) => {
            tracing::error!(request_id = %request_id, "checkout completion failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"status": "error", "simulated": true, "message": "The payment could not be processed."})),
            )
                .into_response()
        }
        Err(_) => generic_not_found(),
    }
}

/// Convenience for tests: parse a checkout response payload.
pub fn parse_checkout_result(value: &serde_json::Value) -> Option<CheckoutResult> {
    serde_json::from_value(value.clone()).ok()
}
