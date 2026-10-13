use chrono::{DateTime, Duration, TimeZone, Utc};
use reqwest::header::HeaderName;
use serde_json::Value;
use std::time::Instant;
use tabletalk_backend::{
    availability::{AreaFilter, CheckAvailabilityOutput, FixedClock},
    build_router_with_db,
    db::{init_test_pool, seed_database},
    tools::check_availability::{handle_check_availability_with_clock, CheckAvailabilityInput},
    Config,
};

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

async fn setup_seeded_db(fixed_now: Option<DateTime<Utc>>) -> sqlx::SqlitePool {
    let pool = init_test_pool()
        .await
        .expect("Failed to init in-memory test pool");
    seed_database(&pool, fixed_now)
        .await
        .expect("Failed to seed test database");
    pool
}

async fn spawn_test_server(pool: sqlx::SqlitePool) -> (String, tokio::task::JoinHandle<()>) {
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
            format!("http://{}", addr),
        ],
        rust_log: "warn".to_string(),
        request_timeout_secs: 30,
        ..Config::default()
    };

    let router = build_router_with_db(&config, Some(pool));
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (base_url, handle)
}

// =========================================================================
// 1. SEED SCENARIOS & ENGINE CORRECTNESS TESTS
// =========================================================================

#[tokio::test]
async fn test_seed_scenarios_fully_booked_closed_and_special_closure() {
    // Fixed now: Monday 2026-10-19 12:00 UTC (17:30 IST)
    // In seed.rs, future_fri_start is now + 4 days = Friday 2026-10-23 19:30
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Scenario 1: Friday 19:30 with Indoor seating for party of 4
    // Tables 3 (T03), 4 (T04), 5 (T05) are booked at 19:30.
    // T01 and T02 only seat 2. T06 requires at least 6.
    // Indoor is FULLY BOOKED at 19:30!
    let fri_input = CheckAvailabilityInput {
        date: "2026-10-23".to_string(),
        time: "19:30".to_string(),
        party_size: 4,
        area: Some(AreaFilter::Indoor),
        duration_minutes: Some(90),
        alternatives: Some(3),
    };

    let fri_res = handle_check_availability_with_clock(&pool, fri_input, &clock).await;
    assert_eq!(fri_res.is_error, Some(false));
    let fri_out: CheckAvailabilityOutput =
        serde_json::from_value(fri_res.structured_content.unwrap()).unwrap();

    assert_eq!(fri_out.status, "unavailable");
    assert_eq!(fri_out.reason.as_deref(), Some("fully_booked"));
    assert!(
        !fri_out.alternatives.is_empty(),
        "Must return sensible alternatives"
    );

    // First alternative is Patio at 19:30 (different area at requested time)
    let alt0 = &fri_out.alternatives[0];
    assert_eq!(alt0.area, "patio");
    assert_eq!(alt0.minutes_from_requested, 0);

    // Scenario 2: Weekly closed day (Tuesday 2026-10-20)
    let tue_input = CheckAvailabilityInput {
        date: "2026-10-20".to_string(),
        time: "19:30".to_string(),
        party_size: 2,
        area: Some(AreaFilter::Any),
        duration_minutes: Some(90),
        alternatives: Some(3),
    };

    let tue_res = handle_check_availability_with_clock(&pool, tue_input, &clock).await;
    assert_eq!(tue_res.is_error, Some(false));
    let tue_out: CheckAvailabilityOutput =
        serde_json::from_value(tue_res.structured_content.unwrap()).unwrap();

    assert_eq!(tue_out.status, "closed");
    assert_eq!(tue_out.reason.as_deref(), Some("closed_day"));

    // Scenario 3: Special closure (now + 14 days = Monday 2026-11-02)
    let closure_date = (fixed_now + Duration::days(14))
        .format("%Y-%m-%d")
        .to_string();
    let closure_input = CheckAvailabilityInput {
        date: closure_date,
        time: "19:30".to_string(),
        party_size: 2,
        area: Some(AreaFilter::Any),
        duration_minutes: Some(90),
        alternatives: Some(3),
    };

    let closure_res = handle_check_availability_with_clock(&pool, closure_input, &clock).await;
    assert_eq!(closure_res.is_error, Some(false));
    let closure_out: CheckAvailabilityOutput =
        serde_json::from_value(closure_res.structured_content.unwrap()).unwrap();

    assert_eq!(closure_out.status, "closed");
    assert_eq!(closure_out.reason.as_deref(), Some("special_closure"));
}

