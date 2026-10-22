use std::sync::Arc;
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use sqlx::SqlitePool;

use tabletalk_backend::availability::{Clock, FixedClock};
use tabletalk_backend::db::seed::seed_database;
use tabletalk_backend::orchestrator::router::{
    extract_confirmation_code, extract_guest_name, extract_party_size, extract_time,
    format_spaced_code, handle_scripted_chat, normalize_text, resolve_date,
    sanitize_spoken_safety,
};
use tabletalk_backend::orchestrator::session::SessionManager;

/// Fixed clock timestamp: Thursday Oct 22, 2026, 14:07:01 IST (+05:30)
/// UTC equivalent: 2026-10-22T08:37:01Z
fn test_clock() -> Arc<dyn Clock> {
    let dt_utc = Utc.with_ymd_and_hms(2026, 10, 22, 8, 37, 1).unwrap();
    Arc::new(FixedClock(dt_utc))
}

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    seed_database(&pool, Some(Utc.with_ymd_and_hms(2026, 10, 22, 8, 37, 1).unwrap()))
        .await
        .unwrap();
    pool
}

// -------------------------------------------------------------
// Unit Tests: Normalization, Slot Extraction & Date Resolution
// -------------------------------------------------------------

#[test]
fn test_normalization() {
    assert_eq!(
        normalize_text("  Table for 4, Friday at 7:30 PM!  "),
        "table for 4 friday at 7:30 pm"
    );
    assert_eq!(
        normalize_text("Vegan dishes... without peanuts???"),
        "vegan dishes without peanuts"
    );
}

#[test]
fn test_number_and_party_size_parsing() {
    assert_eq!(extract_party_size("table for 4 Friday at 7:30"), Some(4));
    assert_eq!(extract_party_size("table for four"), Some(4));
    assert_eq!(extract_party_size("table for 8 Saturday at 8"), Some(8));
    assert_eq!(extract_party_size("party of eight"), Some(8));
    assert_eq!(extract_party_size("for two guests"), Some(2));
    assert_eq!(extract_party_size("for 2"), Some(2));
    assert_eq!(extract_party_size("pickup at 8"), None);
}

#[test]
fn test_time_parsing() {
    let t1 = extract_time("table for 4 Friday at 7:30").unwrap();
    assert_eq!(t1.to_string(), "19:30:00");

    let t2 = extract_time("seven thirty pm").unwrap();
    assert_eq!(t2.to_string(), "19:30:00");

    let t3 = extract_time("half past seven").unwrap();
    assert_eq!(t3.to_string(), "19:30:00");

    let t4 = extract_time("at 8 pm").unwrap();
    assert_eq!(t4.to_string(), "20:00:00");

    let t5 = extract_time("for pickup at 8").unwrap();
    assert_eq!(t5.to_string(), "20:00:00");

    let t6 = extract_time("at 19:30").unwrap();
    assert_eq!(t6.to_string(), "19:30:00");
}

#[test]
fn test_date_resolution_fixed_clock() {
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    // Today is Thursday Oct 22, 2026
    let d_fri = resolve_date("Friday", clock.as_ref(), tz, None);
    assert_eq!(d_fri, "2026-10-23", "Friday must resolve to Oct 23, 2026");

    let d_this_fri = resolve_date("this Friday at 7:30", clock.as_ref(), tz, None);
    assert_eq!(d_this_fri, "2026-10-23");

    let d_next_fri = resolve_date("next Friday", clock.as_ref(), tz, None);
    assert_eq!(d_next_fri, "2026-10-23");

    let d_sat = resolve_date("Saturday at 8", clock.as_ref(), tz, None);
    assert_eq!(d_sat, "2026-10-24");

    let d_today = resolve_date("today", clock.as_ref(), tz, None);
    assert_eq!(d_today, "2026-10-22");

    let d_tonight = resolve_date("tonight", clock.as_ref(), tz, None);
    assert_eq!(d_tonight, "2026-10-22");

    let d_tomorrow = resolve_date("tomorrow", clock.as_ref(), tz, None);
    assert_eq!(d_tomorrow, "2026-10-23");

    // Past date check: an ISO date in the past must never be returned
    let d_past = resolve_date("2020-01-01", clock.as_ref(), tz, None);
    assert!(d_past.as_str() >= "2026-10-22", "Must never return past date");
}

