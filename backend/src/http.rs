use crate::config::Config;
use crate::server::TableTalkServer;
use axum::{
    extract::{DefaultBodyLimit, Request, State},
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
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer};

const MAX_BODY_SIZE_BYTES: usize = 2 * 1024 * 1024; // 2 MB
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2024-11-05"];

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
        }
    }

    next.run(req).await
}

/// Build the full application router with health check, MCP service, CORS, and security middleware.
pub fn build_router(config: &Config) -> Router {
    let server = TableTalkServer::new();
    let session_manager = Arc::new(LocalSessionManager::default());

    let mcp_config =
        StreamableHttpServerConfig::default().with_allowed_origins(config.allowed_origins.clone());

    let mcp_service =
        StreamableHttpService::new(move || Ok(server.clone()), session_manager, mcp_config);

    let allowed_origins_state = Arc::new(config.allowed_origins.clone());

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
        ])
        .expose_headers([
            HeaderName::from_static("mcp-session-id"),
            HeaderName::from_static("mcp-protocol-version"),
        ])
        .max_age(Duration::from_secs(3600));

    Router::new()
        .route("/health", get(health_handler))
        .nest_service("/mcp", mcp_service)
        .layer(from_fn_with_state(allowed_origins_state, validate_origin))
        .layer(axum::middleware::from_fn(validate_protocol_version))
        .layer(cors)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(REQUEST_TIMEOUT_SECS),
        ))
}
