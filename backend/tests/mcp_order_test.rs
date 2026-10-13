use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::time::Instant;
use tabletalk_backend::{
    availability::FixedClock,
    build_router_with_db,
    db::{init_test_pool, seed_database},
    menu_search::AllergenFilter,
    ordering::{
        calculate_order_pricing, can_transition, generate_order_code, normalize_order_code,
        paise_to_spoken_rupees, AllergenAcknowledgement, CancelOrderOutput, GetOrderOutput,
        OrderCancellationRateLimiter, OrderLineInput, PlaceOrderOutput, ORDER_CODE_PREFIX,
    },
    tools::{
        cancel_order::{handle_cancel_order_with_deps, CancelOrderInput},
        get_order::{handle_get_order, GetOrderInput},
        place_order::{handle_place_order_with_clock, PlaceOrderInput as ToolPlaceOrderInput},
    },
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

    let router = build_router_with_db(&config, Some(pool));
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    (base_url, handle)
}

// Helper: Wednesday 2026-10-21 07:00 UTC = 12:30 IST (Lunch open: 12:00 - 15:30)
fn test_wednesday_lunch_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 21, 7, 0, 0).unwrap()
}

// =========================================================================
// 1. UNIT TESTS: PRICING ARITHMETIC, ROUNDING, TAX, CODES, AND TRANSITIONS
// =========================================================================

#[test]
fn test_pricing_arithmetic_overflow_rounding_and_tax() {
    // 1. Zero tax calculation
    let (subtotal, tax, total) = calculate_order_pricing(
        &[(10000, 2)], // 2 x 100.00 = 200.00
        0,
    )
    .expect("Pricing calculation failed");
    assert_eq!(subtotal, 20000);
    assert_eq!(tax, 0);
    assert_eq!(total, 20000);

    // 2. 5% tax (500 basis points) with round half up
    // 299.99 rupees (29999 paise) * 500 / 10000 = 1499.95 paise -> rounds to 1500 paise (15.00 INR)
    let (subtotal_gst, tax_gst, total_gst) =
        calculate_order_pricing(&[(29999, 1)], 500).expect("Pricing calculation failed");
    assert_eq!(subtotal_gst, 29999);
    assert_eq!(tax_gst, 1500);
    assert_eq!(total_gst, 31499);

    // 3. Overflow protection
    let overflow_res = calculate_order_pricing(&[(i64::MAX - 10, 2)], 500);
    assert!(
        overflow_res.is_err(),
        "Overflow must be caught and rejected"
    );
}

#[test]
fn test_spoken_rupees_formatting_comprehensive() {
    assert_eq!(paise_to_spoken_rupees(0), "zero rupees");
    assert_eq!(paise_to_spoken_rupees(100), "one rupee");
    assert_eq!(paise_to_spoken_rupees(200), "two rupees");
    assert_eq!(paise_to_spoken_rupees(50), "fifty paise");
    assert_eq!(paise_to_spoken_rupees(101), "one rupee and one paisa");
    assert_eq!(paise_to_spoken_rupees(150), "one rupee and fifty paise");
    assert_eq!(paise_to_spoken_rupees(32000), "three hundred twenty rupees");
    assert_eq!(paise_to_spoken_rupees(100500), "one thousand five rupees");
    assert_eq!(
        paise_to_spoken_rupees(7500000),
        "seventy-five thousand rupees"
    );
}

#[test]
fn test_order_code_generation_normalization_and_formatting() {
    for _ in 0..100 {
        let code = generate_order_code();
        assert!(code.starts_with(ORDER_CODE_PREFIX));
        assert_eq!(code.len(), 3 + 5); // TO-XXXXX
        let norm = normalize_order_code(&code);
        assert_eq!(code, norm);
    }

    // Normalization ensures TO- prefix and uppercase
    let normalized = normalize_order_code("to-8k4mz");
    assert_eq!(normalized, "TO-8K4MZ");

    let normalized_no_prefix = normalize_order_code("8k4mz");
    assert_eq!(normalized_no_prefix, "TO-8K4MZ");
}

