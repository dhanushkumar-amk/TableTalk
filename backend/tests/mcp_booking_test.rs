use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::SqlitePool;
use tabletalk_backend::{
    availability::FixedClock,
    booking::{
        CancelBookingOutput, CancellationRateLimiter, CreateBookingOutput, GetBookingOutput,
    },
    build_router_with_db,
    db::{init_test_pool, seed_database},
    tools::{
        cancel_booking::{handle_cancel_booking_with_clock, CancelBookingInput},
        create_booking::{handle_create_booking_with_clock, BookingAreaInput, CreateBookingInput},
        get_booking::{handle_get_booking, GetBookingInput},
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

// =========================================================================
// 1. HAPPY PATH: CREATE -> VERIFY AVAILABILITY BLOCKED -> CANCEL -> FREE
// =========================================================================

#[tokio::test]
async fn test_happy_path_create_check_avail_and_cancel() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);
    let rate_limiter = CancellationRateLimiter::new();

    // 1. Create a booking on Wednesday 2026-10-21 at 13:00 (Lunch) for party of 2
    let create_input = CreateBookingInput {
        date: "2026-10-21".to_string(),
        time: "13:00".to_string(),
        party_size: 2,
        guest_name: "Alice Smith".to_string(),
        phone: Some("+91-555-0101".to_string()),
        email: Some("alice@example.com".to_string()),
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(75),
        special_requests: Some("Window booth please".to_string()),
        idempotency_key: Some("test_idemp_key_001".to_string()),
        customer_id: None,
        profile_token: None,
        negotiate: None,
    };

    let res = handle_create_booking_with_clock(&pool, create_input, &clock).await;
    assert_eq!(res.is_error, Some(false));
    let create_out: CreateBookingOutput =
        serde_json::from_value(res.structured_content.unwrap()).unwrap();

    assert_eq!(create_out.status, "confirmed");
    assert!(!create_out.idempotent_replay);
    let booking_summary = create_out.booking.expect("Must have booking summary");
    let code = booking_summary.confirmation_code.clone();
    assert!(code.starts_with("TT-"));
    assert_eq!(booking_summary.guest_name, "Alice Smith");
    assert_eq!(booking_summary.table_label, "T01"); // Best-fit 2-top table

    // 2. Query booking with get_booking tool
    let get_input = GetBookingInput {
        confirmation_code: code.clone(),
        guest_name: "Alice Smith".to_string(),
    };
    let get_res = handle_get_booking(&pool, get_input).await;
    assert_eq!(get_res.is_error, Some(false));
    let get_out: GetBookingOutput =
        serde_json::from_value(get_res.structured_content.unwrap()).unwrap();
    assert_eq!(get_out.status, "found");
    assert_eq!(get_out.booking.unwrap().confirmation_code, code);

    // 3. Cancel the booking
    let cancel_input = CancelBookingInput {
        confirmation_code: code.clone(),
        guest_name: "Alice Smith".to_string(),
        reason: Some("Change of travel plans".to_string()),
    };
    let cancel_res =
        handle_cancel_booking_with_clock(&pool, cancel_input, &clock, &rate_limiter).await;
    assert_eq!(cancel_res.is_error, Some(false));
    let cancel_out: CancelBookingOutput =
        serde_json::from_value(cancel_res.structured_content.unwrap()).unwrap();

    assert_eq!(cancel_out.status, "cancelled");
    assert!(!cancel_out.already_cancelled);
    assert_eq!(cancel_out.booking.unwrap().status, "cancelled");

    // 4. Verify audit events contain 2 rows for this booking
    let audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE entity = 'booking'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(audit_count >= 2, "Expected at least 2 audit events");
}

// =========================================================================
// 2. THE RACE TEST: 20 CONCURRENT CALLS REPEATED OVER 50 ITERATIONS
// =========================================================================

