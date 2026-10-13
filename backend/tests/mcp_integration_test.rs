use reqwest::header::HeaderName;
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
    // If not SSE, try parsing as direct JSON
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
        ..Config::default()
    };

    let router = build_router(&config);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Small delay to ensure the server starts accepting
    tokio::time::sleep(Duration::from_millis(50)).await;

    (base_url, handle)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Health request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body: Value = resp.json().await.expect("Failed to parse health JSON");
    assert_eq!(body["status"], "ok");

    server_handle.abort();
}

#[tokio::test]
async fn test_disallowed_origin_rejected_with_403() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let init_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "attacker-client",
                "version": "1.0.0"
            }
        }
    });

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Origin", "http://evil-attacker.example.com")
        .header("Accept", "application/json, text/event-stream")
        .json(&init_payload)
        .send()
        .await
        .expect("Failed to send request with disallowed Origin");

    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);

    server_handle.abort();
}

#[tokio::test]
async fn test_unsupported_protocol_version_header_rejected_with_400() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("mcp-protocol-version", "1999-01-01")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        }))
        .send()
        .await
        .expect("Failed to send request with unsupported protocol version");

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_REQUEST);

    server_handle.abort();
}

#[tokio::test]
async fn test_full_mcp_lifecycle_and_tools() {
    let (base_url, server_handle) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. Initialize with protocolVersion "2025-11-25"
    let init_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test-client",
                "version": "1.0.0"
            }
        }
    });

    let init_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .json(&init_payload)
        .send()
        .await
        .expect("Initialize request failed");

    assert_eq!(init_resp.status(), reqwest::StatusCode::OK);

    // Extract session id header
    let session_header_name = HeaderName::from_static("mcp-session-id");
    let session_id = init_resp
        .headers()
        .get(&session_header_name)
        .expect("Expected mcp-session-id header in initialize response")
        .to_str()
        .expect("Invalid session id header string")
        .to_string();

    assert!(!session_id.is_empty(), "Session ID must not be empty");

    let init_raw_body = init_resp
        .text()
        .await
        .expect("Failed to read initialize body");
    let init_json = extract_json_from_response(&init_raw_body);

    assert_eq!(init_json["jsonrpc"], "2.0");
    assert_eq!(init_json["id"], 1);

    let result = &init_json["result"];
    assert_eq!(result["protocolVersion"], "2025-11-25");
    assert_eq!(result["serverInfo"]["name"], "tabletalk");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(result["capabilities"]["tools"].is_object());

    // 2. Client sends notifications/initialized
    let notif_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    let notif_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&notif_payload)
        .send()
        .await
        .expect("notifications/initialized request failed");

    assert_eq!(notif_resp.status(), reqwest::StatusCode::ACCEPTED);

    // 3. tools/list returns ping_restaurant with input and output schemas
    let list_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list"
    });

    let list_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&list_payload)
        .send()
        .await
        .expect("tools/list request failed");

    assert_eq!(list_resp.status(), reqwest::StatusCode::OK);
    let list_raw_body = list_resp
        .text()
        .await
        .expect("Failed to read tools/list body");
    let list_json = extract_json_from_response(&list_raw_body);

    assert_eq!(list_json["jsonrpc"], "2.0");
    assert_eq!(list_json["id"], 2);

    let tools = list_json["result"]["tools"]
        .as_array()
        .expect("tools should be an array");
    let ping_tool = tools
        .iter()
        .find(|t| t["name"] == "ping_restaurant")
        .expect("ping_restaurant tool must exist in tools/list");

    assert!(
        ping_tool["description"].is_string(),
        "Tool description must be present"
    );
    assert!(
        ping_tool["inputSchema"].is_object(),
        "inputSchema must be an object"
    );
    assert!(
        ping_tool["outputSchema"].is_object(),
        "outputSchema must be an object"
    );

    // Verify Draft 2020-12 schema dialect
    assert_eq!(
        ping_tool["inputSchema"]["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        ping_tool["outputSchema"]["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );

    // 4. tools/call ping_restaurant returns expected structured content
    let call_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "ping_restaurant",
            "arguments": {
                "name": "Guest Chef"
            }
        }
    });

    let call_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&call_payload)
        .send()
        .await
        .expect("tools/call request failed");

    assert_eq!(call_resp.status(), reqwest::StatusCode::OK);
    let call_raw_body = call_resp
        .text()
        .await
        .expect("Failed to read tools/call body");
    let call_json = extract_json_from_response(&call_raw_body);

    assert_eq!(call_json["jsonrpc"], "2.0");
    assert_eq!(call_json["id"], 3);

    let tool_result = &call_json["result"];
    assert_eq!(tool_result["isError"], false);

    // Check structured content or content text
    let content_arr = tool_result["content"].as_array().expect("content array");
    let text_block = &content_arr[0]["text"];
    assert!(text_block
        .as_str()
        .unwrap()
        .contains("Hello, Guest Chef! TableTalk restaurant MCP server is online."));

    // Check structuredContent if present
    if let Some(sc) = tool_result.get("structuredContent") {
        assert_eq!(sc["server"], "tabletalk");
        assert_eq!(sc["version"], env!("CARGO_PKG_VERSION"));
    }

    // 5. tools/call with unknown tool name returns proper error
    let unknown_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "non_existent_tool",
            "arguments": {}
        }
    });

    let unknown_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&unknown_payload)
        .send()
        .await
        .expect("unknown tools/call request failed");

    assert_eq!(unknown_resp.status(), reqwest::StatusCode::OK);
    let unknown_raw_body = unknown_resp
        .text()
        .await
        .expect("Failed to read unknown tool body");
    let unknown_json = extract_json_from_response(&unknown_raw_body);

    // Must return an error or result with isError: true
    assert!(
        unknown_json.get("error").is_some() || unknown_json["result"]["isError"] == true,
        "Unknown tool call must return an error response, got: {:?}",
        unknown_json
    );

    server_handle.abort();
}
