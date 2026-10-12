use serde_json::Value;
use std::time::Duration;
use tabletalk_backend::{build_router, Config};

/// Helper to parse JSON from an SSE or JSON response body.
fn extract_json_from_response(raw: &str) -> Value {
    for line in raw.lines() {
        let line = line.trim();
        if let Some(json_text) = line.strip_prefix("data:") {
            let json_text = json_text.trim();
            if !json_text.is_empty() {
                if let Ok(val) = serde_json::from_str::<Value>(json_text) {
                    return val;
                }
            }
        }
    }
    serde_json::from_str::<Value>(raw)
        .unwrap_or_else(|_| panic!("Failed to parse response as JSON or SSE data: {}", raw))
}

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral listener");
    let addr = listener.local_addr().expect("Failed to get local address");
    let base_url = format!("http://{}", addr);

    let config = Config {
        port: addr.port(),
        allowed_origins: vec![
            "http://localhost:5173".to_string(),
            "http://127.0.0.1:5173".to_string(),
            "http://localhost:8080".to_string(),
            format!("http://{}", addr),
        ],
        rust_log: "warn".to_string(),
        request_timeout_secs: 30,
    };

    let router = build_router(&config);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (base_url, handle)
}

/// Helper to establish a valid initialized MCP session for subsequent request tests.
async fn init_session(client: &reqwest::Client, base_url: &str) -> String {
    let init_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }))
        .send()
        .await
        .expect("Failed to send initialize request");

    assert_eq!(init_resp.status(), reqwest::StatusCode::OK);
    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .expect("Expected mcp-session-id header")
        .to_str()
        .unwrap()
        .to_string();

    // Complete handshake with notifications/initialized
    let notif_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .send()
        .await
        .expect("Failed to send notifications/initialized");

    assert_eq!(notif_resp.status(), reqwest::StatusCode::ACCEPTED);
    session_id
}

// ---------------------------------------------------------------------------
// a) Malformed JSON body
// Spec reference: JSON-RPC 2.0 §5.1, MCP Transports (Streamable HTTP)
// Expected: Transport returns HTTP 415 or 400 with parse failure diagnostic.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_a_malformed_json_body() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body("{ this is : definitely not valid json")
        .send()
        .await
        .expect("Request failed");

    // rmcp streamable HTTP returns 415 with deserialization error message
    assert_eq!(resp.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let text = resp.text().await.unwrap();
    assert!(text.contains("fail to deserialize request body"));

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// b) Valid JSON that is not a valid JSON-RPC message
// Spec reference: JSON-RPC 2.0 §4, §5.1
// Expected: Rejection as invalid message variant (HTTP 415 or JSON-RPC -32600).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_b_valid_json_not_jsonrpc() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "arbitrary_field": "hello",
            "not_jsonrpc": 123
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let text = resp.text().await.unwrap();
    assert!(text.contains("did not match any variant of untagged enum JsonRpcMessage"));

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// c) Unknown method
// Spec reference: JSON-RPC 2.0 §5.1 (code: -32601 Method not found)
// Expected: Server responds with JSON-RPC error code -32601 on initialized session.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_c_unknown_method() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "kitchen/cook_nonexistent"
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body_text = resp.text().await.unwrap();
    let json = extract_json_from_response(&body_text);

    assert_eq!(json["jsonrpc"], "2.0");
    assert_eq!(json["id"], 42);
    let error = &json["error"];
    assert_eq!(error["code"], -32601); // Standard JSON-RPC Method not found

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// d) tools/call before initialize
// Spec reference: MCP Lifecycle §Initialization, Transports §Session Management
// Expected: Rejected before session initialization (HTTP 422 or 400).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_d_tools_call_before_initialize() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "ping_restaurant",
                "arguments": {}
            }
        }))
        .send()
        .await
        .expect("Request failed");

    // rmcp rejects non-initialize initial message with 422 Unprocessable Entity
    assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    let text = resp.text().await.unwrap();
    assert!(text.contains("Unexpected message, expect initialize request"));

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// e) Unsupported protocolVersion in initialize
// Spec reference: MCP Lifecycle §Protocol version negotiation
// Expected: Server responds with a version it supports (2025-11-25).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_e_unsupported_protocol_version_negotiation() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Client advertises an unknown future version
    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "9999-12-31",
                "capabilities": {},
                "clientInfo": {"name": "future-bot", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body_text = resp.text().await.unwrap();
    let json = extract_json_from_response(&body_text);

    assert_eq!(json["result"]["protocolVersion"], "2025-11-25");

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// f) Missing or invalid MCP-Protocol-Version header on later requests
// Spec reference: MCP Transports §Streamable HTTP (PR #1439)
// Expected: HTTP 400 Bad Request for unsupported/invalid protocol version header.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_f_invalid_protocol_version_header() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    // Send request with unsupported protocol version header
    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "1999-01-01")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping"
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// g) Invalid/unknown Mcp-Session-Id and request after session termination (DELETE)
// Spec reference: MCP Transports §Session Management
// Expected: Unknown session returns 404; DELETE /mcp returns 202; subsequent request returns 404.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_g_session_termination_and_unknown_session() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. Unknown session id yields 404 Not Found
    let fake_session_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", "00000000-0000-0000-0000-000000000000")
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(fake_session_resp.status(), reqwest::StatusCode::NOT_FOUND);

    // 2. Initialize real session then terminate via DELETE /mcp
    let session_id = init_session(&client, &base_url).await;

    let delete_resp = client
        .delete(format!("{}/mcp", base_url))
        .header("mcp-session-id", &session_id)
        .send()
        .await
        .expect("DELETE request failed");

    assert_eq!(delete_resp.status(), reqwest::StatusCode::ACCEPTED);

    // 3. Subsequent request with terminated session yields 404 Not Found
    let post_delete_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "ping"
        }))
        .send()
        .await
        .expect("Post-delete request failed");

    assert_eq!(post_delete_resp.status(), reqwest::StatusCode::NOT_FOUND);

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// h) Wrong Content-Type and missing Accept header
// Spec reference: MCP Transports §Streamable HTTP
// Expected: Wrong Content-Type -> 415; Missing required Accept -> 406.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_h_wrong_content_type_and_missing_accept() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. Wrong Content-Type (text/plain)
    let wrong_ct_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "text/plain")
        .header("Accept", "application/json, text/event-stream")
        .body("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        wrong_ct_resp.status(),
        reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE
    );

    // 2. Missing Accept header
    let missing_accept_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        missing_accept_resp.status(),
        reqwest::StatusCode::NOT_ACCEPTABLE
    );

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// i) JSON-RPC batch behavior
// Spec reference: MCP 2025-11-25 Transports §JSON-RPC Envelope
// Expected: Array envelopes are rejected (HTTP 415 in rmcp).
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_i_jsonrpc_batch_behavior() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!([
            {"jsonrpc": "2.0", "id": 1, "method": "ping"},
            {"jsonrpc": "2.0", "id": 2, "method": "ping"}
        ]))
        .send()
        .await
        .expect("Batch request failed");

    // rmcp strictly expects single JsonRpcMessage variant and rejects batches
    assert_eq!(resp.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE);

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// j) Oversized request body
// Spec reference: RFC 9110 §15.5.14 (413 Payload Too Large) & TableTalk limits
// Expected: Rejection with HTTP 413 Payload Too Large without server crash.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_j_oversized_request_body() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Generate 3MB payload (exceeds 2MB limit)
    let oversized_string = "x".repeat(3 * 1024 * 1024);
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": oversized_string, "version": "1.0"}
        }
    });

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&payload)
        .send()
        .await;

    // The server rejects the oversized payload either via HTTP 413 or by terminating
    // the TCP stream before consuming excessive bytes. Neither causes a crash.
    match resp {
        Ok(response) => {
            assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        }
        Err(err) => {
            // Connection reset/aborted by server upon exceeding body limit
            assert!(err.is_request() || err.is_connect());
        }
    }

    // Verify the server remains alive, responsive, and did not crash
    let health_resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Server must remain healthy after rejecting oversized request");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// k) Client notification (no id)