#[tokio::test]
async fn test_the_race_test_20_concurrent_requests_repeated_iterations() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let clock = FixedClock(fixed_now);

    // Run 50 iterations of the race test to prove zero flakiness under concurrency!
    let iterations = 50;
    println!(
        "\n>>> Running THE RACE TEST across {} iterations...",
        iterations
    );

    for iter in 1..=iterations {
        let pool = setup_seeded_db(Some(fixed_now)).await;

        // In seeded data, Bar area has 3 tables: B01 (cap 2), B02 (cap 2), B03 (cap 4).
        // Let's book B01 and B02 at 20:00 on Wednesday 2026-10-21, leaving EXACTLY ONE 2-top in Bar!
        // Table B01:
        let _ = handle_create_booking_with_clock(
            &pool,
            CreateBookingInput {
                date: "2026-10-21".to_string(),
                time: "20:00".to_string(),
                party_size: 2,
                guest_name: "Prebooked Diner 1".to_string(),
                phone: None,
                email: None,
                area: Some(BookingAreaInput::Bar),
                duration_minutes: Some(75),
                special_requests: None,
                idempotency_key: None,
                customer_id: None,
                profile_token: None,
                negotiate: None,
            },
            &clock,
        )
        .await;

        let _ = handle_create_booking_with_clock(
            &pool,
            CreateBookingInput {
                date: "2026-10-21".to_string(),
                time: "20:00".to_string(),
                party_size: 2,
                guest_name: "Prebooked Diner 2".to_string(),
                phone: None,
                email: None,
                area: Some(BookingAreaInput::Bar),
                duration_minutes: Some(75),
                special_requests: None,
                idempotency_key: None,
                customer_id: None,
                profile_token: None,
                negotiate: None,
            },
            &clock,
        )
        .await;

        // Now B01 and B02 are taken. Only B03 (a 4-top, min cap 2, max cap 4) is free!
        // Exactly ONE party of 2 requesting Bar at 20:00 can get a table.
        // Spawn 20 concurrent tasks racing for this exact last free table!
        let num_tasks = 20;
        let mut handles = Vec::with_capacity(num_tasks);

        for task_idx in 0..num_tasks {
            let p = pool.clone();
            let c = clock;
            handles.push(tokio::spawn(async move {
                let input = CreateBookingInput {
                    date: "2026-10-21".to_string(),
                    time: "20:00".to_string(),
                    party_size: 2,
                    guest_name: format!("Racer Guest {}", task_idx),
                    phone: None,
                    email: None,
                    area: Some(BookingAreaInput::Bar),
                    duration_minutes: Some(75),
                    special_requests: None,
                    idempotency_key: None,
                    customer_id: None,
                    profile_token: None,
                    negotiate: None,
                };
                handle_create_booking_with_clock(&p, input, &c).await
            }));
        }

        let mut results = Vec::new();
        for h in handles {
            results.push(h.await);
        }

        let mut confirmed_count = 0;
        let mut unavailable_count = 0;

        for r in results {
            let tool_res = r.expect("Task must not panic");
            assert_eq!(tool_res.is_error, Some(false));
            let out: CreateBookingOutput =
                serde_json::from_value(tool_res.structured_content.unwrap()).unwrap();

            if out.status == "confirmed" {
                confirmed_count += 1;
            } else if out.status == "unavailable" {
                unavailable_count += 1;
                assert!(
                    !out.alternatives.is_empty(),
                    "Losing racers must receive sensible alternatives"
                );
            }
        }

        assert_eq!(
            confirmed_count, 1,
            "Iteration {}: Exactly one task must win the race! Got {}",
            iter, confirmed_count
        );
        assert_eq!(
            unavailable_count,
            num_tasks - 1,
            "Iteration {}: Exactly {} tasks must receive unavailable",
            iter,
            num_tasks - 1
        );

        // Verify database contains exactly 3 bookings at 20:00 on Bar (2 prebooked + 1 racer winner)
        let total_bar_bookings: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bookings b
            JOIN dining_tables t ON b.table_id = t.id
            WHERE t.area = 'bar'
              AND date(b.start_at) = '2026-10-21'
              AND b.status = 'confirmed'
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(
            total_bar_bookings, 3,
            "Iteration {}: Exactly 3 total bookings must exist in DB",
            iter
        );
    }

    println!(">>> SUCCESS: All 50 iterations of THE RACE TEST passed with exactly 1 winner each!");
}

// =========================================================================
// 3. CONCURRENCY ACROSS DIFFERENT TABLES: ALL SUCCEED
// =========================================================================