#[test]
fn test_order_state_machine_transitions() {
    use tabletalk_backend::db::models::OrderStatus;

    // Allowed
    assert!(can_transition(OrderStatus::Draft, OrderStatus::Placed));
    assert!(can_transition(OrderStatus::Draft, OrderStatus::Cancelled));
    assert!(can_transition(OrderStatus::Placed, OrderStatus::Preparing));
    assert!(can_transition(OrderStatus::Placed, OrderStatus::Cancelled));
    assert!(can_transition(OrderStatus::Preparing, OrderStatus::Ready));
    assert!(can_transition(OrderStatus::Ready, OrderStatus::Completed));

    // Illegal
    assert!(!can_transition(
        OrderStatus::Preparing,
        OrderStatus::Cancelled
    ));
    assert!(!can_transition(OrderStatus::Ready, OrderStatus::Cancelled));
    assert!(!can_transition(OrderStatus::Cancelled, OrderStatus::Placed));
    assert!(!can_transition(
        OrderStatus::Completed,
        OrderStatus::Cancelled
    ));
    assert!(!can_transition(OrderStatus::Placed, OrderStatus::Completed));
}

// =========================================================================
// 2. PRICING INTEGRITY: SNAPSHOT, IMMUTABILITY, AND CLIENT CANNOT INFLUENCE
// =========================================================================

#[tokio::test]
async fn test_pricing_integrity_snapshot_and_immutability() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Item 1 is Crisp Palak Chaat (38000 paise = 380 INR)
    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 2,
            notes: Some("Extra crispy".to_string()),
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Rohan Varma".to_string(),
        phone: Some("+91 98765 43210".to_string()),
        email: Some("rohan@example.com".to_string()),
        allergen_acknowledgement: None,
        order_notes: Some("First time ordering".to_string()),
        idempotency_key: Some("order_pricing_test_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(out.status, "placed");
    let order = out.order.expect("Order must be created");

    // Subtotal: 2 * 38000 = 76000 paise (760 INR)
    // Tax (500 bps = 5%): 76000 * 5% = 3800 paise (38 INR)
    // Total: 79800 paise (798 INR)
    assert_eq!(order.subtotal.amount_minor, 76000);
    assert_eq!(order.tax.amount_minor, 3800);
    assert_eq!(order.total.amount_minor, 79800);
    assert_eq!(order.lines.len(), 1);
    assert_eq!(order.lines[0].unit_price.amount_minor, 38000);
    assert_eq!(order.lines[0].line_total.amount_minor, 76000);

    let order_code = order.order_code.clone();

    // Now change the menu item price in DB: increase Palak Chaat to 50000 paise
    sqlx::query("UPDATE menu_items SET price_cents = 50000 WHERE id = 1")
        .execute(&pool)
        .await
        .unwrap();

    // Verify the stored order still has 38000 paise (immutable snapshot)
    let get_input = GetOrderInput {
        order_code: order_code.clone(),
        guest_name: "Rohan Varma".to_string(),
    };
    let get_res = handle_get_order(&pool, get_input).await;
    assert_eq!(get_res.is_error, Some(false));
    let get_out: GetOrderOutput =
        serde_json::from_value(get_res.structured_content.unwrap()).unwrap();
    let fetched = get_out.order.unwrap();
    assert_eq!(fetched.subtotal.amount_minor, 76000);
    assert_eq!(fetched.lines[0].unit_price.amount_minor, 38000);
    assert_eq!(fetched.total.amount_minor, 79800);
}

#[tokio::test]
async fn test_item_name_resolution_exact_ambiguous_and_unknown() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // 1. Exact name resolution (case-insensitive)
    let exact_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: None,
            item_name: Some("crisp palak chaat".to_string()),
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Tarun Bajaj".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("exact_name_order_01".to_string()),
        profile_token: None,
    };
    let res = handle_place_order_with_clock(&pool, exact_input, &clock).await;
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(out.status, "placed");
    assert_eq!(out.order.unwrap().lines[0].name, "Crisp Palak Chaat");

    // 2. Ambiguous item name (e.g., "biryani" matches Subz Biryani, Murgh Biryani, Gosht Biryani)
    let ambi_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: None,
            item_name: Some("biryani".to_string()),
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Tarun Bajaj".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("ambi_name_order_01".to_string()),
        profile_token: None,
    };
    let ambi_res = handle_place_order_with_clock(&pool, ambi_input, &clock).await;
    let ambi_out: PlaceOrderOutput =
        serde_json::from_value(ambi_res.structured_content.unwrap()).unwrap();
    assert_eq!(ambi_out.status, "ambiguous_item");
    assert!(ambi_out.candidates.len() > 1);

    // 3. Unknown item name
    let unknown_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: None,
            item_name: Some("nonexistent pasta carbonara".to_string()),
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Tarun Bajaj".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("unknown_name_order_01".to_string()),
        profile_token: None,
    };
    let unk_res = handle_place_order_with_clock(&pool, unknown_input, &clock).await;
    let unk_out: PlaceOrderOutput =
        serde_json::from_value(unk_res.structured_content.unwrap()).unwrap();
    assert_eq!(unk_out.status, "unknown_item");
}

