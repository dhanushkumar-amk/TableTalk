use serde_json::{json, Value};
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

async fn spawn_test_server(rate_limit: usize) -> (String, tokio::task::JoinHandle<()>) {
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
        rate_limit_per_minute: rate_limit,
        ..Config::default()
    };

    let router = build_router(&config);
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (base_url, handle)
}

async fn init_session(client: &reqwest::Client, base_url: &str) -> String {
    let init_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": { "name": "fuzz-tester", "version": "1.0.0" }
        }
    });

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .json(&init_payload)
        .send()
        .await
        .expect("initialize failed");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    let session_id = resp
        .headers()
        .get("mcp-session-id")
        .expect("session id missing")
        .to_str()
        .unwrap()
        .to_string();

    let notif_payload = json!({
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
        .expect("notifications/initialized failed");
    assert_eq!(notif_resp.status(), reqwest::StatusCode::ACCEPTED);

    session_id
}

#[tokio::test]
async fn test_rate_limiter_enforcement_and_health_unthrottled() {
    // Set a strict rate limit of 5 requests per minute
    let (base_url, server_handle) = spawn_test_server(5).await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    // Send 10 rapid requests to /mcp
    let mut rate_limited_count = 0;
    for i in 2..12 {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": i,
            "method": "tools/call",
            "params": {
                "name": "ping_restaurant",
                "arguments": { "name": format!("Guest-{}", i) }
            }
        });

        let resp = client
            .post(format!("{}/mcp", base_url))
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", &session_id)
            .header("mcp-protocol-version", "2025-11-25")
            .json(&payload)
            .send()
            .await
            .expect("request failed");

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            rate_limited_count += 1;
            // Ensure Retry-After header is present and valid
            let retry_after = resp.headers().get("retry-after");
            assert!(
                retry_after.is_some(),
                "Retry-After header missing on 429 response"
            );
        }
    }

    assert!(
        rate_limited_count > 0,
        "Expected at least some requests to be rate-limited under limit=5"
    );

    // Verify /health is NOT rate-limited and returns 200 OK
    for _ in 0..5 {
        let health_resp = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .expect("health check failed");
        assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_unknown_extra_fields_policy_ignored() {
    let (base_url, server_handle) = spawn_test_server(600).await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    // Call ping_restaurant with extra rogue fields
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/call",
        "params": {
            "name": "ping_restaurant",
            "arguments": {
                "name": "Alice",
                "extra_unexpected_field": "ignore_me_safely",
                "nested_garbage": { "foo": "bar" },
                "number_flag": 42
            }
        }
    });

    let resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&payload)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body = resp.text().await.unwrap();
    let json_val = extract_json_from_response(&body);
    let result = &json_val["result"];
    assert_eq!(result["isError"], false);
    assert!(result["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Alice"));

    server_handle.abort();
}