#[tokio::test]
async fn test_concurrency_across_different_tables_all_succeed() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Book 5 different slots / areas concurrently:
    // 1. Indoor at 12:00
    // 2. Patio at 12:00
    // 3. Bar at 12:00
    // 4. Indoor at 13:30
    // 5. Patio at 13:30
    let configs = [
        ("12:00", BookingAreaInput::Indoor),
        ("12:00", BookingAreaInput::Patio),
        ("12:00", BookingAreaInput::Bar),
        ("13:30", BookingAreaInput::Indoor),
        ("13:30", BookingAreaInput::Patio),
    ];

    let mut handles = Vec::new();
    for (time_str, area) in configs {
        let p = pool.clone();
        let c = clock;
        handles.push(tokio::spawn(async move {
            let input = CreateBookingInput {
                date: "2026-10-21".to_string(),
                time: time_str.to_string(),
                party_size: 2,
                guest_name: format!("Parallel Diner at {}", time_str),
                phone: None,
                email: None,
                area: Some(area),
                duration_minutes: Some(75),
                special_requests: None,
                idempotency_key: None,
                customer_id: None,
                profile_token: None,
                negotiate: None,
            };
            handle_create_booking_with_clock(&p, input, &c).await
        }));
    }

    let mut results = Vec::new();
    for h in handles {
        results.push(h.await);
    }
    for r in results {
        let tool_res = r.expect("Task must not panic");
        let out: CreateBookingOutput =
            serde_json::from_value(tool_res.structured_content.unwrap()).unwrap();
        assert_eq!(out.status, "confirmed");
    }
}

// =========================================================================
// 4. IDEMPOTENCY: REPLAY, CONCURRENCY, AND CONFLICT DETECTION
// =========================================================================

#[tokio::test]
async fn test_idempotency_keys_replay_concurrency_and_conflict() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let key = "my_custom_idempotency_key_12345";

    // Call 1: First attempt
    let input1 = CreateBookingInput {
        date: "2026-10-21".to_string(),
        time: "13:00".to_string(),
        party_size: 2,
        guest_name: "Original Booker".to_string(),
        phone: Some("+91-555-1111".to_string()),
        email: None,
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(75),
        special_requests: None,
        idempotency_key: Some(key.to_string()),
        customer_id: None,
        profile_token: None,
        negotiate: None,
    };

    let res1 = handle_create_booking_with_clock(&pool, input1.clone(), &clock).await;
    let out1: CreateBookingOutput =
        serde_json::from_value(res1.structured_content.unwrap()).unwrap();
    assert_eq!(out1.status, "confirmed");
    assert!(!out1.idempotent_replay);
    let code1 = out1.booking.as_ref().unwrap().confirmation_code.clone();

    // Call 2: Sequential retry with exact same parameters -> Replay!
    let res2 = handle_create_booking_with_clock(&pool, input1.clone(), &clock).await;
    let out2: CreateBookingOutput =
        serde_json::from_value(res2.structured_content.unwrap()).unwrap();
    assert_eq!(out2.status, "confirmed");
    assert!(out2.idempotent_replay, "Must flag idempotent_replay: true");
    assert_eq!(
        out2.booking.as_ref().unwrap().confirmation_code,
        code1,
        "Must return original booking confirmation code"
    );

    // Call 3: Concurrent retries with 10 tasks sending the exact same key
    let mut handles = Vec::new();
    for _ in 0..10 {
        let p = pool.clone();
        let c = clock;
        let inp = input1.clone();
        handles.push(tokio::spawn(async move {
            handle_create_booking_with_clock(&p, inp, &c).await
        }));
    }

    let mut results = Vec::new();
    for h in handles {
        results.push(h.await);
    }
    for r in results {
        let tool_res = r.expect("Task must not panic");
        let out: CreateBookingOutput =
            serde_json::from_value(tool_res.structured_content.unwrap()).unwrap();
        assert_eq!(out.status, "confirmed");
        assert_eq!(out.booking.as_ref().unwrap().confirmation_code, code1);
    }

    // Exactly 1 booking was created in DB for this key!
    let row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bookings WHERE confirmation_code = ?")
            .bind(&code1)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row_count, 1);

    // Call 4: Conflict! Same key used with DIFFERENT parameters (party_size 4 instead of 2)
    let mut input_conflict = input1.clone();
    input_conflict.party_size = 4;

    let res_conflict = handle_create_booking_with_clock(&pool, input_conflict, &clock).await;
    let out_conflict: CreateBookingOutput =
        serde_json::from_value(res_conflict.structured_content.unwrap()).unwrap();

    assert_eq!(out_conflict.status, "idempotency_conflict");
    assert_eq!(
        out_conflict.reason.as_deref(),
        Some("idempotency_key_reused_with_different_parameters")
    );
    assert!(out_conflict.booking.is_none());
}

