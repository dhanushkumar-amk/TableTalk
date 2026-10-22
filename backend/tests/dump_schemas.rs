use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tabletalk_backend::{
    db::{init_test_pool, seed_database},
    Config,
};

fn parse_mcp_json_or_sse(raw: &str) -> Value {
    for line in raw.lines() {
        if let Some(json_text) = line.strip_prefix("data: ") {
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

async fn setup_seeded_db(fixed_now: Option<DateTime<Utc>>) -> SqlitePool {
    let pool = init_test_pool()
        .await
        .expect("Failed to init in-memory test pool");
    seed_database(&pool, fixed_now)
        .await
        .expect("Failed to seed test database");
    pool
}

async fn spawn_test_server(pool: SqlitePool) -> (String, tokio::task::JoinHandle<()>) {
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

    let router = tabletalk_backend::build_router_with_db(&config, Some(pool));
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (base_url, handle)
}

struct TestClient {
    client: reqwest::Client,
    base_url: String,
    session_id: String,
    request_id: i64,
}

impl TestClient {
    async fn new(base_url: String) -> Self {
        let client = reqwest::Client::new();
        let init_res = client
            .post(format!("{}/mcp", base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", "2025-11-25")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "fixture-generator-client",
                        "version": "1.0.0"
                    }
                }
            }))
            .send()
            .await
            .unwrap();

        let session_id = init_res
            .headers()
            .get("mcp-session-id")
            .expect("Session ID header missing")
            .to_str()
            .unwrap()
            .to_string();

        let notif = client
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
            .unwrap();
        assert_eq!(notif.status(), reqwest::StatusCode::ACCEPTED);

        Self {
            client,
            base_url,
            session_id,
            request_id: 1,
        }
    }

    async fn list_tools(&mut self) -> Value {
        self.request_id += 1;
        let res = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", &self.session_id)
            .header("mcp-protocol-version", "2025-11-25")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": "tools/list"
            }))
            .send()
            .await
            .unwrap();

        let raw = res.text().await.unwrap();
        let parsed = parse_mcp_json_or_sse(&raw);
        parsed["result"]["tools"].clone()
    }

    async fn call_tool(&mut self, tool: &str, arguments: Value) -> Value {
        self.request_id += 1;
        let res = self
            .client
            .post(format!("{}/mcp", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .header("mcp-session-id", &self.session_id)
            .header("mcp-protocol-version", "2025-11-25")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": self.request_id,
                "method": "tools/call",
                "params": {
                    "name": tool,
                    "arguments": arguments
                }
            }))
            .send()
            .await
            .unwrap();

        let raw = res.text().await.unwrap();
        let parsed = parse_mcp_json_or_sse(&raw);
        parsed["result"].clone()
    }
}