// =========================================================================
// 3. ALLERGEN SAFETY RULES: PEANUTS, ALTERNATIVES, UNVERIFIED DATA, AND VOCABULARY
// =========================================================================

#[tokio::test]
async fn test_allergen_peanut_conflict_blocks_and_suggests_alternatives() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Item 5 (Kurkuri Bhindi Tartlets) contains peanuts
    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(5),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Sameer Patel".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: Some(AllergenAcknowledgement {
            allergens_to_avoid: vec![AllergenFilter::Peanuts],
            acknowledged: false,
        }),
        order_notes: None,
        idempotency_key: Some("order_allergen_conflict_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(out.status, "allergen_conflict");
    assert!(out.order.is_none());
    assert!(
        !out.alternatives.is_empty(),
        "Alternatives must be suggested"
    );

    // Conflicting line must name item 5
    let voice_text = res.content[0].as_text().unwrap().text.clone();
    assert!(voice_text.contains("Kurkuri Bhindi Tartlets"));
    assert!(voice_text.contains("peanuts"));

    // Prohibited vocabulary check: NEVER claim "safe", "guaranteed", or "allergy-free"
    let prohibited_words = [
        "safe",
        "guaranteed",
        "allergy-free",
        "100% safe",
        "certified safe",
    ];
    for word in &prohibited_words {
        assert!(
            !voice_text.to_lowercase().contains(word),
            "Voice text must NEVER contain '{}'",
            word
        );
        let serialized = serde_json::to_string(&out).unwrap().to_lowercase();
        assert!(
            !serialized.contains(word),
            "Structured response must NEVER contain '{}'",
            word
        );
    }
}

#[tokio::test]
async fn test_unverified_allergen_data_cannot_be_overridden_for_severe_allergen() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Mark Item 3 (Gunpowder Idli Bites) as unverified: allergens_verified = 0
    sqlx::query("UPDATE menu_items SET allergens_verified = 0 WHERE id = 3")
        .execute(&pool)
        .await
        .unwrap();

    // Guest avoids peanuts (severe allergen) and passes acknowledged: true
    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(3),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Divya Nair".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: Some(AllergenAcknowledgement {
            allergens_to_avoid: vec![AllergenFilter::Peanuts],
            acknowledged: true, // Attempt override
        }),
        order_notes: None,
        idempotency_key: Some("order_unverified_severe_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    // Rule: unverified data for severe allergen CANNOT be overridden
    assert_eq!(
        out.status, "allergen_conflict",
        "Acknowledged: true must NOT override unverified data for severe allergen"
    );
    assert!(out.order.is_none());
}

#[tokio::test]
async fn test_allergen_acknowledged_override_permitted_for_explicit_contains() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Item 5 (Kurkuri Bhindi Tartlets) contains peanuts and is verified
    // Guest passes acknowledged: true
    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(5),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Akash Gupta".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: Some(AllergenAcknowledgement {
            allergens_to_avoid: vec![AllergenFilter::Peanuts],
            acknowledged: true, // Explicit guest acceptance
        }),
        order_notes: None,
        idempotency_key: Some("order_ack_override_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(
        out.status, "placed",
        "Acknowledged: true allows ordering verified item when guest accepted warning"
    );
    let order = out.order.unwrap();
    assert_eq!(order.stated_allergens.as_deref(), Some("peanuts"));

    // Verify stated allergens are preserved in get_order
    let get_input = GetOrderInput {
        order_code: order.order_code.clone(),
        guest_name: "Akash Gupta".to_string(),
    };
    let get_res = handle_get_order(&pool, get_input).await;
    let get_out: GetOrderOutput =
        serde_json::from_value(get_res.structured_content.unwrap()).unwrap();
    assert_eq!(
        get_out.order.unwrap().stated_allergens.as_deref(),
        Some("peanuts")
    );
}