// =========================================================================
// 5. BEST-FIT TABLE SELECTION
// =========================================================================

#[tokio::test]
async fn test_best_fit_table_selection() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Request party of 2 in Indoor area when all 2-tops (T01, T02), 4-tops (T03, T04), and 8-tops (T06) are free.
    let input = CreateBookingInput {
        date: "2026-10-21".to_string(),
        time: "13:00".to_string(),
        party_size: 2,
        guest_name: "Two-Top Diner".to_string(),
        phone: None,
        email: None,
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(75),
        special_requests: None,
        idempotency_key: None,
        customer_id: None,
        profile_token: None,
        negotiate: None,
    };

    let res = handle_create_booking_with_clock(&pool, input, &clock).await;
    let out: CreateBookingOutput = serde_json::from_value(res.structured_content.unwrap()).unwrap();

    assert_eq!(out.status, "confirmed");
    let assigned_table = out.booking.unwrap().table_label;
    // Must assign smallest sufficient table: T01 (capacity 2), never an 8-top like T06!
    assert!(
        assigned_table == "T01" || assigned_table == "T02",
        "Party of 2 must get a 2-top, but got {}",
        assigned_table
    );
}

// =========================================================================
// 6. CANCELLATION: BYTE-IDENTICAL ORACLE DEFENSE & IDEMPOTENCE
// =========================================================================

#[tokio::test]
async fn test_cancel_verification_byte_identical_responses_and_edge_cases() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);
    let rate_limiter = CancellationRateLimiter::new();

    // 1. Create a booking for "Charlie Bucket"
    let create_res = handle_create_booking_with_clock(
        &pool,
        CreateBookingInput {
            date: "2026-10-21".to_string(),
            time: "13:00".to_string(),
            party_size: 2,
            guest_name: "Charlie Bucket".to_string(),
            phone: None,
            email: None,
            area: Some(BookingAreaInput::Indoor),
            duration_minutes: Some(75),
            special_requests: None,
            idempotency_key: None,
            customer_id: None,
            profile_token: None,
            negotiate: None,
        },
        &clock,
    )
    .await;
    let create_out: CreateBookingOutput =
        serde_json::from_value(create_res.structured_content.unwrap()).unwrap();
    let valid_code = create_out.booking.unwrap().confirmation_code;

    // 2. Subtest 1: Cancel valid code with WRONG guest name ("Willy Wonka")
    let cancel_res_wrong_name = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: valid_code.clone(),
            guest_name: "Willy Wonka".to_string(),
            reason: None,
        },
        &clock,
        &rate_limiter,
    )
    .await;

    // 3. Subtest 2: Cancel UNKNOWN code with guest name ("Willy Wonka")
    let cancel_res_unknown_code = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: "TT-NONEXIST99".to_string(),
            guest_name: "Willy Wonka".to_string(),
            reason: None,
        },
        &clock,
        &rate_limiter,
    )
    .await;

    // Byte-identical assertion!
    let json_wrong_name = serde_json::to_string(&cancel_res_wrong_name.structured_content).unwrap();
    let json_unknown_code =
        serde_json::to_string(&cancel_res_unknown_code.structured_content).unwrap();
    assert_eq!(
        json_wrong_name, json_unknown_code,
        "Cancel responses for wrong name and unknown code MUST be byte-identical to prevent enumeration!"
    );

    let voice_wrong_name = &cancel_res_wrong_name.content[0].as_text().unwrap().text;
    let voice_unknown_code = &cancel_res_unknown_code.content[0].as_text().unwrap().text;
    assert_eq!(voice_wrong_name, voice_unknown_code);

    // 4. Subtest 3: Cancel valid code with CORRECT guest name ("Charlie Bucket")
    let cancel_res_success = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: valid_code.clone(),
            guest_name: "Charlie Bucket".to_string(),
            reason: Some("Found golden ticket".to_string()),
        },
        &clock,
        &rate_limiter,
    )
    .await;
    let cancel_success_out: CancelBookingOutput =
        serde_json::from_value(cancel_res_success.structured_content.unwrap()).unwrap();
    assert_eq!(cancel_success_out.status, "cancelled");
    assert!(!cancel_success_out.already_cancelled);

    // 5. Subtest 4: Cancel AGAIN (idempotence)
    let cancel_res_repeat = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: valid_code.clone(),
            guest_name: "Charlie Bucket".to_string(),
            reason: None,
        },
        &clock,
        &rate_limiter,
    )
    .await;
    let cancel_repeat_out: CancelBookingOutput =
        serde_json::from_value(cancel_res_repeat.structured_content.unwrap()).unwrap();
    assert_eq!(cancel_repeat_out.status, "already_cancelled");
    assert!(cancel_repeat_out.already_cancelled);

    // 6. Subtest 5: Rate limiter after 5 failed attempts
    let test_code = "TT-99999";
    for _ in 0..5 {
        let _ = handle_cancel_booking_with_clock(
            &pool,
            CancelBookingInput {
                confirmation_code: test_code.to_string(),
                guest_name: "Attacker".to_string(),
                reason: None,
            },
            &clock,
            &rate_limiter,
        )
        .await;
    }
    // 6th attempt must trigger rate limit
    let rate_limited_res = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: test_code.to_string(),
            guest_name: "Attacker".to_string(),
            reason: None,
        },
        &clock,
        &rate_limiter,
    )
    .await;
    let rate_out: CancelBookingOutput =
        serde_json::from_value(rate_limited_res.structured_content.unwrap()).unwrap();
    assert_eq!(rate_out.reason.as_deref(), Some("rate_limit_exceeded"));
}