#[test]
fn test_guest_name_and_code_extraction() {
    assert_eq!(extract_guest_name("my name is Ravi"), Some("Ravi".to_string()));
    assert_eq!(extract_guest_name("under Ravi"), Some("Ravi".to_string()));
    assert_eq!(extract_guest_name("for Ravi"), Some("Ravi".to_string()));
    assert_eq!(extract_guest_name("for two"), None);

    assert_eq!(extract_confirmation_code("cancel my booking ABC47Z under Ravi"), Some("ABC47Z".to_string()));
    assert_eq!(extract_confirmation_code("booking TT-9201"), Some("TT-9201".to_string()));

    assert_eq!(format_spaced_code("TT-9201"), "T T hyphen 9 2 0 1");
    assert_eq!(format_spaced_code("ABC47Z"), "A B C 4 7 Z");
}

#[test]
fn test_safety_sanitizer() {
    let input = "This dish is 100% allergy-free and safe to eat with a guaranteed safe prep.";
    let cleaned = sanitize_spoken_safety(input);
    assert!(!cleaned.contains("allergy-free"));
    assert!(!cleaned.contains("safe to eat"));
    assert!(!cleaned.contains("guaranteed safe"));
}

// -------------------------------------------------------------
// Integration Tests: Scenarios S1 through S15 on In-Memory DB
// -------------------------------------------------------------

#[tokio::test]
async fn test_scenario_s1_greeting_and_help() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s1_sess", "what can you do", None).await;
    assert!(resp.structured_contents.is_empty(), "Greeting should not invoke tools");
    assert!(resp.message.content.to_lowercase().contains("saffron nebula"));
    assert!(resp.message.content.to_lowercase().contains("reserve"));
}

#[tokio::test]
async fn test_scenario_s2_time_and_date() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s2_sess", "what is time now", None).await;
    assert!(resp.structured_contents.is_empty(), "Time query needs no tool");
    assert!(resp.message.content.contains("October twenty-second"));
    assert!(resp.message.content.contains("Thursday"));
}

#[tokio::test]
async fn test_scenario_s3_opening_hours() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp1 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s3_sess", "when are you open", None).await;
    assert!(resp1.message.content.contains("12:00 PM"));
    assert!(resp1.message.content.contains("closed on Tuesdays"));

    let resp2 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s3_sess", "are you open Sunday", None).await;
    assert!(resp2.message.content.contains("Sundays"));
    assert!(resp2.message.content.contains("4:00 PM"));
}

#[tokio::test]
async fn test_scenario_s4_menu_search() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s4_sess", "vegan options without peanuts", None).await;
    assert_eq!(resp.structured_contents.len(), 1);
    assert_eq!(resp.structured_contents[0]["_tool_name"], "search_menu");
    assert!(!resp.message.content.is_empty());
}

#[tokio::test]
async fn test_scenario_s5_allergy_search_never_safe() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "s5_sess", "I'm allergic to peanuts, what can I eat", None).await;
    assert_eq!(resp.structured_contents.len(), 1);
    assert_eq!(resp.structured_contents[0]["_tool_name"], "search_menu");
    assert!(resp.message.content.contains("listed as not containing peanuts"));
    assert!(resp.message.content.contains("confirm with our staff"));

    // Critical safety invariants
    assert!(!resp.message.content.to_lowercase().contains("safe"));
    assert!(!resp.message.content.to_lowercase().contains("allergy-free"));
    assert!(!resp.message.content.to_lowercase().contains("guaranteed"));
}