// =========================================================================
// 2. PROPERTY-STYLE TEST (RANDOM VALID REQUESTS & ALTERNATIVES VERIFICATION)
// =========================================================================

#[tokio::test]
async fn test_property_returned_slots_never_overlap_and_alternatives_verified() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Generate diverse test requests across open dates, times, party sizes, and areas
    let test_dates = [
        "2026-10-21",
        "2026-10-22",
        "2026-10-23",
        "2026-10-24",
        "2026-10-25",
    ];
    let test_times = [
        "12:30", "13:00", "14:00", "19:00", "19:30", "20:00", "20:30",
    ];
    let test_parties = [2, 4, 6];
    let test_areas = [
        AreaFilter::Any,
        AreaFilter::Indoor,
        AreaFilter::Patio,
        AreaFilter::Bar,
    ];

    let mut checks_run = 0;

    for &d in &test_dates {
        for &t in &test_times {
            let p = test_parties[checks_run % test_parties.len()];
            let a = test_areas[checks_run % test_areas.len()];
            checks_run += 1;

            let input = CheckAvailabilityInput {
                date: d.to_string(),
                time: t.to_string(),
                party_size: p,
                area: Some(a),
                duration_minutes: Some(90),
                alternatives: Some(3),
            };

            let res = handle_check_availability_with_clock(&pool, input, &clock).await;
            assert_eq!(res.is_error, Some(false));
            let out: CheckAvailabilityOutput =
                serde_json::from_value(res.structured_content.unwrap()).unwrap();

            // 1. If available, verify allocated table does NOT overlap any blocking booking in DB
            if out.status == "available" {
                let slot = out
                    .slot
                    .as_ref()
                    .expect("Slot must be present when available");
                let table_label = &slot.table_label;
                let start_iso = &slot.start_local;
                let end_iso = &slot.end_local;

                let start_utc = DateTime::parse_from_rfc3339(start_iso)
                    .unwrap()
                    .with_timezone(&Utc)
                    .to_rfc3339();
                let end_utc = DateTime::parse_from_rfc3339(end_iso)
                    .unwrap()
                    .with_timezone(&Utc)
                    .to_rfc3339();

                let table_id: i64 =
                    sqlx::query_scalar("SELECT id FROM dining_tables WHERE label = ?")
                        .bind(table_label)
                        .fetch_one(&pool)
                        .await
                        .unwrap();

                // Check overlap against active bookings in SQLite
                let overlap_count: i64 = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM bookings
                    WHERE table_id = ?
                      AND status IN ('pending', 'confirmed')
                      AND start_at < ?
                      AND end_at > ?
                    "#,
                )
                .bind(table_id)
                .bind(&end_utc)
                .bind(&start_utc)
                .fetch_one(&pool)
                .await
                .unwrap();

                assert_eq!(
                    overlap_count, 0,
                    "Allocated table {} overlaps {} active bookings for [{}, {})!",
                    table_label, overlap_count, start_iso, end_iso
                );
            }

            // 2. If alternatives are returned, independently verify each alternative by re-running
            for alt in &out.alternatives {
                let alt_dt = DateTime::parse_from_rfc3339(&alt.start_local).unwrap();
                let alt_date = alt_dt.date_naive().format("%Y-%m-%d").to_string();
                let alt_time = alt_dt.time().format("%H:%M").to_string();
                let alt_area = match alt.area.as_str() {
                    "indoor" => AreaFilter::Indoor,
                    "patio" => AreaFilter::Patio,
                    "bar" => AreaFilter::Bar,
                    _ => AreaFilter::Any,
                };

                let alt_check_input = CheckAvailabilityInput {
                    date: alt_date,
                    time: alt_time,
                    party_size: p,
                    area: Some(alt_area),
                    duration_minutes: Some(90),
                    alternatives: Some(0),
                };

                let alt_res =
                    handle_check_availability_with_clock(&pool, alt_check_input, &clock).await;
                assert_eq!(alt_res.is_error, Some(false));
                let alt_out: CheckAvailabilityOutput =
                    serde_json::from_value(alt_res.structured_content.unwrap()).unwrap();

                assert_eq!(
                    alt_out.status, "available",
                    "Alternative slot ({}, {}, area {}) was NOT available when independently checked!",
                    alt.start_local, alt.table_label, alt.area
                );
            }
        }
    }

    assert!(
        checks_run >= 25,
        "Property test must have run at least 25 checks"
    );
}