// Spec reference: MCP Transports §Streamable HTTP
// Expected: HTTP 202 Accepted with empty body.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_k_client_notification() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .send()
        .await
        .expect("Notification request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::ACCEPTED);
    let body = resp.text().await.unwrap();
    assert!(body.is_empty(), "Notification body must be empty");

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// l) Disallowed Origin (403), allowed Origin (200), and no Origin (200)
// Spec reference: MCP Transports §Security and Origin Validation
// Expected: 403 Forbidden on disallowed; 200 OK on allowed and no Origin.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_l_origin_header_validation() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let init_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });

    // 1. Disallowed Origin -> 403 Forbidden
    let disallowed = client
        .post(format!("{}/mcp", base_url))
        .header("Origin", "http://unauthorized-domain.com")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .unwrap();
    assert_eq!(disallowed.status(), reqwest::StatusCode::FORBIDDEN);

    // 2. Allowed Origin -> 200 OK
    let allowed = client
        .post(format!("{}/mcp", base_url))
        .header("Origin", "http://localhost:5173")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .unwrap();
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);

    // 3. No Origin header -> 200 OK
    let no_origin = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_body)
        .send()
        .await
        .unwrap();
    assert_eq!(no_origin.status(), reqwest::StatusCode::OK);

    server_handle.abort();
}

// ---------------------------------------------------------------------------
// m) Cancellation or ping messages
// Spec reference: MCP Lifecycle §Ping, §Cancellation
// Expected: ping returns 200 with result: {}; notifications/cancelled returns 202 Accepted.
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_edge_case_m_cancellation_and_ping() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    // 1. Ping message -> returns result: {}
    let ping_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 99,
            "method": "ping"
        }))
        .send()
        .await
        .expect("Ping request failed");

    assert_eq!(ping_resp.status(), reqwest::StatusCode::OK);
    let ping_body = ping_resp.text().await.unwrap();
    let ping_json = extract_json_from_response(&ping_body);
    assert_eq!(ping_json["jsonrpc"], "2.0");
    assert_eq!(ping_json["id"], 99);
    assert_eq!(ping_json["result"], serde_json::json!({}));

    // 2. Cancellation notification -> returns 202 Accepted
    let cancel_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/cancelled",
            "params": {
                "requestId": 99,
                "reason": "Client cancelled"
            }
        }))
        .send()
        .await
        .expect("Cancellation request failed");

    assert_eq!(cancel_resp.status(), reqwest::StatusCode::ACCEPTED);

    server_handle.abort();
}