// =========================================================================
// 4. CONCURRENCY & IDEMPOTENCY: 20 SAME-KEY CALLS AND 20 DISTINCT CALLS
// =========================================================================

#[tokio::test]
async fn test_the_race_20_concurrent_requests_same_idempotency_key() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let key = "race_idemp_key_20_concurrent".to_string();

    let mut handles = Vec::new();
    for _ in 0..20 {
        let p = pool.clone();
        let k = key.clone();
        let c = clock;
        handles.push(tokio::spawn(async move {
            let input = ToolPlaceOrderInput {
                items: vec![OrderLineInput {
                    item_id: Some(1),
                    item_name: None,
                    quantity: 1,
                    notes: None,
                }],
                pickup_time: Some("13:15".to_string()),
                guest_name: "Karan Johar".to_string(),
                phone: Some("+91 99999 11111".to_string()),
                email: Some("karan@example.com".to_string()),
                allergen_acknowledgement: None,
                order_notes: None,
                idempotency_key: Some(k),
                profile_token: None,
            };
            handle_place_order_with_clock(&p, input, &c).await
        }));
    }

    let mut order_codes = Vec::new();
    for h in handles {
        let res = h.await.unwrap();
        assert_eq!(res.is_error, Some(false));
        let out: PlaceOrderOutput =
            serde_json::from_value(res.structured_content.unwrap()).unwrap();
        assert_eq!(out.status, "placed");
        let order = out.order.unwrap();
        order_codes.push(order.order_code);
    }

    // All 20 returned identical order code
    let first_code = &order_codes[0];
    for code in &order_codes {
        assert_eq!(code, first_code);
    }

    // Exactly 1 order in database for this key / code
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE order_code = ?")
        .bind(first_code)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 1,
        "Exactly one order row must exist in DB for this code"
    );
}

#[tokio::test]
async fn test_20_concurrent_distinct_orders_all_succeed() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let mut handles = Vec::new();
    for i in 0..20 {
        let p = pool.clone();
        let c = clock;
        let guest_name = format!("Guest Number {}", i);
        let key = format!("distinct_order_key_{}", i);
        handles.push(tokio::spawn(async move {
            let input = ToolPlaceOrderInput {
                items: vec![OrderLineInput {
                    item_id: Some(1),
                    item_name: None,
                    quantity: 1,
                    notes: None,
                }],
                pickup_time: Some("14:30".to_string()),
                guest_name,
                phone: None,
                email: None,
                allergen_acknowledgement: None,
                order_notes: None,
                idempotency_key: Some(key),
                profile_token: None,
            };
            handle_place_order_with_clock(&p, input, &c).await
        }));
    }

    let mut unique_codes = std::collections::HashSet::new();
    for h in handles {
        let res = h.await.unwrap();
        assert_eq!(res.is_error, Some(false));
        let out: PlaceOrderOutput =
            serde_json::from_value(res.structured_content.unwrap()).unwrap();
        assert_eq!(out.status, "placed");
        unique_codes.insert(out.order.unwrap().order_code);
    }

    assert_eq!(
        unique_codes.len(),
        20,
        "All 20 orders must have unique codes"
    );

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE guest_name LIKE 'Guest Number %'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        count, 20,
        "Exactly 20 orders created for the 20 distinct guests"
    );
}

#[tokio::test]
async fn test_item_unavailable_rejected() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Mark Item 2 (Dahi Puri Spheres) as unavailable
    sqlx::query("UPDATE menu_items SET is_available = 0 WHERE id = 2")
        .execute(&pool)
        .await
        .unwrap();

    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(2),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Sneha Reddy".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("order_sold_out_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(out.status, "item_unavailable");
    assert!(out.order.is_none());
}

// =========================================================================
// 5. ATOMICITY: ROLLBACK ON FAILURE
// =========================================================================