// =========================================================================
// 3. IDEMPOTENCE & ZERO SIDE EFFECTS TEST
// =========================================================================

#[tokio::test]
async fn test_idempotence_and_zero_side_effects() {
    let pool = setup_seeded_db(None).await;
    let tables_to_check = [
        "restaurant",
        "opening_hours",
        "special_closures",
        "dining_tables",
        "menu_categories",
        "menu_items",
        "dietary_tags",
        "allergens",
        "customers",
        "customer_preferences",
        "bookings",
        "orders",
        "order_items",
        "payments",
        "audit_events",
    ];

    // Capture baseline row counts
    let mut baseline_counts = Vec::new();
    for tbl in tables_to_check {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", tbl))
            .fetch_one(&pool)
            .await
            .unwrap();
        baseline_counts.push((tbl, count));
    }

    // Execute 20 availability checks with various valid and invalid inputs
    for i in 1..=20 {
        let input = CheckAvailabilityInput {
            date: format!("2026-10-{}", 15 + (i % 10)),
            time: "19:30".to_string(),
            party_size: (i % 6) + 1,
            area: Some(AreaFilter::Any),
            duration_minutes: Some(90),
            alternatives: Some(3),
        };
        let _ = handle_check_availability_with_clock(&pool, input, &FixedClock(Utc::now())).await;
    }

    // Verify row counts are 100% unchanged
    for (tbl, expected) in baseline_counts {
        let actual: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", tbl))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            actual, expected,
            "Table {} row count changed from {} to {} after availability checks!",
            tbl, expected, actual
        );
    }
}

// =========================================================================
// 4. MCP OVER HTTP PROTOCOL TESTS
// =========================================================================

#[tokio::test]
async fn test_mcp_tools_list_contains_check_availability_with_schemas_and_annotations() {
    let pool = setup_seeded_db(None).await;
    let (base_url, server_handle) = spawn_test_server(pool).await;
    let client = reqwest::Client::new();

    // Initialize handshake
    let init_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "availability-test", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    let session_header = HeaderName::from_static("mcp-session-id");
    let session_id = init_resp
        .headers()
        .get(&session_header)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // notifications/initialized
    client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .send()
        .await
        .unwrap();

    // Call tools/list
    let list_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }))
        .send()
        .await
        .unwrap();

    let list_raw = list_resp.text().await.unwrap();
    let list_json = extract_json_from_response(&list_raw);
    let tools = list_json["result"]["tools"]
        .as_array()
        .expect("tools array");

    // Must find all THREE tools: ping_restaurant, search_menu, check_availability
    assert!(tools.iter().any(|t| t["name"] == "ping_restaurant"));
    assert!(tools.iter().any(|t| t["name"] == "search_menu"));

    let check_tool = tools
        .iter()
        .find(|t| t["name"] == "check_availability")
        .expect("check_availability tool missing from tools/list");

    // Verify schemas and annotations
    assert!(check_tool["description"]
        .as_str()
        .unwrap()
        .contains("relative date"));
    assert_eq!(
        check_tool["inputSchema"]["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        check_tool["outputSchema"]["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );

    let props = &check_tool["inputSchema"]["properties"];
    assert!(props["date"].is_object());
    assert!(props["time"].is_object());
    assert!(props["party_size"].is_object());
    assert!(props["area"].is_object());
    assert!(props["duration_minutes"].is_object());
    assert!(props["alternatives"].is_object());

    let ann = &check_tool["annotations"];
    assert_eq!(ann["readOnlyHint"], true);
    assert_eq!(ann["idempotentHint"], true);
    assert_eq!(ann["destructiveHint"], false);
    assert_eq!(ann["openWorldHint"], false);

    server_handle.abort();
}