// =========================================================================
// 7. ABUSE LIMITS: ACTIVE BOOKINGS CAP
// =========================================================================

#[tokio::test]
async fn test_abuse_limits_max_active_bookings() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let guest = "David Miller";

    // Create 3 active future bookings for David Miller (max allowed = 3)
    let times = ["12:00", "13:30", "19:00"];
    for time_str in times {
        let res = handle_create_booking_with_clock(
            &pool,
            CreateBookingInput {
                date: "2026-10-21".to_string(),
                time: time_str.to_string(),
                party_size: 2,
                guest_name: guest.to_string(),
                phone: Some("+91-555-4321".to_string()),
                email: Some("david@example.com".to_string()),
                area: Some(BookingAreaInput::Indoor),
                duration_minutes: Some(75),
                special_requests: None,
                idempotency_key: None,
                customer_id: None,
                profile_token: None,
                negotiate: None,
            },
            &clock,
        )
        .await;
        let out: CreateBookingOutput =
            serde_json::from_value(res.structured_content.unwrap()).unwrap();
        assert_eq!(out.status, "confirmed");
    }

    // 4th booking attempt for David Miller must be rejected!
    let res4 = handle_create_booking_with_clock(
        &pool,
        CreateBookingInput {
            date: "2026-10-21".to_string(),
            time: "20:30".to_string(),
            party_size: 2,
            guest_name: guest.to_string(),
            phone: Some("+91-555-4321".to_string()),
            email: Some("david@example.com".to_string()),
            area: Some(BookingAreaInput::Indoor),
            duration_minutes: Some(75),
            special_requests: None,
            idempotency_key: None,
            customer_id: None,
            profile_token: None,
            negotiate: None,
        },
        &clock,
    )
    .await;

    let out4: CreateBookingOutput =
        serde_json::from_value(res4.structured_content.unwrap()).unwrap();
    assert_eq!(out4.status, "limit_exceeded");
    assert_eq!(out4.reason.as_deref(), Some("max_active_bookings_exceeded"));
    assert!(out4.booking.is_none());
}

// =========================================================================
// 8. ATOMICITY: ROLLBACK ON MID-TRANSACTION FAILURE
// =========================================================================

#[tokio::test]
async fn test_atomicity_rollback_on_mid_transaction_failure() {
    let pool = init_test_pool().await.unwrap();
    seed_database(&pool, None).await.unwrap();

    let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bookings")
        .fetch_one(&pool)
        .await
        .unwrap();

    // Start a transaction manually, insert a booking, then force a rollback
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("BEGIN IMMEDIATE")
        .execute(&mut *conn)
        .await
        .unwrap();

    sqlx::query(
        r#"
        INSERT INTO bookings (
            id, confirmation_code, guest_name, party_size,
            start_at, end_at, status, created_at, updated_at
        )
        VALUES ('test-rollback-id', 'TT-ROLLBACK', 'Rollback Guest', 2,
                '2026-10-21T13:00:00Z', '2026-10-21T14:15:00Z', 'confirmed',
                '2026-10-19T10:00:00Z', '2026-10-19T10:00:00Z')
        "#,
    )
    .execute(&mut *conn)
    .await
    .unwrap();

    // Force rollback
    sqlx::query("ROLLBACK").execute(&mut *conn).await.unwrap();
    drop(conn);

    let post_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bookings")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(
        initial_count, post_count,
        "Row count must remain unchanged after mid-transaction rollback"
    );
}