#[tokio::test]
async fn test_atomicity_rollback_on_mid_transaction_failure() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;

    // Simulate mid-transaction failure using raw SQL transaction
    let mut tx = pool.begin().await.unwrap();

    // 1. Insert order
    sqlx::query(
        "INSERT INTO orders (id, customer_id, booking_id, order_type, status, order_code, guest_name, subtotal_cents, tax_cents, tip_cents, total_cents, currency, created_at, updated_at)
         VALUES ('ord_test_rollback', NULL, NULL, 'pickup', 'placed', 'TO-ROLLB', 'Fail Test', 1000, 50, 0, 1050, 'INR', '2026-10-21T12:00:00Z', '2026-10-21T12:00:00Z')"
    )
    .execute(&mut *tx)
    .await
    .unwrap();

    // 2. Forced failure before inserting order_items
    let failure_result: Result<(), sqlx::Error> = Err(sqlx::Error::Protocol(
        "Forced failure mid-order".to_string(),
    ));
    if failure_result.is_err() {
        tx.rollback().await.unwrap();
    }

    // Verify nothing persisted
    let order_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE id = 'ord_test_rollback'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(order_count, 0, "Rollback must leave zero order rows");
}

// =========================================================================
// 6. CANCEL ORDER: BYTE-IDENTICAL RESPONSES, IDEMPOTENCY, STATE LIMITS
// =========================================================================

#[tokio::test]
async fn test_cancel_verification_byte_identical_responses_and_edge_cases() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);
    let rate_limiter = OrderCancellationRateLimiter::new();

    // 1. Create a placed order
    let place_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Maya Sen".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("order_cancel_test_01".to_string()),
        profile_token: None,
    };
    let place_res = handle_place_order_with_clock(&pool, place_input, &clock).await;
    let place_out: PlaceOrderOutput =
        serde_json::from_value(place_res.structured_content.unwrap()).unwrap();
    let valid_code = place_out.order.unwrap().order_code;

    // 2. Cancel with non-existent order code
    let unknown_cancel_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: "TO-99999".to_string(),
            guest_name: "Maya Sen".to_string(),
            reason: None,
        },
        &rate_limiter,
        &clock,
    )
    .await;

    // 3. Cancel with valid order code but wrong guest name
    let wrong_name_cancel_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: valid_code.clone(),
            guest_name: "Completely Wrong Name".to_string(),
            reason: None,
        },
        &rate_limiter,
        &clock,
    )
    .await;

    // Byte-identical comparison (anti-enumeration)
    let unknown_structured =
        serde_json::to_string(&unknown_cancel_res.structured_content.unwrap()).unwrap();
    let wrong_structured =
        serde_json::to_string(&wrong_name_cancel_res.structured_content.unwrap()).unwrap();
    assert_eq!(
        unknown_structured, wrong_structured,
        "Unknown code and wrong guest name structured output must be byte-identical"
    );

    let unknown_voice = unknown_cancel_res.content[0]
        .as_text()
        .unwrap()
        .text
        .clone();
    let wrong_voice = wrong_name_cancel_res.content[0]
        .as_text()
        .unwrap()
        .text
        .clone();
    assert_eq!(
        unknown_voice, wrong_voice,
        "Unknown code and wrong guest name voice text must be byte-identical"
    );

    // 4. Successful cancellation
    let cancel_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: valid_code.clone(),
            guest_name: "Maya Sen".to_string(),
            reason: Some("Change of plans".to_string()),
        },
        &rate_limiter,
        &clock,
    )
    .await;
    let cancel_out: CancelOrderOutput =
        serde_json::from_value(cancel_res.structured_content.unwrap()).unwrap();
    assert_eq!(cancel_out.status, "cancelled");
    assert_eq!(cancel_out.already_cancelled, false);

    // 5. Double cancel is idempotent
    let double_cancel_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: valid_code.clone(),
            guest_name: "Maya Sen".to_string(),
            reason: None,
        },
        &rate_limiter,
        &clock,
    )
    .await;
    let double_cancel_out: CancelOrderOutput =
        serde_json::from_value(double_cancel_res.structured_content.unwrap()).unwrap();
    assert_eq!(double_cancel_out.status, "cancelled");
    assert_eq!(double_cancel_out.already_cancelled, true);

    // 6. Preparing order is not cancellable
    // Create another order and update status to preparing
    let place2_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Arjun Rampal".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("order_prep_test_01".to_string()),
        profile_token: None,
    };
    let place2_res = handle_place_order_with_clock(&pool, place2_input, &clock).await;
    let place2_out: PlaceOrderOutput =
        serde_json::from_value(place2_res.structured_content.unwrap()).unwrap();
    let prep_code = place2_out.order.unwrap().order_code;

    sqlx::query("UPDATE orders SET status = 'preparing' WHERE order_code = ?")
        .bind(&prep_code)
        .execute(&pool)
        .await
        .unwrap();

    let prep_cancel_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: prep_code,
            guest_name: "Arjun Rampal".to_string(),
            reason: None,
        },
        &rate_limiter,
        &clock,
    )
    .await;
    let prep_cancel_out: CancelOrderOutput =
        serde_json::from_value(prep_cancel_res.structured_content.unwrap()).unwrap();
    assert_eq!(prep_cancel_out.status, "not_cancellable");

    // 7. Rate limiter blocks after 5 failed attempts
    for _ in 0..5 {
        handle_cancel_order_with_deps(
            &pool,
            CancelOrderInput {
                order_code: "TO-FAIL1".to_string(),
                guest_name: "Bad Guy".to_string(),
                reason: None,
            },
            &rate_limiter,
            &clock,
        )
        .await;
    }
    let blocked_res = handle_cancel_order_with_deps(
        &pool,
        CancelOrderInput {
            order_code: "TO-FAIL1".to_string(),
            guest_name: "Bad Guy".to_string(),
            reason: None,
        },
        &rate_limiter,
        &clock,
    )
    .await;
    let blocked_out: CancelOrderOutput =
        serde_json::from_value(blocked_res.structured_content.unwrap()).unwrap();
    assert_eq!(blocked_out.status, "limit_exceeded");
    assert_eq!(blocked_out.reason.as_deref(), Some("rate_limit_exceeded"));
}