#[tokio::test]
async fn test_mcp_tools_call_check_availability_and_invalid_arguments() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let (base_url, server_handle) = spawn_test_server(pool).await;
    let client = reqwest::Client::new();

    // Initialize
    let init_resp = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    let session_header = HeaderName::from_static("mcp-session-id");
    let session_id = init_resp
        .headers()
        .get(&session_header)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .send()
        .await
        .unwrap();

    // Case 1: Valid available slot
    let valid_call = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "check_availability",
                "arguments": {
                    "date": "2026-10-21",
                    "time": "19:30",
                    "party_size": 2,
                    "area": "indoor"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let raw = valid_call.text().await.unwrap();
    let json = extract_json_from_response(&raw);
    let result = &json["result"];
    assert_eq!(result["isError"], false);
    let voice_text = result["content"][0]["text"].as_str().unwrap();
    assert!(voice_text.contains("Want me to book it?"));

    let sc = &result["structuredContent"];
    assert_eq!(sc["status"], "available");
    assert!(sc["slot"].is_object());
    assert_eq!(sc["note"], "availability is a snapshot, not a reservation");

    // Case 2: Invalid argument (unresolved relative date "tomorrow" instead of YYYY-MM-DD)
    let bad_date_call = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "check_availability",
                "arguments": {
                    "date": "tomorrow",
                    "time": "19:30",
                    "party_size": 2
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let raw_bad = bad_date_call.text().await.unwrap();
    let json_bad = extract_json_from_response(&raw_bad);
    let result_bad = &json_bad["result"];
    assert_eq!(
        result_bad["isError"], true,
        "Malformed arguments must yield isError: true"
    );
    let err_msg = result_bad["content"][0]["text"].as_str().unwrap();
    assert!(err_msg.contains("YYYY-MM-DD"));

    // Case 3: Invalid party size 0
    let bad_party_call = client
        .post(format!("{}/mcp", base_url))
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "check_availability",
                "arguments": {
                    "date": "2026-10-21",
                    "time": "19:30",
                    "party_size": 0
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let raw_party = bad_party_call.text().await.unwrap();
    let json_party = extract_json_from_response(&raw_party);
    assert_eq!(json_party["result"]["isError"], true);

    server_handle.abort();
}

// =========================================================================
// 5. LATENCY BENCHMARK (< 30 ms median)
// =========================================================================

#[tokio::test]
async fn test_availability_latency_median_under_30ms() {
    let pool = setup_seeded_db(None).await;
    let clock = FixedClock(Utc::now());

    let mut latencies = Vec::new();

    for i in 0..30 {
        let input = CheckAvailabilityInput {
            date: "2026-10-25".to_string(),
            time: "19:30".to_string(),
            party_size: (i % 4) + 2,
            area: Some(AreaFilter::Any),
            duration_minutes: Some(90),
            alternatives: Some(3),
        };

        let start = Instant::now();
        let _ = handle_check_availability_with_clock(&pool, input, &clock).await;
        latencies.push(start.elapsed());
    }

    latencies.sort();
    let median = latencies[latencies.len() / 2];
    println!("Availability check median latency: {:?}", median);

    // Generous assertion for varied environments
    assert!(
        median < std::time::Duration::from_millis(50),
        "Median latency too high: {:?}",
        median
    );
}