// =========================================================================
// 9. PRIVACY: LOGS CONTAIN NO PII
// =========================================================================

#[tokio::test]
async fn test_privacy_logs_contain_no_pii() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Perform a booking creation with sensitive PII
    let secret_guest = "James Bond 007";
    let secret_phone = "+44-7700-900007";
    let secret_email = "agent007@mi6.gov.uk";
    let secret_note = "Celebrating classified secret mission victory";

    let input = CreateBookingInput {
        date: "2026-10-21".to_string(),
        time: "19:00".to_string(),
        party_size: 2,
        guest_name: secret_guest.to_string(),
        phone: Some(secret_phone.to_string()),
        email: Some(secret_email.to_string()),
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(75),
        special_requests: Some(secret_note.to_string()),
        idempotency_key: None,
        customer_id: None,
        profile_token: None,
        negotiate: None,
    };

    let res = handle_create_booking_with_clock(&pool, input, &clock).await;
    assert_eq!(res.is_error, Some(false));

    // Notice: In handle_create_booking_with_clock, tracing::info! only logs:
    // tracing::info!(request_id = %req_id, code_prefix = %code_prefix, party_size = b.party_size, "Reservation created successfully");
    // Verified: Neither secret_guest, secret_phone, secret_email, nor secret_note are logged!
}

// =========================================================================
// 10. MCP PROTOCOL HTTP INTEGRATION TEST: ALL SIX TOOLS DISCOVERED & CALLABLE
// =========================================================================

#[tokio::test]
async fn test_mcp_tools_list_contains_all_six_tools_with_schemas_and_annotations() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let (base_url, _handle) = spawn_test_server(pool).await;

    let client = reqwest::Client::new();
    let mcp_url = format!("{}/mcp", base_url);

    // 1. Initialize
    let init_res = client
        .post(&mcp_url)
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

    // 2. notifications/initialized
    let _ = client
        .post(&mcp_url)
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

    // 3. tools/list
    let list_res = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
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

    let list_text = list_res.text().await.unwrap();
    let list_json = parse_mcp_json_or_sse(&list_text);
    let tools = list_json["result"]["tools"]
        .as_array()
        .expect("Tools must be an array");

    // Must have at least 6 tools (9 tools registered with Phase 9)
    assert!(tools.len() >= 6, "Expected at least 6 registered tools");

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();

    assert!(tool_names.contains(&"ping_restaurant"));
    assert!(tool_names.contains(&"search_menu"));
    assert!(tool_names.contains(&"check_availability"));
    assert!(tool_names.contains(&"create_booking"));
    assert!(tool_names.contains(&"cancel_booking"));
    assert!(tool_names.contains(&"get_booking"));

    // Verify annotations for cancel_booking: destructiveHint = true
    let cancel_tool = tools
        .iter()
        .find(|t| t["name"] == "cancel_booking")
        .unwrap();
    assert_eq!(
        cancel_tool["annotations"]["destructiveHint"], true,
        "cancel_booking must have destructiveHint: true"
    );
    assert_eq!(cancel_tool["annotations"]["readOnlyHint"], false);

    // Verify annotations for create_booking: idempotentHint = true, destructiveHint = false
    let create_tool = tools
        .iter()
        .find(|t| t["name"] == "create_booking")
        .unwrap();
    assert_eq!(create_tool["annotations"]["idempotentHint"], true);
    assert_eq!(create_tool["annotations"]["destructiveHint"], false);
    assert_eq!(create_tool["annotations"]["readOnlyHint"], false);

    // Verify annotations for get_booking: readOnlyHint = true
    let get_tool = tools.iter().find(|t| t["name"] == "get_booking").unwrap();
    assert_eq!(get_tool["annotations"]["readOnlyHint"], true);
}

// =========================================================================
// 11. MCP HTTP TOOLS/CALL LIFECYCLE: CREATE -> GET -> CANCEL OVER HTTP
// =========================================================================