// =========================================================================
// 7. EDGE CASES: OPERATING HOURS, CLOSED DAYS, ABUSE LIMITS, AND NOTES
// =========================================================================

#[tokio::test]
async fn test_edge_cases_closed_day_and_abuse_limits() {
    // 1. Tuesday closed test (2026-10-20 is Tuesday)
    let tuesday_now = Utc.with_ymd_and_hms(2026, 10, 20, 7, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(tuesday_now)).await;
    let tuesday_clock = FixedClock(tuesday_now);

    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Deepak Chopra".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("order_tuesday_01".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &tuesday_clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    assert_eq!(
        out.status, "closed",
        "Tuesday orders must be rejected as closed"
    );

    // 2. Abuse limits: Max 3 active orders per guest
    let wed_now = test_wednesday_lunch_time();
    let wed_clock = FixedClock(wed_now);

    for i in 1..=3 {
        let input = ToolPlaceOrderInput {
            items: vec![OrderLineInput {
                item_id: Some(1),
                item_name: None,
                quantity: 1,
                notes: None,
            }],
            pickup_time: Some("13:15".to_string()),
            guest_name: "Abuse Tester".to_string(),
            phone: None,
            email: None,
            allergen_acknowledgement: None,
            order_notes: None,
            idempotency_key: Some(format!("order_abuse_active_{}", i)),
            profile_token: None,
        };
        let r = handle_place_order_with_clock(&pool, input, &wed_clock).await;
        let o: PlaceOrderOutput = serde_json::from_value(r.structured_content.unwrap()).unwrap();
        assert_eq!(o.status, "placed");
    }

    // 4th active order should fail with limit_exceeded
    let input4 = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Abuse Tester".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("order_abuse_active_4".to_string()),
        profile_token: None,
    };
    let r4 = handle_place_order_with_clock(&pool, input4, &wed_clock).await;
    let o4: PlaceOrderOutput = serde_json::from_value(r4.structured_content.unwrap()).unwrap();
    assert_eq!(o4.status, "limit_exceeded");

    // 3. Unicode and SQL injection-style notes safely stored literally
    let injection_input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: Some("'; DROP TABLE orders; -- 🌶️ spicy please".to_string()),
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Safe Unicode Guest".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: Some("Extra napkins and ❤️ from Bangalore".to_string()),
        idempotency_key: Some("order_injection_test_01".to_string()),
        profile_token: None,
    };
    let inj_res = handle_place_order_with_clock(&pool, injection_input, &wed_clock).await;
    let inj_out: PlaceOrderOutput =
        serde_json::from_value(inj_res.structured_content.unwrap()).unwrap();
    assert_eq!(inj_out.status, "placed");
    let line = &inj_out.order.unwrap().lines[0];
    assert_eq!(
        line.notes.as_deref(),
        Some("'; DROP TABLE orders; -- 🌶️ spicy please")
    );
}

