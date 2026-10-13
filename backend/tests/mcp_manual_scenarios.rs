use chrono::{TimeZone, Utc};
use tabletalk_backend::{
    availability::FixedClock,
    db::{init_test_pool, seed_database},
    menu_search::AllergenFilter,
    ordering::{AllergenAcknowledgement, OrderCancellationRateLimiter, OrderLineInput},
    tools::{
        cancel_order::{handle_cancel_order_with_deps, CancelOrderInput},
        get_order::{handle_get_order, GetOrderInput},
        place_order::{handle_place_order_with_clock, PlaceOrderInput},
    },
};

#[tokio::test]
async fn run_and_print_nine_manual_scenarios() {
    // Wednesday 2026-10-21 07:00 UTC = 12:30 IST (Lunch service 12:00 - 15:30)
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 21, 7, 0, 0).unwrap();
    let pool = init_test_pool().await.unwrap();
    seed_database(&pool, Some(fixed_now)).await.unwrap();
    let clock = FixedClock(fixed_now);
    let rate_limiter = OrderCancellationRateLimiter::new();

    println!("========== SCENARIO 1: A NORMAL ORDER ==========");
    let normal_input = PlaceOrderInput {
        items: vec![
            OrderLineInput {
                item_id: Some(1),
                item_name: None,
                quantity: 2,
                notes: Some("Extra crispy leaves".to_string()),
            },
            OrderLineInput {
                item_id: Some(25),
                item_name: None,
                quantity: 2,
                notes: None,
            },
        ],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Aarav Mehta".to_string(),
        phone: Some("+91 98765 43210".to_string()),
        email: Some("aarav@example.com".to_string()),
        allergen_acknowledgement: None,
        order_notes: Some("Please include cutlery".to_string()),
        idempotency_key: Some("manual_idemp_key_001".to_string()),
    };
    let res1 = handle_place_order_with_clock(&pool, normal_input.clone(), &clock).await;
    let json1 = serde_json::to_string_pretty(&res1).unwrap();
    println!("{}", json1);
    let placed_order_code = res1.structured_content.as_ref().unwrap()["order"]["order_code"]
        .as_str()
        .unwrap()
        .to_string();

    println!("\n========== SCENARIO 2: IDEMPOTENT REPLAY ==========");
    let res2 = handle_place_order_with_clock(&pool, normal_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res2).unwrap());

    println!("\n========== SCENARIO 3: ALLERGEN CONFLICT (PEANUT ALLERGY) ==========");
    let peanut_conflict_input = PlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(5), // Kurkuri Bhindi Tartlets (contains peanuts)
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Pooja Hegde".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: Some(AllergenAcknowledgement {
            allergens_to_avoid: vec![AllergenFilter::Peanuts],
            acknowledged: false,
        }),
        order_notes: None,
        idempotency_key: Some("manual_idemp_key_002".to_string()),
    };
    let res3 = handle_place_order_with_clock(&pool, peanut_conflict_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res3).unwrap());

    println!("\n========== SCENARIO 4: AMBIGUOUS ITEM NAME ==========");
    let ambi_input = PlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: None,
            item_name: Some("biryani".to_string()),
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Vikram Seth".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("manual_idemp_key_003".to_string()),
    };
    let res4 = handle_place_order_with_clock(&pool, ambi_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res4).unwrap());

    println!("\n========== SCENARIO 5: PICKUP TIME TOO SOON ==========");
    let too_soon_input = PlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("12:35".to_string()), // Current time is 12:30, prep takes 12 min -> 12:42 snapped to 12:45
        guest_name: "Rahul Bose".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("manual_idemp_key_004".to_string()),
    };
    let res5 = handle_place_order_with_clock(&pool, too_soon_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res5).unwrap());

    println!("\n========== SCENARIO 6: CLOSED HOURS REQUEST ==========");
    // Tuesday 2026-10-20 (closed day)
    let tuesday_now = Utc.with_ymd_and_hms(2026, 10, 20, 7, 0, 0).unwrap();
    let tuesday_clock = FixedClock(tuesday_now);
    let closed_input = PlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: "Neha Sharma".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("manual_idemp_key_005".to_string()),
    };
    let res6 = handle_place_order_with_clock(&pool, closed_input, &tuesday_clock).await;
    println!("{}", serde_json::to_string_pretty(&res6).unwrap());

    println!("\n========== SCENARIO 7: GET_ORDER WITH WRONG NAME (ORACLE DEFENSE) ==========");
    let wrong_name_get = GetOrderInput {
        order_code: placed_order_code.clone(),
        guest_name: "Wrong Name Person".to_string(),
    };
    let res7 = handle_get_order(&pool, wrong_name_get).await;
    println!("{}", serde_json::to_string_pretty(&res7).unwrap());

    println!("\n========== SCENARIO 8: CANCEL_ORDER ==========");
    let cancel_input = CancelOrderInput {
        order_code: placed_order_code,
        guest_name: "Aarav Mehta".to_string(),
        reason: Some("Change of lunch plans".to_string()),
    };
    let res8 = handle_cancel_order_with_deps(&pool, cancel_input, &rate_limiter, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res8).unwrap());

    println!("\n========== SCENARIO 9: INVALID ARGUMENT (VALIDATION ERROR) ==========");
    let invalid_input = PlaceOrderInput {
        items: vec![], // Empty items
        pickup_time: Some("13:15".to_string()),
        guest_name: "Invalid Tester".to_string(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: None,
    };
    let res9 = handle_place_order_with_clock(&pool, invalid_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&res9).unwrap());
}
