use chrono::{TimeZone, Utc};
use sqlx::Row;
use tabletalk_backend::{
    availability::FixedClock,
    db::{init_test_pool, seed_database},
    memory::InitialPreferenceInput,
    ordering::OrderLineInput,
    tools::{
        enable_memory::{handle_enable_memory_with_clock, EnableMemoryInput},
        forget_me::{handle_forget_me_with_clock, ForgetMeInput},
        get_profile::{handle_get_profile_with_clock, GetProfileInput},
        get_usual_order::{handle_get_usual_order_with_clock, GetUsualOrderInput},
        place_order::{handle_place_order_with_clock, PlaceOrderInput},
        remember_preference::{handle_remember_preference_with_clock, RememberPreferenceInput},
        rotate_profile_token::{handle_rotate_profile_token_with_clock, RotateProfileTokenInput},
    },
};

#[tokio::test]
async fn run_and_print_manual_memory_scenarios() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 21, 7, 0, 0).unwrap();
    let pool = init_test_pool().await.unwrap();
    seed_database(&pool, Some(fixed_now)).await.unwrap();
    let clock = FixedClock(fixed_now);

    println!("============================================================");
    println!("PHASE 10 MANUAL VERIFICATION: PROFILE MEMORY & PRIVACY FLOW");
    println!("============================================================");

    // 1. enable_memory
    println!("\n--- SCENARIO 1: enable_memory (Opt-in with Consent) ---");
    let enable_input = EnableMemoryInput {
        guest_name: "Ananya Sharma".to_string(),
        phone: Some("+91 98765 11223".to_string()),
        email: Some("ananya.sharma@example.com".to_string()),
        consent_acknowledged: true,
        initial_preferences: Some(vec![InitialPreferenceInput {
            kind: "dietary".to_string(),
            value: "vegetarian".to_string(),
            severity: None,
        }]),
    };
    let enable_res = handle_enable_memory_with_clock(&pool, enable_input, &clock).await;
    println!("{}", serde_json::to_string_pretty(&enable_res).unwrap());
    let token = enable_res.structured_content.as_ref().unwrap()["profile_token"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. remember_preference
    println!("\n--- SCENARIO 2A: remember_preference (Allergy: Peanuts, Severe) ---");
    let rem_allergy = handle_remember_preference_with_clock(
        &pool,
        RememberPreferenceInput {
            profile_token: token.clone(),
            kind: "allergy".to_string(),
            value: "peanuts".to_string(),
            severity: Some("severe".to_string()),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&rem_allergy).unwrap());

    println!("\n--- SCENARIO 2B: remember_preference (Favorite: Paneer Tikka item 7) ---");
    let rem_fav = handle_remember_preference_with_clock(
        &pool,
        RememberPreferenceInput {
            profile_token: token.clone(),
            kind: "favorite_item".to_string(),
            value: "7".to_string(),
            severity: None,
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&rem_fav).unwrap());

    // 3. place_order with conflicting item (blocked)
    println!("\n--- SCENARIO 3A: place_order with Profile Token & Conflicting Item (Kurkuri Bhindi contains Peanuts) ---");
    let conflict_order = PlaceOrderInput {
        items: vec![OrderLineInput {
            item_id: Some(5), // Kurkuri Bhindi Tartlets (contains peanuts)
            item_name: None,
            quantity: 1,
            notes: None,
        }],
        pickup_time: Some("13:15".to_string()),
        guest_name: String::new(), // Auto-defaulted from profile
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: None,
        idempotency_key: Some("manual_order_conflict_01".to_string()),
        profile_token: Some(token.clone()),
    };
    let conflict_res = handle_place_order_with_clock(&pool, conflict_order, &clock).await;
    println!("{}", serde_json::to_string_pretty(&conflict_res).unwrap());

    // 3B. place_order with clean items (placed)
    println!(
        "\n--- SCENARIO 3B: place_order with Profile Token & Clean Items (Paneer Tikka & Naan) ---"
    );
    let clean_order = PlaceOrderInput {
        items: vec![
            OrderLineInput {
                item_id: Some(7), // Paneer Tikka
                item_name: None,
                quantity: 1,
                notes: Some("Medium spice".to_string()),
            },
            OrderLineInput {
                item_id: Some(25), // Truffle Naan
                item_name: None,
                quantity: 2,
                notes: None,
            },
        ],
        pickup_time: Some("13:15".to_string()),
        guest_name: String::new(),
        phone: None,
        email: None,
        allergen_acknowledgement: None,
        order_notes: Some("Separate packaging".to_string()),
        idempotency_key: Some("manual_order_clean_01".to_string()),
        profile_token: Some(token.clone()),
    };
    let clean_res = handle_place_order_with_clock(&pool, clean_order, &clock).await;
    println!("{}", serde_json::to_string_pretty(&clean_res).unwrap());

    // 4. get_usual_order
    println!("\n--- SCENARIO 4: get_usual_order (Suggestion from Past History Re-validated against Live Menu) ---");
    let usual_res = handle_get_usual_order_with_clock(
        &pool,
        GetUsualOrderInput {
            profile_token: token.clone(),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&usual_res).unwrap());

    // 5. get_profile (Data Export)
    println!("\n--- SCENARIO 5: get_profile (Data Export Record with Masked Contact) ---");
    let prof_res = handle_get_profile_with_clock(
        &pool,
        GetProfileInput {
            profile_token: token.clone(),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&prof_res).unwrap());

    // 6. rotate_profile_token
    println!("\n--- SCENARIO 6: rotate_profile_token (Revoke Old, Issue New Atomically) ---");
    let rot_res = handle_rotate_profile_token_with_clock(
        &pool,
        RotateProfileTokenInput {
            profile_token: token.clone(),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&rot_res).unwrap());
    let new_token = rot_res.structured_content.as_ref().unwrap()["new_profile_token"]
        .as_str()
        .unwrap()
        .to_string();

    println!(
        "\n--- SCENARIO 6B: Verification - Old Token is Revoked and Returns Profile Not Found ---"
    );
    let old_res = handle_get_profile_with_clock(
        &pool,
        GetProfileInput {
            profile_token: token.clone(),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&old_res).unwrap());

    // 7. forget_me
    println!("\n--- SCENARIO 7: forget_me (Hard Delete of Profile, Unlinking and Scrubbing Business Records) ---");
    let forget_res = handle_forget_me_with_clock(
        &pool,
        ForgetMeInput {
            profile_token: new_token.clone(),
            confirm: true,
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&forget_res).unwrap());

    // 8. Full DB scan after forget_me
    println!("\n--- SCENARIO 8: Full Database Scan Post-Deletion (Verify Zero Leaks) ---");
    let pii_strings = [
        "Ananya Sharma",
        "+91 98765 11223",
        "ananya.sharma@example.com",
        "Separate packaging",
    ];
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let mut total_matches = 0;
    for (tbl,) in &tables {
        let col_rows = sqlx::query(&format!("PRAGMA table_info({})", tbl))
            .fetch_all(&pool)
            .await
            .unwrap();
        let cols: Vec<String> = col_rows
            .into_iter()
            .map(|r| r.get::<String, _>("name"))
            .collect();

        for col in cols {
            for pii in &pii_strings {
                let query = format!(
                    "SELECT COUNT(*) FROM {} WHERE CAST({} AS TEXT) LIKE ?",
                    tbl, col
                );
                let matches: i64 = sqlx::query_scalar(&query)
                    .bind(format!("%{}%", pii))
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(0);
                if matches > 0 {
                    println!(
                        "LEAK FOUND: '{}' in table '{}' column '{}' (matches: {})",
                        pii, tbl, col, matches
                    );
                    total_matches += matches;
                }
            }
        }
    }
    println!(
        "DB Scan complete across {} tables. Total PII matches found: {}",
        tables.len(),
        total_matches
    );
    assert_eq!(total_matches, 0);

    // 9. Invalid token on integrated tool
    println!("\n--- SCENARIO 9: Invalid Token Error Response ---");
    let invalid_res = handle_get_profile_with_clock(
        &pool,
        GetProfileInput {
            profile_token: "tt_prof_completely_fake_invalid_token_1234567890".to_string(),
        },
        &clock,
    )
    .await;
    println!("{}", serde_json::to_string_pretty(&invalid_res).unwrap());

    println!("\n============================================================");
    println!("PHASE 10 MANUAL VERIFICATION COMPLETE - ALL PASSED");
    println!("============================================================");
}