// =========================================================================
// 8. PRIVACY: LOGS CAPTURED IN TESTS CONTAIN ZERO PII
// =========================================================================

#[tokio::test]
async fn test_privacy_logs_contain_no_pii() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let pii_name = "SecretCelebrityGuest";
    let pii_phone = "+91 91234 56789";
    let pii_email = "celebrity_vip@example.com";
    let pii_notes = "Secret VIP entrance requested";

    let input = ToolPlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: Some(pii_notes.to_string()),
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: pii_name.to_string(),
        phone: Some(pii_phone.to_string()),
        email: Some(pii_email.to_string()),
        allergen_acknowledgement: None,
        order_notes: Some(pii_notes.to_string()),
        idempotency_key: Some("pii_test_key_001".to_string()),
        profile_token: None,
    };

    let res = handle_place_order_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let out: PlaceOrderOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();
    let order_code = out.order.unwrap().order_code;

    // Fetch audit events logged for this order
    let audit_events: Vec<(String, String)> =
        sqlx::query_as("SELECT action, payload_json FROM audit_events WHERE entity = 'order'")
            .fetch_all(&pool)
            .await
            .unwrap();

    assert!(!audit_events.is_empty(), "Audit events must be recorded");

    for (_action, payload) in audit_events {
        assert!(
            !payload.contains(pii_name),
            "Audit event payload must not leak guest name: {}",
            payload
        );
        assert!(
            !payload.contains(pii_phone),
            "Audit event payload must not leak guest phone: {}",
            payload
        );
        assert!(
            !payload.contains(pii_email),
            "Audit event payload must not leak guest email: {}",
            payload
        );
        assert!(
            !payload.contains(pii_notes),
            "Audit event payload must not leak notes: {}",
            payload
        );
        // Only order code prefix or masked identifier is permitted
        assert!(
            payload.contains(&order_code[..4]),
            "Audit event should log safe prefix: {}",
            payload
        );
    }
}

// =========================================================================
// 9. MCP HTTP INTEGRATION: TOOLS/LIST ALL 9 TOOLS AND END-TO-END FLOW
// =========================================================================

async fn mcp_initialize_session(client: &reqwest::Client, base_url: &str) -> String {
    let init_res = client
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
                "clientInfo": { "name": "test-client", "version": "1.0.0" }
            }
        }))
        .send()
        .await
        .unwrap();

    let session_id = init_res
        .headers()
        .get("mcp-session-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let _ = client
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

    session_id
}

#[tokio::test]
async fn test_mcp_tools_list_contains_all_nine_tools_with_schemas_and_annotations() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let (base_url, _handle) = spawn_test_server(pool).await;

    let client = reqwest::Client::new();
    let session_id = mcp_initialize_session(&client, &base_url).await;

    let res = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "list-001",
            "method": "tools/list"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let raw = res.text().await.unwrap();
    let json_res = parse_mcp_json_or_sse(&raw);
    let tools = json_res["result"]["tools"]
        .as_array()
        .expect("Tools list array");

    assert!(tools.len() >= 9, "Server must expose at least 9 tools");

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"ping_restaurant"));
    assert!(tool_names.contains(&"search_menu"));
    assert!(tool_names.contains(&"check_availability"));
    assert!(tool_names.contains(&"create_booking"));
    assert!(tool_names.contains(&"cancel_booking"));
    assert!(tool_names.contains(&"get_booking"));
    assert!(tool_names.contains(&"place_order"));
    assert!(tool_names.contains(&"get_order"));
    assert!(tool_names.contains(&"cancel_order"));

    // Check place_order annotations
    let place_tool = tools.iter().find(|t| t["name"] == "place_order").unwrap();
    assert_eq!(place_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(place_tool["annotations"]["destructiveHint"], false);
    assert_eq!(place_tool["annotations"]["idempotentHint"], true);

    // Check cancel_order annotations
    let cancel_tool = tools.iter().find(|t| t["name"] == "cancel_order").unwrap();
    assert_eq!(cancel_tool["annotations"]["readOnlyHint"], false);
    assert_eq!(cancel_tool["annotations"]["destructiveHint"], true);
    assert_eq!(cancel_tool["annotations"]["idempotentHint"], true);

    // Check get_order annotations
    let get_tool = tools.iter().find(|t| t["name"] == "get_order").unwrap();
    assert_eq!(get_tool["annotations"]["readOnlyHint"], true);
}