#[tokio::test]
async fn test_scenario_s6_and_s7_hero_flow_multi_turn() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();
    let session_id = "hero_session_1";

    // Turn 1 (S6): Check availability for Friday 19:30 table for 4
    let resp1 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "table for 4 Friday at 7:30", None).await;
    assert_eq!(resp1.structured_contents.len(), 1);
    assert_eq!(resp1.structured_contents[0]["_tool_name"], "check_availability");
    assert_eq!(resp1.structured_contents[0]["status"], "unavailable");
    assert_eq!(resp1.structured_contents[0]["reason"], "fully_booked");
    println!("CONTENT: {}", resp1.message.content);
    assert!(resp1.message.content.to_lowercase().contains("fully booked"));
    assert!(resp1.message.content.to_lowercase().contains("patio"));

    // Turn 2 (S7): Follow-up negotiation - pick "the patio one"
    let resp2 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "the patio one", None).await;
    assert!(resp2.structured_contents.is_empty(), "Selecting option does not run tool");
    assert!(resp2.message.content.contains("Patio table"));
    assert!(resp2.message.content.contains("hold this table for 5 minutes"));

    // Turn 3 (S7): "hold it"
    let resp3 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "hold it", None).await;
    assert!(resp3.message.content.contains("temporary hold"));
    assert!(resp3.message.content.contains("5 minutes"));

    // Turn 4 (S7): "yes book it" (missing name -> asks name)
    let resp4 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "yes book it", None).await;
    assert!(resp4.message.content.contains("May I have your name"));

    // Turn 5 (S7): "my name is Ravi" -> creates booking and confirms
    let resp5 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "my name is Ravi", None).await;
    assert_eq!(resp5.structured_contents.len(), 1);
    assert_eq!(resp5.structured_contents[0]["_tool_name"], "create_booking");
    assert!(resp5.message.content.contains("confirmed"));
    assert!(resp5.message.content.contains("Ravi"));
    // Spaced confirmation code test (e.g. contains space between characters)
    assert!(resp5.message.content.contains("T T") || resp5.message.content.contains("hyphen"));
}

#[tokio::test]
async fn test_scenario_s8_waitlist() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();
    let session_id = "waitlist_session";

    // Seed state with party size & date
    let _ = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "table for 4 Friday at 7:30", None).await;

    // Join waitlist
    let resp_join = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "put me on the waitlist", None).await;
    assert_eq!(resp_join.structured_contents.len(), 1);
    assert_eq!(resp_join.structured_contents[0]["_tool_name"], "join_waitlist");
    assert!(resp_join.message.content.contains("waitlist"));

    // Check waitlist status
    let resp_status = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "check my waitlist", None).await;
    assert_eq!(resp_status.structured_contents.len(), 1);
    assert_eq!(resp_status.structured_contents[0]["_tool_name"], "get_waitlist_status");
}

#[tokio::test]
async fn test_scenario_s9_large_party_deposit() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "deposit_sess", "table for 8 Saturday at 8", None).await;
    assert_eq!(resp.structured_contents.len(), 1);
    assert_eq!(resp.structured_contents[0]["_tool_name"], "create_booking");
    assert!(resp.message.content.contains("2000 rupees"));
    assert!(resp.message.content.contains("simulated checkout"));
    assert!(!resp.message.content.to_lowercase().contains("i have processed your payment"));
}

#[tokio::test]
async fn test_scenario_s10_pickup_order() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(
        &pool,
        &session_mgr,
        clock.as_ref(),
        tz,
        "order_sess",
        "order paneer tikka and two garlic naan for pickup at 8, name Ravi",
        None,
    )
    .await;
    assert_eq!(resp.structured_contents.len(), 1);
    assert_eq!(resp.structured_contents[0]["_tool_name"], "place_order");
    assert!(resp.message.content.contains("order"));

    // Ambiguous / unknown dish
    let resp_unknown = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "order_sess", "order pizza", None).await;
    assert!(resp_unknown.message.content.contains("Did you mean Paneer Tikka"));
}

#[tokio::test]
async fn test_scenario_s11_order_allergy_conflict() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    let resp = handle_scripted_chat(
        &pool,
        &session_mgr,
        clock.as_ref(),
        tz,
        "conflict_sess",
        "order the satay, I'm allergic to peanuts",
        None,
    )
    .await;
    assert_eq!(resp.structured_contents.len(), 1);
    assert_eq!(resp.structured_contents[0]["_tool_name"], "place_order");
    assert_eq!(resp.structured_contents[0]["status"], "allergen_conflict");
    assert!(resp.message.content.contains("Allergen safety conflict"));
}

