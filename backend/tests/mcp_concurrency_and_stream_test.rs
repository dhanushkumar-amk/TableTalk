use serde_json::Value;
use std::time::{Duration, Instant};
use tabletalk_backend::{build_router, Config};

async fn spawn_test_server(timeout_secs: u64) -> (String, tokio::task::JoinHandle<()>) {
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
        request_timeout_secs: timeout_secs,
        ..Config::default()
    };

    let router = build_router(&config);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (base_url, handle)
}

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

#[tokio::test]
async fn test_sse_stream_open_and_client_disconnect() {
    let (base_url, server_handle) = spawn_test_server(30).await;
    let client = reqwest::Client::new();

    // 1. Initialize session
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
                "clientInfo": {"name": "stream-client", "version": "1.0"}
            }
        }))
        .send()
        .await
        .unwrap();

    let session_id = init_resp
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 2. Open GET /mcp SSE stream
    let sse_resp = client
        .get(format!("{}/mcp", base_url))
        .header("Accept", "text/event-stream")
        .header("mcp-session-id", &session_id)
        .send()
        .await
        .expect("Failed to connect to SSE stream");

    assert_eq!(sse_resp.status(), reqwest::StatusCode::OK);
    assert_eq!(
        sse_resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap(),
        "text/event-stream"
    );

    // 3. Client abruptly disconnects mid-stream by dropping the response stream
    drop(sse_resp);

    // 4. Server must remain fully operational without panicking or task leaks
    let health_resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Server must remain healthy after stream disconnect");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);

    // 5. Subsequent tool call on the same session still works
    let tool_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "ping_restaurant",
                "arguments": {"name": "AfterDisconnect"}
            }
        }))
        .send()
        .await
        .expect("Tool call after stream disconnect must succeed");

    assert_eq!(tool_resp.status(), reqwest::StatusCode::OK);
    let body = tool_resp.text().await.unwrap();
    let json = extract_json_from_response(&body);
    assert_eq!(json["result"]["isError"], false);

    server_handle.abort();
}

#[tokio::test]
async fn test_50_concurrent_sessions_and_latency() {
    let (base_url, server_handle) = spawn_test_server(30).await;
    let concurrency = 50;

    let mut handles = Vec::with_capacity(concurrency);

    let overall_start = Instant::now();

    for i in 0..concurrency {
        let url = base_url.clone();
        handles.push(tokio::spawn(async move {
            let client = reqwest::Client::new();
            let start = Instant::now();

            // Step 1: initialize
            let init_resp = client
                .post(format!("{}/mcp", url))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": {},
                        "clientInfo": {
                            "name": format!("concurrent-bot-{}", i),
                            "version": "1.0.0"
                        }
                    }
                }))
                .send()
                .await
                .expect("Concurrent initialize failed");

            assert_eq!(init_resp.status(), reqwest::StatusCode::OK);
            let session_id = init_resp
                .headers()
                .get("mcp-session-id")
                .expect("mcp-session-id header missing")
                .to_str()
                .unwrap()
                .to_string();

            // Step 2: notifications/initialized
            let notif_resp = client
                .post(format!("{}/mcp", url))
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
                .expect("Concurrent initialized notification failed");

            assert_eq!(notif_resp.status(), reqwest::StatusCode::ACCEPTED);

            // Step 3: tools/call ping_restaurant
            let call_resp = client
                .post(format!("{}/mcp", url))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json, text/event-stream")
                .header("mcp-session-id", &session_id)
                .header("mcp-protocol-version", "2025-11-25")
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {
                        "name": "ping_restaurant",
                        "arguments": {
                            "name": format!("Guest-{}", i)
                        }
                    }
                }))
                .send()
                .await
                .expect("Concurrent tools/call failed");

            assert_eq!(call_resp.status(), reqwest::StatusCode::OK);
            let body = call_resp.text().await.unwrap();
            let json = extract_json_from_response(&body);
            assert_eq!(json["result"]["isError"], false);

            start.elapsed()
        }));
    }

    let mut latencies: Vec<Duration> = Vec::with_capacity(concurrency);
    for handle in handles {
        let latency = handle.await.expect("Task panicked during concurrent load");
        latencies.push(latency);
    }

    let total_elapsed = overall_start.elapsed();
    latencies.sort();

    let p50 = latencies[concurrency * 50 / 100];
    let p95 = latencies[concurrency * 95 / 100];
    let max = latencies[concurrency - 1];

    println!("\n--- CONCURRENCY LOAD TEST RESULTS (50 Concurrent Sessions) ---");
    println!("Total Completed Sessions: {}", concurrency);
    println!("Total Elapsed Time:       {:.2?}", total_elapsed);
    println!("p50 Latency:              {:.2?}", p50);
    println!("p95 Latency:              {:.2?}", p95);
    println!("Max Latency:              {:.2?}", max);
    println!("-------------------------------------------------------------\n");

    // All sessions must succeed within reasonable bounds
    assert!(p95 < Duration::from_secs(2));

    server_handle.abort();
}

#[tokio::test]
async fn test_request_timeout_cutoff() {
    // Spawn server with a tight 1-second timeout
    let (base_url, server_handle) = spawn_test_server(1).await;
    let client = reqwest::Client::new();

    // 1. Send request with simulated 1500ms delay on 1s timeout server
    let timeout_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("x-test-simulate-delay-ms", "1500")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "timeout-client", "version": "1.0"}
            }
        }))
        .send()
        .await
        .expect("Request failed to complete");

    // TimeoutLayer cuts off hung request and returns HTTP 408 Request Timeout
    assert_eq!(timeout_resp.status(), reqwest::StatusCode::REQUEST_TIMEOUT);

    // 2. Verify server is still healthy and operational after timing out
    let health = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), reqwest::StatusCode::OK);

    server_handle.abort();
}