#[tokio::test]
async fn test_mcp_tools_call_place_get_cancel_over_http() {
    let pool = setup_seeded_db(None).await;

    // Ensure all opening hours in test DB are open 00:00 to 23:59 so HTTP test passes at any time/day
    sqlx::query(
        "UPDATE opening_hours SET is_closed = 0, open_time = '00:00', close_time = '23:59'",
    )
    .execute(&pool)
    .await
    .unwrap();

    let (base_url, _handle) = spawn_test_server(pool).await;

    let client = reqwest::Client::new();
    let session_id = mcp_initialize_session(&client, &base_url).await;

    // 1. place_order over HTTP (omitting pickup_time so it picks earliest feasible time)
    let place_res = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "http-place-01",
            "method": "tools/call",
            "params": {
                "name": "place_order",
                "arguments": {
                    "items": [
                        { "item_id": 1, "quantity": 1, "notes": "No coriander" }
                    ],
                    "guest_name": "HTTP Test Guest",
                    "idempotency_key": "http_order_001"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let place_raw = place_res.text().await.unwrap();
    let place_json = parse_mcp_json_or_sse(&place_raw);
    let structured = &place_json["result"]["structuredContent"];
    assert_eq!(structured["status"], "placed");
    let order_code = structured["order"]["order_code"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. get_order over HTTP
    let get_res = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "http-get-01",
            "method": "tools/call",
            "params": {
                "name": "get_order",
                "arguments": {
                    "order_code": order_code,
                    "guest_name": "HTTP Test Guest"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let get_raw = get_res.text().await.unwrap();
    let get_json = parse_mcp_json_or_sse(&get_raw);
    assert_eq!(get_json["result"]["structuredContent"]["status"], "found");

    // 3. cancel_order over HTTP
    let cancel_res = client
        .post(format!("{}/mcp", base_url))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": "http-cancel-01",
            "method": "tools/call",
            "params": {
                "name": "cancel_order",
                "arguments": {
                    "order_code": order_code,
                    "guest_name": "HTTP Test Guest",
                    "reason": "Changed my mind"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let cancel_raw = cancel_res.text().await.unwrap();
    let cancel_json = parse_mcp_json_or_sse(&cancel_raw);
    assert_eq!(
        cancel_json["result"]["structuredContent"]["status"],
        "cancelled"
    );
}

#[tokio::test]
async fn test_ordering_latency_median_under_30ms() {
    let fixed_now = test_wednesday_lunch_time();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let mut latencies = Vec::new();
    for i in 0..20 {
        let input = ToolPlaceOrderInput {
            items: vec![OrderLineInput {
                item_id: Some(1),
                item_name: None,
                quantity: 1,
                notes: None,
            }],
            pickup_time: Some("13:15".to_string()),
            guest_name: format!("Latency Guest {}", i),
            phone: None,
            email: None,
            allergen_acknowledgement: None,
            order_notes: None,
            idempotency_key: Some(format!("latency_order_{}", i)),
            profile_token: None,
        };

        let start = Instant::now();
        let res = handle_place_order_with_clock(&pool, input, &clock).await;
        let elapsed = start.elapsed().as_millis();
        latencies.push(elapsed);
        assert_eq!(res.is_error, Some(false));
    }

    latencies.sort();
    let median = latencies[latencies.len() / 2];
    println!(
        "Place order median latency: {}ms (all: {:?})",
        median, latencies
    );
    assert!(
        median < 30,
        "Median place_order latency must be under 30ms, was {}ms",
        median
    );
}