#[tokio::test]
async fn test_scenario_s12_memory_flow() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();
    let session_id = "mem_sess_1";

    // Turn 1: "remember I'm allergic to peanuts" -> asks consent
    let resp1 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "remember I'm allergic to peanuts", None).await;
    assert!(resp1.message.content.contains("Would you like me to remember"));
    assert!(resp1.structured_contents.is_empty(), "Consent turn invokes no tool");

    // Turn 2: "yes" -> enables memory and saves preference
    let resp2 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "yes", None).await;
    assert!(resp2.message.content.contains("saved"));

    // Turn 3: "what's my usual order"
    let sess_state = session_mgr.get_or_create(session_id).await;
    let token = sess_state.profile_token.as_deref();
    assert!(token.is_some(), "Profile token must be saved server-side in session");

    let resp3 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "what's my usual order", token).await;
    assert_eq!(resp3.structured_contents.len(), 1);
    assert_eq!(resp3.structured_contents[0]["_tool_name"], "get_usual_order");

    // Turn 4: "what do you remember about me"
    let resp4 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "what do you remember about me", token).await;
    assert_eq!(resp4.structured_contents.len(), 1);
    assert_eq!(resp4.structured_contents[0]["_tool_name"], "get_profile");

    // Ensure token is NEVER in response text
    assert!(!resp4.message.content.contains("tt_prof_"));
}

#[tokio::test]
async fn test_scenario_s13_cancel_destructive_confirmation() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();
    let session_id = "cancel_sess";

    // Turn 1: Request cancel -> asks confirmation
    let resp1 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "cancel my booking ABC47Z under Ravi", None).await;
    assert!(resp1.message.content.contains("Are you sure you want to cancel"));
    assert!(resp1.structured_contents.is_empty());

    // Turn 2: "yes" -> executes cancel_booking tool
    let resp2 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "yes", None).await;
    assert_eq!(resp2.structured_contents.len(), 1);
    assert_eq!(resp2.structured_contents[0]["_tool_name"], "cancel_booking");
    assert!(resp2.message.content.contains("No active reservation was found"));
}

#[tokio::test]
async fn test_scenario_s14_forget_destructive_confirmation() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();
    let session_id = "forget_sess";

    // Enable memory first to have profile token
    let _ = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "remember I'm allergic to peanuts", None).await;
    let _ = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "yes", None).await;

    // Turn 1: "forget me" -> asks confirmation
    let resp1 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "forget me", None).await;
    assert!(resp1.message.content.contains("Are you sure you want me to forget"));

    // Turn 2: "yes" -> calls forget_me tool
    let sess_state = session_mgr.get_or_create(session_id).await;
    let token = sess_state.profile_token.as_deref();

    let resp2 = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, session_id, "yes", token).await;
    assert_eq!(resp2.structured_contents.len(), 1);
    assert_eq!(resp2.structured_contents[0]["_tool_name"], "forget_me");
    assert!(resp2.message.content.contains("forgotten") || resp2.message.content.contains("scrubbed"));
}

#[tokio::test]
async fn test_scenario_s15_goodbye_and_unknown() {
    let pool = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new(10000, 50000));
    let clock = test_clock();
    let tz: Tz = "Asia/Kolkata".parse().unwrap();

    // S15: Goodbye
    let resp_bye = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "bye_sess", "thank you", None).await;
    assert!(resp_bye.structured_contents.is_empty());
    assert!(resp_bye.message.content.contains("welcome"));

    // Unknown input
    let resp_unk = handle_scripted_chat(&pool, &session_mgr, clock.as_ref(), tz, "unk_sess", "random gibberish string xyz123", None).await;
    assert!(resp_unk.structured_contents.is_empty(), "Unknown input must never call a tool");
    assert!(resp_unk.message.content.contains("Try saying"));
}