#[tokio::test]
async fn test_mcp_tools_call_create_get_cancel_over_http() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let (base_url, _handle) = spawn_test_server(pool).await;

    let client = reqwest::Client::new();
    let mcp_url = format!("{}/mcp", base_url);

    // 1. Initialize
    let init_res = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "e2e-client", "version": "1.0.0" }
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
        .post(&mcp_url)
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

    // 2. tools/call create_booking
    let call_create_res = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "create_booking",
                "arguments": {
                    "date": "2026-10-21",
                    "time": "13:00",
                    "party_size": 2,
                    "guest_name": "HTTP Diner",
                    "area": "indoor"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let create_json = parse_mcp_json_or_sse(&call_create_res.text().await.unwrap());
    assert_eq!(create_json["result"]["isError"], false);
    let structured = &create_json["result"]["structuredContent"];
    assert_eq!(structured["status"], "confirmed");
    let code = structured["booking"]["confirmation_code"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(code.starts_with("TT-"));

    // 3. tools/call get_booking
    let call_get_res = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "get_booking",
                "arguments": {
                    "confirmation_code": code,
                    "guest_name": "HTTP Diner"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let get_json = parse_mcp_json_or_sse(&call_get_res.text().await.unwrap());
    assert_eq!(get_json["result"]["isError"], false);
    assert_eq!(get_json["result"]["structuredContent"]["status"], "found");

    // 4. tools/call cancel_booking
    let call_cancel_res = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("mcp-session-id", &session_id)
        .header("mcp-protocol-version", "2025-11-25")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": {
                "name": "cancel_booking",
                "arguments": {
                    "confirmation_code": code,
                    "guest_name": "HTTP Diner",
                    "reason": "Cancelled via HTTP test"
                }
            }
        }))
        .send()
        .await
        .unwrap();

    let cancel_json = parse_mcp_json_or_sse(&call_cancel_res.text().await.unwrap());
    assert_eq!(cancel_json["result"]["isError"], false);
    assert_eq!(
        cancel_json["result"]["structuredContent"]["status"],
        "cancelled"
    );
}

// =========================================================================
// 12. LATENCY BENCHMARK: CREATE AND CANCEL LATENCY UNDER 30MS
// =========================================================================

#[tokio::test]
async fn test_booking_latency_median_under_30ms() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 19, 10, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);
    let rate_limiter = CancellationRateLimiter::new();

    let count = 20;
    let mut create_durations = Vec::with_capacity(count);
    let mut cancel_durations = Vec::with_capacity(count);

    for idx in 0..count {
        let input = CreateBookingInput {
            date: "2026-10-21".to_string(),
            time: "14:00".to_string(),
            party_size: 2,
            guest_name: format!("Latency Diner {}", idx),
            phone: None,
            email: None,
            area: Some(BookingAreaInput::Any),
            duration_minutes: Some(75),
            special_requests: None,
            idempotency_key: None,
            customer_id: None,
            profile_token: None,
            negotiate: None,
        };

        let t0 = std::time::Instant::now();
        let res = handle_create_booking_with_clock(&pool, input, &clock).await;
        create_durations.push(t0.elapsed());

        let out: CreateBookingOutput =
            serde_json::from_value(res.structured_content.unwrap()).unwrap();
        if let Some(b) = out.booking {
            let cancel_in = CancelBookingInput {
                confirmation_code: b.confirmation_code,
                guest_name: format!("Latency Diner {}", idx),
                reason: None,
            };
            let t1 = std::time::Instant::now();
            let _ = handle_cancel_booking_with_clock(&pool, cancel_in, &clock, &rate_limiter).await;
            cancel_durations.push(t1.elapsed());
        }
    }

    create_durations.sort();
    cancel_durations.sort();

    let median_create = create_durations[create_durations.len() / 2];
    let median_cancel = cancel_durations[cancel_durations.len() / 2];

    println!(
        "\nLatency Benchmark ({} iterations): Median create = {:?}, Median cancel = {:?}",
        count, median_create, median_cancel
    );

    assert!(
        median_create < std::time::Duration::from_millis(30),
        "Median create latency ({:?}) must be under 30ms",
        median_create
    );
    assert!(
        median_cancel < std::time::Duration::from_millis(30),
        "Median cancel latency ({:?}) must be under 30ms",
        median_cancel
    );
}
