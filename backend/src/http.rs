use crate::config::Config;
use crate::server::TableTalkServer;
use axum::{
    extract::{Request, State},
    http::{header, HeaderName, HeaderValue, Method, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, tower::StreamableHttpService, StreamableHttpServerConfig,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, timeout::TimeoutLayer};

const MAX_BODY_SIZE_BYTES: usize = 2 * 1024 * 1024; // 2 MB

pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2024-11-05"];

/// Redacts payment-token path segments before they reach request logging.
/// Any path segment beginning with the payment token prefix is replaced.
fn redact_token_segments(uri: &str) -> String {
    uri.split('/')
        .map(|seg| {
            if crate::payments::looks_like_payment_token(seg) {
                "tt_pay_[redacted]"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthResponse {
    pub status: String,
}

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Middleware that validates the `Origin` header against an allowlist to prevent DNS rebinding.
/// Requests without an `Origin` header (such as curl, voice assistants, and non-browser CLI clients)
/// are explicitly permitted.
async fn validate_origin(
    State(allowed_origins): State<Arc<Vec<String>>>,
    req: Request,
    next: Next,
) -> Response {
    if let Some(origin_val) = req.headers().get(header::ORIGIN) {
        if let Ok(origin_str) = origin_val.to_str() {
            if !allowed_origins.iter().any(|allowed| allowed == origin_str) {
                return (
                    StatusCode::FORBIDDEN,
                    "Forbidden: Origin not in allowed list\n",
                )
                    .into_response();
            }
        } else {
            return (
                StatusCode::FORBIDDEN,
                "Forbidden: Malformed Origin header\n",
            )
                .into_response();
        }
    }

    next.run(req).await
}

/// Middleware to assign or propagate a unique Request ID and enter a tracing span.
/// Ensures request bodies are never logged at info level, and tracing spans track every request.
async fn request_id_and_tracing(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let method = req.method().clone();
    let uri = redact_token_segments(&req.uri().to_string());

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        uri = %uri,
    );
    let _guard = span.enter();

    let mut response = next.run(req).await;
    if let Ok(val) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), val);
    }
    response
}

/// Middleware to validate Content-Length header against maximum body size limit.
/// Responds cleanly with HTTP 413 Payload Too Large before body streaming.
async fn validate_body_size(req: Request, next: Next) -> Response {
    if let Some(content_length) = req.headers().get(header::CONTENT_LENGTH) {
        if let Ok(len_str) = content_length.to_str() {
            if let Ok(len) = len_str.parse::<usize>() {
                if len > MAX_BODY_SIZE_BYTES {
                    return (
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Payload Too Large: Request body exceeds 2MB limit\n",
                    )
                        .into_response();
                }
            }
        }
    }

    next.run(req).await
}

/// Diagnostic middleware for testing timeout cut-off under controlled latency simulation.
async fn simulate_delay_for_test(req: Request, next: Next) -> Response {
    if let Some(delay_val) = req.headers().get("x-test-simulate-delay-ms") {
        if let Ok(delay_str) = delay_val.to_str() {
            if let Ok(delay_ms) = delay_str.parse::<u64>() {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    next.run(req).await
}

/// Middleware to validate `MCP-Protocol-Version` header on requests if present.
/// If an unsupported version is sent, rejects with HTTP 400 Bad Request per spec.
async fn validate_protocol_version(req: Request, next: Next) -> Response {
    let header_name = HeaderName::from_static("mcp-protocol-version");
    if let Some(version_val) = req.headers().get(&header_name) {
        if let Ok(version_str) = version_val.to_str() {
            if !SUPPORTED_PROTOCOL_VERSIONS.contains(&version_str) {
                return (
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Bad Request: Unsupported MCP-Protocol-Version: {}\n",
                        version_str
                    ),
                )
                    .into_response();
            }
        } else {
            return (
                StatusCode::BAD_REQUEST,
                "Bad Request: Malformed MCP-Protocol-Version header\n".to_string(),
            )
                .into_response();
        }
    }

    next.run(req).await
}

pub fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let req_id = uuid::Uuid::new_v4();
    let panic_msg = if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic payload".to_string()
    };
    tracing::error!(
        request_id = %req_id,
        "Server panic intercepted by CatchPanicLayer: {}",
        panic_msg
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal Server Error: An unexpected error occurred. Please try again.\n",
    )
        .into_response()
}

/// Build the full application router with health check, MCP service, CORS, and security middleware.
pub fn build_router(config: &Config) -> Router {
    build_router_with_db(config, None)
}

/// Build the application router with an optional database connection pool wired into TableTalkServer.
pub fn build_router_with_db(config: &Config, pool: Option<sqlx::SqlitePool>) -> Router {
    let server = match &pool {
        Some(p) => TableTalkServer::with_db(p.clone()),
        None => TableTalkServer::new(),
    };
    let pool_for_checkout = pool;
    let session_manager = Arc::new(LocalSessionManager::default());

    let mcp_config =
        StreamableHttpServerConfig::default().with_allowed_origins(config.allowed_origins.clone());

    let mcp_service =
        StreamableHttpService::new(move || Ok(server.clone()), session_manager, mcp_config);

    let allowed_origins_state = Arc::new(config.allowed_origins.clone());
    let ip_rate_limiter = Arc::new(crate::rate_limit::IpRateLimiter::new(
        config.rate_limit_per_minute,
    ));

    // Restrictive CORS configuration
    let parsed_origins: Vec<HeaderValue> = config
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(parsed_origins)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
            HeaderName::from_static("mcp-protocol-version"),
            HeaderName::from_static("mcp-session-id"),
            HeaderName::from_static("x-request-id"),
        ])
        .expose_headers([
            HeaderName::from_static("mcp-session-id"),
            HeaderName::from_static("mcp-protocol-version"),
            HeaderName::from_static("x-request-id"),
        ])
        .max_age(Duration::from_secs(3600));

    // Rate-limited router for MCP and checkout endpoints
    let protected_endpoints = Router::new()
        .merge(match pool_for_checkout {
            Some(p) => crate::checkout::checkout_router(crate::checkout::CheckoutState {
                pool: p,
                clock: Arc::new(crate::availability::SystemClock),
                rate_limiter: Arc::new(crate::checkout::CheckoutRateLimiter::new()),
            }),
            None => Router::new(),
        })
        .nest_service("/mcp", mcp_service)
        .layer(from_fn_with_state(
            ip_rate_limiter,
            crate::rate_limit::rate_limit_middleware,
        ));

    Router::new()
        .route("/health", get(health_handler))
        .merge(protected_endpoints)
        .layer(tower_http::catch_panic::CatchPanicLayer::custom(
            handle_panic,
        ))
        .layer(from_fn_with_state(allowed_origins_state, validate_origin))
        .layer(axum::middleware::from_fn(validate_protocol_version))
        .layer(axum::middleware::from_fn(validate_body_size))
        .layer(axum::middleware::from_fn(simulate_delay_for_test))
        .layer(axum::middleware::from_fn(request_id_and_tracing))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(MAX_BODY_SIZE_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(config.request_timeout_secs),
        ))
}