#[tokio::test]
async fn test_all_24_tools_fuzz_and_malformed_input_safety() {
    let (base_url, server_handle) = spawn_test_server(2000).await;
    let client = reqwest::Client::new();
    let session_id = init_session(&client, &base_url).await;

    let tools = vec![
        "ping_restaurant",
        "search_menu",
        "get_menu_item",
        "check_availability",
        "create_booking",
        "get_booking",
        "cancel_booking",
        "place_order",
        "get_order",
        "cancel_order",
        "enable_memory",
        "get_profile",
        "remember_preference",
        "forget_preference",
        "get_usual_order",
        "rotate_profile_token",
        "forget_me",
        "negotiate_booking",
        "accept_offer",
        "release_offer",
        "join_waitlist",
        "get_waitlist_status",
        "leave_waitlist",
        "get_deposit_status",
        "refresh_payment_session",
    ];

    let huge_string = "A".repeat(12000);
    let control_chars = "Test\x00\x01\x1b[31m\x08Value\r\n";
    let sql_injection = "'; DROP TABLE bookings; SELECT * FROM customers WHERE '1'='1";
    let xss_payload = "<script>alert('xss')</script><img src=x onerror=alert(1)>";

    let test_argument_sets = vec![
        // 1. Empty arguments
        json!({}),
        // 2. Null arguments for all fields
        json!({
            "name": null,
            "guest_name": null,
            "date": null,
            "time": null,
            "party_size": null,
            "items": null,
            "confirmation_code": null,
            "order_code": null,
            "profile_token": null,
            "offer_token": null,
            "waitlist_code": null
        }),
        // 3. Inverted/wrong types (integers for strings, strings for numbers, bool for array)
        json!({
            "name": 12345,
            "guest_name": false,
            "date": 20261014,
            "time": 1900,
            "party_size": "ten",
            "items": true,
            "confirmation_code": 999999,
            "order_code": 888888,
            "profile_token": 123.456,
            "offer_token": true
        }),
        // 4. Extreme numbers / negative values
        json!({
            "party_size": -5,
            "limit": -100,
            "max_price": -50.0,
            "price": -1000,
            "quantity": -2,
            "duration_minutes": -30
        }),
        // 5. Overflow bounds
        json!({
            "party_size": 999999999999_i64,
            "limit": 999999999999_i64,
            "duration_minutes": 999999999999_i64
        }),
        // 6. Huge string attacks (>10k chars)
        json!({
            "name": &huge_string,
            "guest_name": &huge_string,
            "special_requests": &huge_string,
            "notes": &huge_string,
            "reason": &huge_string,
            "profile_token": &huge_string,
            "confirmation_code": &huge_string
        }),
        // 7. Control characters & ANSI escape sequences
        json!({
            "name": control_chars,
            "guest_name": control_chars,
            "reason": control_chars,
            "notes": control_chars,
            "date": "2026\x00-10-14",
            "time": "19:00\x1b"
        }),
        // 8. SQL injection attempts
        json!({
            "guest_name": sql_injection,
            "query": sql_injection,
            "category": sql_injection,
            "confirmation_code": sql_injection,
            "order_code": sql_injection,
            "profile_token": sql_injection
        }),
        // 9. Markup and XSS injection
        json!({
            "guest_name": xss_payload,
            "special_requests": xss_payload,
            "query": xss_payload,
            "notes": xss_payload
        }),
        // 10. Invalid date and time formats
        json!({
            "date": "14-10-2026",
            "time": "7:00 PM",
            "party_size": 2,
            "guest_name": "Test"
        }),
        json!({
            "date": "2026/10/14",
            "time": "25:99",
            "party_size": 4,
            "guest_name": "Test"
        }),
    ];

    let mut call_id = 200;

    for tool_name in &tools {
        for args in &test_argument_sets {
            call_id += 1;
            let payload = json!({
                "jsonrpc": "2.0",
                "id": call_id,
                "method": "tools/call",
                "params": {
                    "name": tool_name,
                    "arguments": args
                }
            });

            let resp = client
                .post(format!("{}/mcp", base_url))
                .header("Accept", "application/json, text/event-stream")
                .header("mcp-session-id", &session_id)
                .header("mcp-protocol-version", "2025-11-25")
                .json(&payload)
                .send()
                .await
                .expect("fuzz request should not crash connection");

            assert_eq!(
                resp.status(),
                reqwest::StatusCode::OK,
                "Server must return 200 JSON-RPC response for tool '{}' even on bad input",
                tool_name
            );

            let body = resp.text().await.unwrap();
            let lower_body = body.to_lowercase();

            // Leak checks: Ensure no SQL internals or stack traces leak to client
            assert!(
                !lower_body.contains("sqlite3")
                    && !lower_body.contains("syntax error near")
                    && !lower_body.contains("stack backtrace")
                    && !lower_body.contains("tabletalk\\src")
                    && !lower_body.contains("tabletalk/src")
                    && !lower_body.contains("unique constraint failed"),
                "Tool '{}' leaked internal details in response: {}",
                tool_name,
                body
            );

            let json_val = extract_json_from_response(&body);
            // Must either be a valid JSON-RPC error or a tool result
            if let Some(error) = json_val.get("error") {
                assert!(error.get("code").is_some());
                assert!(error.get("message").is_some());
            } else if let Some(result) = json_val.get("result") {
                assert!(result.get("content").is_some());
            } else {
                panic!(
                    "Malformed JSON-RPC response from tool '{}': {}",
                    tool_name, body
                );
            }
        }
    }

    // Assert server is completely healthy and operational after extensive fuzzing
    let health_resp = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("health check must succeed after fuzzing");
    assert_eq!(health_resp.status(), reqwest::StatusCode::OK);

    server_handle.abort();
}

#[tokio::test]
async fn test_panic_handler_layer_graceful_recovery() {
    // 1. Verify handle_panic returns generic 500 without leaking the panic message
    let payload = Box::new("Simulated internal runtime panic in database worker");
    let response = tabletalk_backend::http::handle_panic(payload);
    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );

    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&bytes);

    assert!(
        body_str.contains("Internal Server Error"),
        "Must return generic internal server error"
    );
    assert!(
        !body_str.contains("Simulated internal runtime panic"),
        "Must never leak panic payload to client"
    );
}