#[tokio::test]
async fn test_dump_schemas_and_generate_fixtures() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let (base_url, _handle) = spawn_test_server(pool.clone()).await;

    let mut client = TestClient::new(base_url.clone()).await;

    // 1. List tools and extract output schemas
    let tools = client.list_tools().await;
    let tools_arr = tools.as_array().expect("Tools must be an array");
    assert_eq!(tools_arr.len(), 24, "Must have exactly 24 tools");

    let schemas_dir = Path::new("../frontend/src/cards/schemas");
    let fixtures_dir = Path::new("../frontend/src/cards/fixtures");
    fs::create_dir_all(schemas_dir).unwrap();
    fs::create_dir_all(fixtures_dir).unwrap();

    // Save full tools list
    fs::write(
        schemas_dir.join("tools_list.json"),
        serde_json::to_string_pretty(&tools).unwrap(),
    )
    .unwrap();

    // Map tool_name -> outputSchema
    let mut output_schemas = serde_json::Map::new();
    for t in tools_arr {
        let name = t["name"].as_str().unwrap().to_string();
        let output_schema = t["outputSchema"].clone();
        output_schemas.insert(name, output_schema);
    }

    fs::write(
        schemas_dir.join("output_schemas.json"),
        serde_json::to_string_pretty(&Value::Object(output_schemas)).unwrap(),
    )
    .unwrap();

    println!("Saved output schemas for 24 tools.");

    // 2. Generate Real Fixtures by calling tools
    let fixtures: Vec<(&str, &str, Value)> = vec![
        // search_menu
        (
            "search_menu_found.json",
            "search_menu",
            serde_json::json!({ "query": "biryani" }),
        ),
        (
            "search_menu_empty.json",
            "search_menu",
            serde_json::json!({ "query": "nonexistent_dish_xyz_99" }),
        ),
        // check_availability
        (
            "check_availability_available.json",
            "check_availability",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "19:30",
                "party_size": 2,
                "area": "indoor"
            }),
        ),
        (
            "check_availability_fully_booked.json",
            "check_availability",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "19:30",
                "party_size": 8,
                "area": "indoor"
            }),
        ),
        (
            "check_availability_closed.json",
            "check_availability",
            serde_json::json!({
                "date": "2026-10-20",
                "time": "19:30",
                "party_size": 2
            }),
        ),
        // create_booking (standard confirmed booking)
        (
            "create_booking_confirmed.json",
            "create_booking",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "20:00",
                "party_size": 2,
                "guest_name": "Aarav Sharma",
                "area": "patio"
            }),
        ),
        // get_booking (existing seeded booking TT-10001)
        (
            "get_booking_found.json",
            "get_booking",
            serde_json::json!({
                "confirmation_code": "TT-10001",
                "guest_name": "Priya Patel"
            }),
        ),
        (
            "get_booking_not_found.json",
            "get_booking",
            serde_json::json!({
                "confirmation_code": "TT-99999",
                "guest_name": "Unknown Guest"
            }),
        ),
        // cancel_booking
        (
            "cancel_booking_cancelled.json",
            "cancel_booking",
            serde_json::json!({
                "confirmation_code": "TT-10001",
                "guest_name": "Priya Patel"
            }),
        ),
        // negotiate_booking
        (
            "negotiate_booking_options.json",
            "negotiate_booking",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "19:30",
                "party_size": 2,
                "guest_name": "Rohan Gupta"
            }),
        ),
        (
            "negotiate_booking_no_options.json",
            "negotiate_booking",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "19:30",
                "party_size": 12,
                "guest_name": "Huge Group"
            }),
        ),
        // place_order
        (
            "place_order_placed.json",
            "place_order",
            serde_json::json!({
                "guest_name": "Meera Rao",
                "pickup_time": "19:30",
                "items": [
                    { "item_id": 1, "quantity": 2 }
                ]
            }),
        ),
        (
            "place_order_allergen_conflict.json",
            "place_order",
            serde_json::json!({
                "guest_name": "Meera Rao",
                "pickup_time": "19:30",
                "items": [
                    { "item_id": 1, "quantity": 1 }
                ],
                "allergen_acknowledgement": {
                    "allergens_to_avoid": ["dairy"],
                    "acknowledged": false
                }
            }),
        ),
        // get_order (seeded order ORD-10001)
        (
            "get_order_found.json",
            "get_order",
            serde_json::json!({
                "order_code": "ORD-10001",
                "guest_name": "Vikram Singh"
            }),
        ),
        // cancel_order
        (
            "cancel_order_cancelled.json",
            "cancel_order",
            serde_json::json!({
                "order_code": "ORD-10001",
                "guest_name": "Vikram Singh"
            }),
        ),
        // ping
        (
            "ping_restaurant.json",
            "ping_restaurant",
            serde_json::json!({}),
        ),
    ];

    for (file_name, tool_name, args) in fixtures {
        let result = client.call_tool(tool_name, args).await;
        assert!(
            result.is_object(),
            "Tool {} result must be an object",
            tool_name
        );
        if file_name == "create_booking_confirmed.json"
            && result["structuredContent"]["status"] != "confirmed"
        {
            println!(
                "Skipping overwriting {} because status is not confirmed",
                file_name
            );
            continue;
        }
        let path = fixtures_dir.join(file_name);
        fs::write(&path, serde_json::to_string_pretty(&result).unwrap()).unwrap();
        println!("Generated fixture: {}", file_name);
    }

    // Create a large booking that requires a deposit (party_size = 6)
    let deposit_booking = client
        .call_tool(
            "create_booking",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "19:30",
                "party_size": 6,
                "guest_name": "Simulated Deposit Guest",
                "area": "indoor"
            }),
        )
        .await;

    fs::write(
        fixtures_dir.join("create_booking_deposit_required.json"),
        serde_json::to_string_pretty(&deposit_booking).unwrap(),
    )
    .unwrap();

    let conf_code = deposit_booking["structuredContent"]["booking"]["confirmation_code"]
        .as_str()
        .unwrap_or("TT-DEP01");

    // Deposit status query
    let dep_status = client
        .call_tool(
            "get_deposit_status",
            serde_json::json!({
                "confirmation_code": conf_code,
                "guest_name": "Simulated Deposit Guest"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("get_deposit_status_open.json"),
        serde_json::to_string_pretty(&dep_status).unwrap(),
    )
    .unwrap();

    // Refresh payment session
    let refresh_pay = client
        .call_tool(
            "refresh_payment_session",
            serde_json::json!({
                "confirmation_code": conf_code,
                "guest_name": "Simulated Deposit Guest"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("refresh_payment_session_refreshed.json"),
        serde_json::to_string_pretty(&refresh_pay).unwrap(),
    )
    .unwrap();

    // Negotiation and Accept Offer
    let neg_res = client
        .call_tool(
            "negotiate_booking",
            serde_json::json!({
                "date": "2026-10-16",
                "time": "20:30",
                "party_size": 2,
                "guest_name": "HoldTester"
            }),
        )
        .await;

    if let Some(options) = neg_res["structuredContent"]["options"].as_array() {
        if !options.is_empty() {
            let opt_id = options[0]["option_id"].as_str().unwrap();
            let neg_id = neg_res["structuredContent"]["negotiation_id"]
                .as_i64()
                .unwrap();

            // Place a hold
            let hold_res = client
                .call_tool(
                    "accept_offer",
                    serde_json::json!({
                        "negotiation_id": neg_id,
                        "option_id": opt_id,
                        "guest_name": "HoldTester",
                        "hold_only": true
                    }),
                )
                .await;
            fs::write(
                fixtures_dir.join("accept_offer_hold.json"),
                serde_json::to_string_pretty(&hold_res).unwrap(),
            )
            .unwrap();

            let offer_token = hold_res["structuredContent"]["offer_token"]
                .as_str()
                .unwrap_or("");

            // Release offer
            if !offer_token.is_empty() {
                let rel_res = client
                    .call_tool(
                        "release_offer",
                        serde_json::json!({
                            "offer_token": offer_token,
                            "guest_name": "HoldTester"
                        }),
                    )
                    .await;
                fs::write(
                    fixtures_dir.join("release_offer_released.json"),
                    serde_json::to_string_pretty(&rel_res).unwrap(),
                )
                .unwrap();
            }

            // Confirm another option
            let accept_res = client
                .call_tool(
                    "accept_offer",
                    serde_json::json!({
                        "negotiation_id": neg_id,
                        "option_id": opt_id,
                        "guest_name": "HoldTester",
                        "hold_only": false
                    }),
                )
                .await;
            fs::write(
                fixtures_dir.join("accept_offer_confirmed.json"),
                serde_json::to_string_pretty(&accept_res).unwrap(),
            )
            .unwrap();
        }
    }

    // Join waitlist
    let wl_join = client
        .call_tool(
            "join_waitlist",
            serde_json::json!({
                "guest_name": "Ananya Desai",
                "party_size": 4,
                "date": "2026-10-16",
                "window_start": "19:00",
                "window_end": "21:00",
                "area": "indoor"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("join_waitlist_joined.json"),
        serde_json::to_string_pretty(&wl_join).unwrap(),
    )
    .unwrap();

    let wl_code = wl_join["structuredContent"]["code"]
        .as_str()
        .unwrap_or("WL-10001");

    let wl_status = client
        .call_tool(
            "get_waitlist_status",
            serde_json::json!({
                "code": wl_code,
                "guest_name": "Ananya Desai"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("get_waitlist_status_waiting.json"),
        serde_json::to_string_pretty(&wl_status).unwrap(),
    )
    .unwrap();

    let wl_leave = client
        .call_tool(
            "leave_waitlist",
            serde_json::json!({
                "code": wl_code,
                "guest_name": "Ananya Desai"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("leave_waitlist_left.json"),
        serde_json::to_string_pretty(&wl_leave).unwrap(),
    )
    .unwrap();

    // Memory tools
    let mem_enable = client
        .call_tool(
            "enable_memory",
            serde_json::json!({
                "guest_name": "MemoryGuest",
                "consent_acknowledged": true
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("enable_memory_enabled.json"),
        serde_json::to_string_pretty(&mem_enable).unwrap(),
    )
    .unwrap();

    let profile_token = mem_enable["structuredContent"]["profile_token"]
        .as_str()
        .unwrap()
        .to_string();

    let rem_res = client
        .call_tool(
            "remember_preference",
            serde_json::json!({
                "profile_token": profile_token,
                "kind": "allergy",
                "value": "peanuts"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("remember_preference_remembered.json"),
        serde_json::to_string_pretty(&rem_res).unwrap(),
    )
    .unwrap();

    // Add note preference with potential script injection attempt to test inert rendering
    let _rem_note = client
        .call_tool(
            "remember_preference",
            serde_json::json!({
                "profile_token": profile_token,
                "kind": "note",
                "value": "Prefers booth near window, celebrate anniversary <script>alert('xss')</script>"
            }),
        )
        .await;

    let prof_res = client
        .call_tool(
            "get_profile",
            serde_json::json!({
                "profile_token": profile_token
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("get_profile_found.json"),
        serde_json::to_string_pretty(&prof_res).unwrap(),
    )
    .unwrap();

    // Query seeded paid deposit: TT-9203 for Vikramaditya Rao
    let dep_paid = client
        .call_tool(
            "get_deposit_status",
            serde_json::json!({
                "confirmation_code": "TT-9203",
                "guest_name": "Vikramaditya Rao"
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("get_deposit_status_paid.json"),
        serde_json::to_string_pretty(&dep_paid).unwrap(),
    )
    .unwrap();

    // Enable memory for Priya Patel who has seeded orders
    let mem_priya = client
        .call_tool(
            "enable_memory",
            serde_json::json!({
                "guest_name": "Priya Patel",
                "consent_acknowledged": true
            }),
        )
        .await;
    // Place order with Priya's profile token so get_usual_order returns found
    if let Some(token) = mem_priya["structuredContent"]["profile_token"].as_str() {
        let _placed = client
            .call_tool(
                "place_order",
                serde_json::json!({
                    "guest_name": "Priya Patel",
                    "profile_token": token,
                    "pickup_time": "19:30",
                    "items": [
                        { "item_id": 1, "quantity": 1 },
                        { "item_id": 13, "quantity": 2 }
                    ]
                }),
            )
            .await;

        let usual_priya = client
            .call_tool(
                "get_usual_order",
                serde_json::json!({
                    "profile_token": token
                }),
            )
            .await;
        fs::write(
            fixtures_dir.join("get_usual_order_found.json"),
            serde_json::to_string_pretty(&usual_priya).unwrap(),
        )
        .unwrap();
    }

    let forget_pref = client
        .call_tool(
            "forget_preference",
            serde_json::json!({
                "profile_token": profile_token,
                "kind": "allergy",
                "value": "peanuts",
                "confirm_remove_allergy": true
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("forget_preference_forgotten.json"),
        serde_json::to_string_pretty(&forget_pref).unwrap(),
    )
    .unwrap();

    let forget_me = client
        .call_tool(
            "forget_me",
            serde_json::json!({
                "profile_token": profile_token,
                "confirm": true
            }),
        )
        .await;
    fs::write(
        fixtures_dir.join("forget_me_forgotten.json"),
        serde_json::to_string_pretty(&forget_me).unwrap(),
    )
    .unwrap();

    println!("All schema dumping and fixture generation complete!");
}
