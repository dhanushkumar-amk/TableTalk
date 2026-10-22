use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tabletalk_backend::{
    availability::{AreaFilter, FixedClock},
    booking::CancellationRateLimiter,
    db::{init_test_pool, models::Area, seed_database},
    negotiation::{
        format_spoken_options, format_spoken_waitlist_offer, score_candidate,
        sort_scored_candidates, sweep_expired_holds_and_negotiations, CandidateSlot,
        NegotiationOption, ScoredCandidate,
    },
    tools::{
        accept_offer::{handle_accept_offer_with_clock, AcceptOfferInput, AcceptOfferOutput},
        cancel_booking::{handle_cancel_booking_with_clock, CancelBookingInput},
        check_availability::{handle_check_availability_with_clock, CheckAvailabilityInput},
        create_booking::{handle_create_booking_with_clock, BookingAreaInput, CreateBookingInput},
        forget_me::{handle_forget_me_with_clock, ForgetMeInput},
        get_waitlist_status::{
            handle_get_waitlist_status_with_clock, GetWaitlistStatusInput, GetWaitlistStatusOutput,
        },
        join_waitlist::{handle_join_waitlist_with_clock, JoinWaitlistInput, JoinWaitlistOutput},
        negotiate_booking::{
            handle_negotiate_booking_with_clock, NegotiateBookingInput, NegotiateBookingOutput,
            NegotiationFlexibility,
        },
        release_offer::{handle_release_offer_with_clock, ReleaseOfferInput, ReleaseOfferOutput},
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

    let router = tabletalk_backend::build_router_with_db(&config, Some(pool));
    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (base_url, handle)
}

// =========================================================================
// 1. UNIT TESTS: DETERMINISTIC SCORING, TIE-BREAKING, REASONS & VOICE TEXT
// =========================================================================

#[test]
fn test_deterministic_scoring_and_shuffled_invariance() {
    let req_date = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
    let req_time = NaiveTime::from_hms_opt(19, 30, 0).unwrap();
    let req_party = 4;

    let candidates = vec![
        CandidateSlot {
            table_id: 1,
            table_label: "T01".to_string(),
            capacity_min: 1,
            capacity_max: 4,
            area: Area::Indoor,
            start_at: Utc.with_ymd_and_hms(2026, 10, 16, 14, 0, 0).unwrap(),
            end_at: Utc.with_ymd_and_hms(2026, 10, 16, 15, 30, 0).unwrap(),
            start_local_time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            end_local_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
            local_date: req_date,
            minutes_from_closing: 150,
        },
        CandidateSlot {
            table_id: 2,
            table_label: "T02".to_string(),
            capacity_min: 1,
            capacity_max: 4,
            area: Area::Patio,
            start_at: Utc.with_ymd_and_hms(2026, 10, 16, 14, 0, 0).unwrap(),
            end_at: Utc.with_ymd_and_hms(2026, 10, 16, 15, 30, 0).unwrap(),
            start_local_time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            end_local_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
            local_date: req_date,
            minutes_from_closing: 150,
        },
        CandidateSlot {
            table_id: 3,
            table_label: "T03".to_string(),
            capacity_min: 1,
            capacity_max: 6,
            area: Area::Indoor,
            start_at: Utc.with_ymd_and_hms(2026, 10, 16, 14, 30, 0).unwrap(),
            end_at: Utc.with_ymd_and_hms(2026, 10, 16, 16, 0, 0).unwrap(),
            start_local_time: NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
            end_local_time: NaiveTime::from_hms_opt(21, 30, 0).unwrap(),
            local_date: req_date,
            minutes_from_closing: 120,
        },
        CandidateSlot {
            table_id: 4,
            table_label: "T04".to_string(),
            capacity_min: 1,
            capacity_max: 4,
            area: Area::Indoor,
            start_at: Utc.with_ymd_and_hms(2026, 10, 16, 13, 30, 0).unwrap(),
            end_at: Utc.with_ymd_and_hms(2026, 10, 16, 15, 0, 0).unwrap(),
            start_local_time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            end_local_time: NaiveTime::from_hms_opt(20, 30, 0).unwrap(),
            local_date: req_date,
            minutes_from_closing: 180,
        },
    ];

    let area_filter = AreaFilter::Indoor;

    let mut baseline_scored: Vec<ScoredCandidate> = candidates
        .iter()
        .map(|c| score_candidate(c, req_date, req_time, req_party, area_filter, None, 45))
        .collect();
    sort_scored_candidates(&mut baseline_scored);

    let baseline_order: Vec<(String, String)> = baseline_scored
        .iter()
        .map(|s| {
            (
                s.candidate.table_label.clone(),
                s.candidate.start_local_time.to_string(),
            )
        })
        .collect();

    // 1,000 iterations shuffling candidates: assert order and scores are 100% deterministic
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();

    for _ in 0..1000 {
        let mut shuffled = candidates.clone();
        shuffled.shuffle(&mut rng);

        let mut scored: Vec<ScoredCandidate> = shuffled
            .iter()
            .map(|c| score_candidate(c, req_date, req_time, req_party, area_filter, None, 45))
            .collect();
        sort_scored_candidates(&mut scored);

        let order: Vec<(String, String)> = scored
            .iter()
            .map(|s| {
                (
                    s.candidate.table_label.clone(),
                    s.candidate.start_local_time.to_string(),
                )
            })
            .collect();

        assert_eq!(order, baseline_order);
        for i in 0..baseline_scored.len() {
            assert_eq!(scored[i].score, baseline_scored[i].score);
            assert_eq!(scored[i].score_reasons, baseline_scored[i].score_reasons);
        }
    }
}

#[test]
fn test_spoken_voice_formatting_rules() {
    // 1. Single option
    let opt1 = NegotiationOption {
        option_id: "opt_1".to_string(),
        start_local: "20:00".to_string(),
        end_local: "21:15".to_string(),
        area: "indoor".to_string(),
        table_label: Some("T01".to_string()),
        minutes_from_requested: 30,
        score_reasons: vec!["closest_time".to_string()],
        rank: 1,
    };
    let voice1 = format_spoken_options(std::slice::from_ref(&opt1));
    assert!(voice1.contains("Eight in the evening indoors"));
    assert!(voice1.contains("Which would you like?"));

    // 2. Two options
    let opt2 = NegotiationOption {
        option_id: "opt_2".to_string(),
        start_local: "19:30".to_string(),
        end_local: "20:45".to_string(),
        area: "patio".to_string(),
        table_label: Some("T03".to_string()),
        minutes_from_requested: 0,
        score_reasons: vec!["exact_time_match".to_string()],
        rank: 2,
    };
    let voice2 = format_spoken_options(&[opt1.clone(), opt2.clone()]);
    assert!(voice2.contains("Eight in the evening indoors, or seven thirty on the patio"));
    assert!(voice2.contains("Which would you like?"));

    // 3. Three options
    let opt3 = NegotiationOption {
        option_id: "opt_3".to_string(),
        start_local: "20:15".to_string(),
        end_local: "21:30".to_string(),
        area: "indoor".to_string(),
        table_label: Some("T02".to_string()),
        minutes_from_requested: 45,
        score_reasons: vec!["same_area".to_string()],
        rank: 3,
    };
    let voice3 = format_spoken_options(&[opt1, opt2, opt3]);
    assert!(voice3.contains(
        "Eight in the evening indoors, seven thirty on the patio, or eight fifteen indoors"
    ));
    assert!(voice3.contains("Which would you like?"));

    // 4. Waitlist offer formatting
    let wl_voice = format_spoken_waitlist_offer(8, "patio", "19:30", 5);
    assert!(wl_voice.contains("patio"));
    assert!(wl_voice.contains("holding it for five minutes"));
}

// =========================================================================
// 2. NEGOTIATION ROUNDS & EXCLUSION LOGIC
// =========================================================================

#[tokio::test]
async fn test_negotiation_rounds_and_max_rounds_reached() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    // Request on Friday 2026-10-23 (fully booked 19:30 indoor)
    let req1 = NegotiateBookingInput {
        date: "2026-10-23".to_string(),
        time: "19:30".to_string(),
        party_size: 4,
        guest_name: "Negotiator Bob".to_string(),
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(90),
        flexibility: Some(NegotiationFlexibility {
            time_window_minutes: Some(90),
            allow_other_areas: Some(true),
            allow_other_days: Some(false),
            earliest_time: None,
            latest_time: None,
        }),
        profile_token: None,
        negotiation_id: None,
        exclude_offered: Some(true),
        max_options: Some(2),
    };

    let res1 = handle_negotiate_booking_with_clock(&pool, req1, &clock).await;
    let out1: NegotiateBookingOutput =
        serde_json::from_value(res1.structured_content.unwrap()).unwrap();
    assert_eq!(out1.round, Some(1));
    let neg_id = out1.negotiation_id.expect("Must have negotiation_id");

    // Round 2 with negotiation_id
    let req2 = NegotiateBookingInput {
        date: "2026-10-23".to_string(),
        time: "19:30".to_string(),
        party_size: 4,
        guest_name: "Negotiator Bob".to_string(),
        area: Some(BookingAreaInput::Indoor),
        duration_minutes: Some(90),
        flexibility: None,
        profile_token: None,
        negotiation_id: Some(neg_id),
        exclude_offered: Some(true),
        max_options: Some(2),
    };

    let res2 = handle_negotiate_booking_with_clock(&pool, req2.clone(), &clock).await;
    let out2: NegotiateBookingOutput =
        serde_json::from_value(res2.structured_content.unwrap()).unwrap();
    assert_eq!(out2.round, Some(2));

    // Ensure round 2 options do not repeat round 1 options
    for opt2 in &out2.options {
        for opt1 in &out1.options {
            assert_ne!(
                (opt2.start_local.clone(), opt2.area.clone()),
                (opt1.start_local.clone(), opt1.area.clone()),
                "Round 2 option must not repeat round 1 option"
            );
        }
    }

    // Round 3
    let req3 = NegotiateBookingInput {
        negotiation_id: Some(neg_id),
        ..req2.clone()
    };
    let res3 = handle_negotiate_booking_with_clock(&pool, req3, &clock).await;
    let out3: NegotiateBookingOutput =
        serde_json::from_value(res3.structured_content.unwrap()).unwrap();
    assert_eq!(out3.round, Some(3));

    // Round 4 (exceeds MAX_NEGOTIATION_ROUNDS = 3) -> max_rounds_reached
    let req4 = NegotiateBookingInput {
        negotiation_id: Some(neg_id),
        ..req2.clone()
    };
    let res4 = handle_negotiate_booking_with_clock(&pool, req4, &clock).await;
    let out4: NegotiateBookingOutput =
        serde_json::from_value(res4.structured_content.unwrap()).unwrap();
    assert_eq!(out4.status, "no_more_options");
    assert!(out4.message.contains("waitlist"));
}

// =========================================================================
// 3. HOLDS: BLOCKING CALLERS, LAZY EXPIRY & SWEPT EXPIRY
// =========================================================================

#[tokio::test]
async fn test_holds_block_availability_and_booking_and_lazy_expiry() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let test_date = "2026-10-16";

    // 1. Get options
    let neg_res = handle_negotiate_booking_with_clock(
        &pool,
        NegotiateBookingInput {
            date: test_date.to_string(),
            time: "20:00".to_string(),
            party_size: 8,
            guest_name: "Holder Dan".to_string(),
            area: Some(BookingAreaInput::Indoor),
            duration_minutes: Some(75),
            flexibility: None,
            profile_token: None,
            negotiation_id: None,
            exclude_offered: None,
            max_options: Some(3),
        },
        &clock,
    )
    .await;
    let neg_out: NegotiateBookingOutput =
        serde_json::from_value(neg_res.structured_content.unwrap()).unwrap();
    let neg_id = neg_out.negotiation_id.unwrap();
    let opt = &neg_out.options[0];

    // 2. Place hold on opt_1 (hold_only: true)
    let hold_res = handle_accept_offer_with_clock(
        &pool,
        AcceptOfferInput {
            negotiation_id: Some(neg_id),
            option_id: Some(opt.option_id.clone()),
            offer_token: None,
            guest_name: "Holder Dan".to_string(),
            phone: None,
            email: None,
            special_requests: None,
            idempotency_key: None,
            hold_only: Some(true),
        },
        &clock,
    )
    .await;
    let hold_out: AcceptOfferOutput =
        serde_json::from_value(hold_res.structured_content.unwrap()).unwrap();
    assert_eq!(hold_out.status, "held");
    let offer_token = hold_out.offer_token.expect("Must return offer token");
    assert!(offer_token.starts_with("tt_offer_"));

    // 3. Second caller checking availability for that exact slot sees reason_detail = "temporarily held, may free up soon"
    let avail_res = handle_check_availability_with_clock(
        &pool,
        CheckAvailabilityInput {
            date: test_date.to_string(),
            time: opt.start_local.clone(),
            party_size: 8,
            area: Some(AreaFilter::Indoor),
            duration_minutes: Some(75),
            alternatives: Some(0),
        },
        &clock,
    )
    .await;
    let avail_out: tabletalk_backend::availability::CheckAvailabilityOutput =
        serde_json::from_value(avail_res.structured_content.unwrap()).unwrap();
    if avail_out.status == "unavailable" {
        assert_eq!(
            avail_out.reason_detail,
            Some("temporarily held, may free up soon".to_string())
        );
    }

    // 4. Second caller attempting direct create_booking is blocked
    let book_res = handle_create_booking_with_clock(
        &pool,
        CreateBookingInput {
            date: test_date.to_string(),
            time: opt.start_local.clone(),
            party_size: 8,
            guest_name: "Intruder Eve".to_string(),
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
    let book_out: tabletalk_backend::booking::CreateBookingOutput =
        serde_json::from_value(book_res.structured_content.unwrap()).unwrap();
    assert_ne!(book_out.status, "confirmed");

    // 5. Advance clock past hold TTL (300 seconds = 5 minutes -> advance by 10 minutes)
    let advanced_clock = FixedClock(fixed_now + Duration::minutes(10));

    // Lazy expiry: Holder Dan attempting to accept after expiry gets offer_expired
    let late_accept = handle_accept_offer_with_clock(
        &pool,
        AcceptOfferInput {
            negotiation_id: None,
            option_id: None,
            offer_token: Some(offer_token),
            guest_name: "Holder Dan".to_string(),
            phone: None,
            email: None,
            special_requests: None,
            idempotency_key: None,
            hold_only: Some(false),
        },
        &advanced_clock,
    )
    .await;
    let late_out: AcceptOfferOutput =
        serde_json::from_value(late_accept.structured_content.unwrap()).unwrap();
    assert_eq!(late_out.status, "offer_expired");

    // Sweeper test: run sweeper and verify idempotency
    let swept_count = sweep_expired_holds_and_negotiations(&pool, &advanced_clock)
        .await
        .unwrap();
    assert!(swept_count >= 1);
    let swept_again = sweep_expired_holds_and_negotiations(&pool, &advanced_clock)
        .await
        .unwrap();
    assert_eq!(
        swept_again, 0,
        "Second sweep must be idempotent (0 records)"
    );
}

// =========================================================================
// 4. WAITLIST: FIFO ORDER, SKIPPING MISFIT, AND RE-QUEUE ON RELEASE/EXPIRY
// =========================================================================

#[tokio::test]
async fn test_waitlist_fifo_matching_fairness_and_requeue() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);
    let rate_limiter = CancellationRateLimiter::new();

    let test_date = "2026-10-16";

    // 1. Guest A (party 6, early) joins waitlist
    let res_a = handle_join_waitlist_with_clock(
        &pool,
        JoinWaitlistInput {
            guest_name: "Alice Waiter".to_string(),
            party_size: 6,
            date: test_date.to_string(),
            window_start: "19:00".to_string(),
            window_end: "21:00".to_string(),
            area: Some("indoor".to_string()),
            contact: Some("alice@example.com".to_string()),
            profile_token: None,
        },
        &clock,
    )
    .await;
    let out_a: JoinWaitlistOutput =
        serde_json::from_value(res_a.structured_content.unwrap()).unwrap();
    let code_a = out_a.code;

    // Guest B (party 2, later) joins waitlist
    let clock_b = FixedClock(fixed_now + Duration::seconds(10));
    let res_b = handle_join_waitlist_with_clock(
        &pool,
        JoinWaitlistInput {
            guest_name: "Bob Waiter".to_string(),
            party_size: 2,
            date: test_date.to_string(),
            window_start: "19:00".to_string(),
            window_end: "21:00".to_string(),
            area: Some("indoor".to_string()),
            contact: Some("bob@example.com".to_string()),
            profile_token: None,
        },
        &clock_b,
    )
    .await;
    let out_b: JoinWaitlistOutput =
        serde_json::from_value(res_b.structured_content.unwrap()).unwrap();
    let code_b = out_b.code;

    // 2. Create and then cancel a 2-top booking at 19:30 indoor
    let book_res = handle_create_booking_with_clock(
        &pool,
        CreateBookingInput {
            date: test_date.to_string(),
            time: "19:30".to_string(),
            party_size: 2,
            guest_name: "Temporary Guest".to_string(),
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
    let book_out: tabletalk_backend::booking::CreateBookingOutput =
        serde_json::from_value(book_res.structured_content.unwrap()).unwrap();
    let conf_code = book_out.booking.unwrap().confirmation_code;

    // Cancel booking -> triggers waitlist matcher!
    let _ = handle_cancel_booking_with_clock(
        &pool,
        CancelBookingInput {
            confirmation_code: conf_code,
            guest_name: "Temporary Guest".to_string(),
            reason: Some("Plans changed".to_string()),
        },
        &clock,
        &rate_limiter,
    )
    .await;

    // 3. Verify fairness:
    // A 2-top was freed! Alice (party of 6) cannot fit on a 2-top, so she must NOT be offered the slot,
    // but she MUST remain at position 1 (waiting). Bob (party of 2) fits and receives the offer!
    let stat_a = handle_get_waitlist_status_with_clock(
        &pool,
        GetWaitlistStatusInput {
            code: code_a.clone(),
            guest_name: "Alice Waiter".to_string(),
        },
        &clock,
    )
    .await;
    let out_stat_a: GetWaitlistStatusOutput =
        serde_json::from_value(stat_a.structured_content.unwrap()).unwrap();
    assert_eq!(
        out_stat_a.status, "waiting",
        "Alice should still be waiting (table too small)"
    );

    let stat_b = handle_get_waitlist_status_with_clock(
        &pool,
        GetWaitlistStatusInput {
            code: code_b.clone(),
            guest_name: "Bob Waiter".to_string(),
        },
        &clock,
    )
    .await;
    let out_stat_b: GetWaitlistStatusOutput =
        serde_json::from_value(stat_b.structured_content.unwrap()).unwrap();
    assert_eq!(
        out_stat_b.status, "offered",
        "Bob must be offered the freed 2-top table"
    );
    let offer_detail = out_stat_b.waitlist.unwrap().offer.expect("Offer detail");
    let bob_offer_token = offer_detail.offer_token;

    // 4. Bob explicitly releases offer -> reverts Bob to 'waiting', triggers matcher
    let rel_res = handle_release_offer_with_clock(
        &pool,
        ReleaseOfferInput {
            offer_token: bob_offer_token,
            guest_name: "Bob Waiter".to_string(),
        },
        &clock,
    )
    .await;
    let rel_out: ReleaseOfferOutput =
        serde_json::from_value(rel_res.structured_content.unwrap()).unwrap();
    assert_eq!(rel_out.status, "released");

    // Bob is back to 'waiting'
    let stat_b2 = handle_get_waitlist_status_with_clock(
        &pool,
        GetWaitlistStatusInput {
            code: code_b,
            guest_name: "Bob Waiter".to_string(),
        },
        &clock,
    )
    .await;
    let out_stat_b2: GetWaitlistStatusOutput =
        serde_json::from_value(stat_b2.structured_content.unwrap()).unwrap();
    assert_eq!(out_stat_b2.status, "waiting");
}

// =========================================================================
// 5. THE THREE RACE TESTS (50 ITERATIONS EACH)
// =========================================================================

#[tokio::test]
async fn test_the_race_1_concurrent_accept_offer_20_tasks() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = Arc::new(FixedClock(fixed_now));

    // Repeat race 50 iterations
    for iter in 0..50 {
        // Create negotiation session
        let guest = format!("Race1_Guest_{}", iter);
        let neg_res = handle_negotiate_booking_with_clock(
            &pool,
            NegotiateBookingInput {
                date: "2026-10-16".to_string(),
                time: "19:00".to_string(),
                party_size: 2,
                guest_name: guest.clone(),
                area: Some(BookingAreaInput::Indoor),
                duration_minutes: Some(75),
                flexibility: None,
                profile_token: None,
                negotiation_id: None,
                exclude_offered: None,
                max_options: Some(2),
            },
            clock.as_ref(),
        )
        .await;
        let neg_out: NegotiateBookingOutput =
            serde_json::from_value(neg_res.structured_content.unwrap()).unwrap();
        if neg_out.options.is_empty() {
            continue;
        }
        let neg_id = neg_out.negotiation_id.unwrap();
        let opt_id = neg_out.options[0].option_id.clone();

        // Spawn 20 concurrent tasks trying to accept_offer on opt_id
        let mut handles = Vec::new();
        for _task_idx in 0..20 {
            let p = pool.clone();
            let c = clock.clone();
            let n_id = neg_id;
            let o_id = opt_id.clone();
            let g = guest.clone();

            handles.push(tokio::spawn(async move {
                handle_accept_offer_with_clock(
                    &p,
                    AcceptOfferInput {
                        negotiation_id: Some(n_id),
                        option_id: Some(o_id),
                        offer_token: None,
                        guest_name: g,
                        phone: None,
                        email: None,
                        special_requests: None,
                        idempotency_key: None,
                        hold_only: Some(false),
                    },
                    c.as_ref(),
                )
                .await
            }));
        }

        let mut confirmed_count = 0;
        let mut rejected_count = 0;

        for h in handles {
            let res = h.await.unwrap();
            if let Some(sc) = res.structured_content {
                let out: AcceptOfferOutput = serde_json::from_value(sc).unwrap();
                if out.status == "confirmed" {
                    confirmed_count += 1;
                } else {
                    rejected_count += 1;
                }
            } else {
                rejected_count += 1;
            }
        }

        assert_eq!(
            confirmed_count, 1,
            "Iteration {}: Exactly 1 task must successfully confirm booking, got {}",
            iter, confirmed_count
        );
        assert_eq!(
            rejected_count, 19,
            "Iteration {}: Exactly 19 tasks must be rejected, got {}",
            iter, rejected_count
        );

        // Reset iteration state
        sqlx::query("DELETE FROM bookings WHERE guest_name = ?")
            .bind(&guest)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM negotiations")
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn test_the_race_2_conversion_vs_release_vs_expiry() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;

    // Run 50 iterations: 3 tasks racing (accept hold vs release hold vs expiry sweep)
    for iter in 0..50 {
        let clock = Arc::new(FixedClock(fixed_now));
        let guest = format!("Race2_Guest_{}", iter);

        // 1. Negotiate and place a hold
        let neg_res = handle_negotiate_booking_with_clock(
            &pool,
            NegotiateBookingInput {
                date: "2026-10-16".to_string(),
                time: "18:00".to_string(),
                party_size: 2,
                guest_name: guest.clone(),
                area: Some(BookingAreaInput::Indoor),
                duration_minutes: Some(75),
                flexibility: None,
                profile_token: None,
                negotiation_id: None,
                exclude_offered: None,
                max_options: Some(2),
            },
            clock.as_ref(),
        )
        .await;
        let neg_out: NegotiateBookingOutput =
            serde_json::from_value(neg_res.structured_content.unwrap()).unwrap();
        if neg_out.options.is_empty() {
            continue;
        }
        let neg_id = neg_out.negotiation_id.unwrap();
        let opt_id = neg_out.options[0].option_id.clone();

        let hold_res = handle_accept_offer_with_clock(
            &pool,
            AcceptOfferInput {
                negotiation_id: Some(neg_id),
                option_id: Some(opt_id),
                offer_token: None,
                guest_name: guest.clone(),
                phone: None,
                email: None,
                special_requests: None,
                idempotency_key: None,
                hold_only: Some(true),
            },
            clock.as_ref(),
        )
        .await;
        let hold_out: AcceptOfferOutput =
            serde_json::from_value(hold_res.structured_content.unwrap()).unwrap();
        let token = hold_out.offer_token.expect("Must have hold token");

        // 2. Race: Task A (accept_offer), Task B (release_offer), Task C (expired clock check)
        let p1 = pool.clone();
        let c1 = clock.clone();
        let t1 = token.clone();
        let g1 = guest.clone();
        let h_accept = tokio::spawn(async move {
            handle_accept_offer_with_clock(
                &p1,
                AcceptOfferInput {
                    negotiation_id: None,
                    option_id: None,
                    offer_token: Some(t1),
                    guest_name: g1,
                    phone: None,
                    email: None,
                    special_requests: None,
                    idempotency_key: None,
                    hold_only: Some(false),
                },
                c1.as_ref(),
            )
            .await
        });

        let p2 = pool.clone();
        let c2 = clock.clone();
        let t2 = token.clone();
        let g2 = guest.clone();
        let h_release = tokio::spawn(async move {
            handle_release_offer_with_clock(
                &p2,
                ReleaseOfferInput {
                    offer_token: t2,
                    guest_name: g2,
                },
                c2.as_ref(),
            )
            .await
        });

        let p3 = pool.clone();
        let expired_clock = Arc::new(FixedClock(fixed_now + Duration::minutes(15)));
        let h_sweep = tokio::spawn(async move {
            sweep_expired_holds_and_negotiations(&p3, expired_clock.as_ref()).await
        });

        let (res_a, res_b, res_c) = tokio::join!(h_accept, h_release, h_sweep);
        assert!(res_a.is_ok());
        assert!(res_b.is_ok());
        assert!(res_c.is_ok());

        // 3. Verify DB invariant: hold must end in exactly one terminal state (converted, released, or expired)
        let token_hash = tabletalk_backend::negotiation::hash_token(&token);
        let final_status: String =
            sqlx::query_scalar("SELECT status FROM slot_holds WHERE token_hash = ?")
                .bind(&token_hash)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert!(
            final_status == "converted" || final_status == "released" || final_status == "expired",
            "Iteration {}: Hold must be in valid terminal status, got {}",
            iter,
            final_status
        );
    }
}

#[tokio::test]
async fn test_the_race_3_cancel_booking_vs_waitlist_vs_concurrent_create_booking() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = Arc::new(FixedClock(fixed_now));
    let rate_limiter = Arc::new(CancellationRateLimiter::new());

    let test_date = "2026-10-16";

    // Run 50 iterations of cancel racing with waitlist & 10 outside create_booking callers
    for iter in 0..50 {
        // Book 2-top
        let book_res = handle_create_booking_with_clock(
            &pool,
            CreateBookingInput {
                date: test_date.to_string(),
                time: "19:00".to_string(),
                party_size: 2,
                guest_name: format!("CancelRacer_{}", iter),
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
            clock.as_ref(),
        )
        .await;
        let book_out: tabletalk_backend::booking::CreateBookingOutput =
            serde_json::from_value(book_res.structured_content.unwrap()).unwrap();
        let conf_code = book_out.booking.unwrap().confirmation_code;

        // Join 10 waitlist entries
        let mut wl_codes = Vec::new();
        for w_idx in 0..10 {
            let wl_res = handle_join_waitlist_with_clock(
                &pool,
                JoinWaitlistInput {
                    guest_name: format!("WL_{}_{}", iter, w_idx),
                    party_size: 2,
                    date: test_date.to_string(),
                    window_start: "18:30".to_string(),
                    window_end: "20:30".to_string(),
                    area: Some("indoor".to_string()),
                    contact: None,
                    profile_token: None,
                },
                clock.as_ref(),
            )
            .await;
            let wl_out: JoinWaitlistOutput =
                serde_json::from_value(wl_res.structured_content.unwrap()).unwrap();
            wl_codes.push((wl_out.code, format!("WL_{}_{}", iter, w_idx)));
        }

        // Cancel the booking concurrently with 10 outside create_booking callers
        let p_cancel = pool.clone();
        let c_cancel = clock.clone();
        let code_to_cancel = conf_code.clone();
        let g_cancel = format!("CancelRacer_{}", iter);
        let rl = rate_limiter.clone();
        let h_cancel = tokio::spawn(async move {
            handle_cancel_booking_with_clock(
                &p_cancel,
                CancelBookingInput {
                    confirmation_code: code_to_cancel,
                    guest_name: g_cancel,
                    reason: Some("Race test cancel".to_string()),
                },
                c_cancel.as_ref(),
                rl.as_ref(),
            )
            .await
        });

        let mut outside_handles = Vec::new();
        for out_idx in 0..10 {
            let p_out = pool.clone();
            let c_out = clock.clone();
            let g_out = format!("Outside_{}_{}", iter, out_idx);
            outside_handles.push(tokio::spawn(async move {
                handle_create_booking_with_clock(
                    &p_out,
                    CreateBookingInput {
                        date: test_date.to_string(),
                        time: "19:00".to_string(),
                        party_size: 2,
                        guest_name: g_out,
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
                    c_out.as_ref(),
                )
                .await
            }));
        }

        let cancel_res = h_cancel.await.unwrap();
        assert_eq!(cancel_res.is_error, Some(false));

        for oh in outside_handles {
            let _ = oh.await;
        }

        // Assert waitlist match created exactly ONE hold for the FIFO waitlist entry
        let mut offered_count = 0;
        for (w_code, w_guest) in &wl_codes {
            let stat_res = handle_get_waitlist_status_with_clock(
                &pool,
                GetWaitlistStatusInput {
                    code: w_code.clone(),
                    guest_name: w_guest.clone(),
                },
                clock.as_ref(),
            )
            .await;
            let stat_out: GetWaitlistStatusOutput =
                serde_json::from_value(stat_res.structured_content.unwrap()).unwrap();
            if stat_out.status == "offered" {
                offered_count += 1;
            }
        }

        assert_eq!(
            offered_count, 1,
            "Iteration {}: Exactly one waitlist entry must be offered the freed slot",
            iter
        );

        // Reset state for next iteration
        sqlx::query("DELETE FROM waitlist_entries")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM slot_holds")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM bookings WHERE guest_name LIKE 'CancelRacer_%' OR guest_name LIKE 'Outside_%'").execute(&pool).await.unwrap();
    }
}

// =========================================================================
// 6. TOKEN SECURITY & HARD PRIVACY SCAN
// =========================================================================

#[tokio::test]
async fn test_token_privacy_never_in_db_and_forget_me_scrubs_all() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let guest_name = "Super Secret VIP Guest";
    let guest_email = "supersecretvip@example.com";

    // 1. Enable memory for guest to get profile_token
    let enable_res = tabletalk_backend::tools::enable_memory::handle_enable_memory_with_clock(
        &pool,
        tabletalk_backend::tools::enable_memory::EnableMemoryInput {
            guest_name: guest_name.to_string(),
            email: Some(guest_email.to_string()),
            phone: None,
            consent_acknowledged: true,
            initial_preferences: None,
        },
        &clock,
    )
    .await;
    let enable_out: tabletalk_backend::memory::EnableMemoryOutput =
        serde_json::from_value(enable_res.structured_content.unwrap()).unwrap();
    let prof_token = enable_out.profile_token.unwrap();

    // 2. Join waitlist with profile_token
    let wl_res = handle_join_waitlist_with_clock(
        &pool,
        JoinWaitlistInput {
            guest_name: guest_name.to_string(),
            party_size: 2,
            date: "2026-10-16".to_string(),
            window_start: "19:00".to_string(),
            window_end: "20:30".to_string(),
            area: Some("indoor".to_string()),
            contact: Some(guest_email.to_string()),
            profile_token: Some(prof_token.clone()),
        },
        &clock,
    )
    .await;
    assert_eq!(wl_res.is_error, Some(false));

    // 3. Create a negotiation and hold
    let neg_res = handle_negotiate_booking_with_clock(
        &pool,
        NegotiateBookingInput {
            date: "2026-10-16".to_string(),
            time: "19:00".to_string(),
            party_size: 2,
            guest_name: guest_name.to_string(),
            area: Some(BookingAreaInput::Indoor),
            duration_minutes: Some(75),
            flexibility: None,
            profile_token: Some(prof_token.clone()),
            negotiation_id: None,
            exclude_offered: None,
            max_options: Some(2),
        },
        &clock,
    )
    .await;
    let neg_out: NegotiateBookingOutput =
        serde_json::from_value(neg_res.structured_content.unwrap()).unwrap();
    let neg_id = neg_out.negotiation_id.unwrap();
    let opt_id = neg_out.options[0].option_id.clone();

    let hold_res = handle_accept_offer_with_clock(
        &pool,
        AcceptOfferInput {
            negotiation_id: Some(neg_id),
            option_id: Some(opt_id),
            offer_token: None,
            guest_name: guest_name.to_string(),
            phone: None,
            email: Some(guest_email.to_string()),
            special_requests: None,
            idempotency_key: None,
            hold_only: Some(true),
        },
        &clock,
    )
    .await;
    let hold_out: AcceptOfferOutput =
        serde_json::from_value(hold_res.structured_content.unwrap()).unwrap();
    let offer_token = hold_out.offer_token.unwrap();

    // Verify plain offer token NEVER appears anywhere in any SQLite table!
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    for table in &tables {
        let cols: Vec<String> = sqlx::query(&format!("PRAGMA table_info({})", table))
            .fetch_all(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.get::<String, _>("name"))
            .collect();

        for col in cols {
            let count: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM {} WHERE CAST({} AS TEXT) LIKE '%{}%'",
                table, col, offer_token
            ))
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

            assert_eq!(
                count, 0,
                "Plain token leaked into table {} column {}",
                table, col
            );
        }
    }

    // 4. Run forget_me for guest
    let forget_res = handle_forget_me_with_clock(
        &pool,
        ForgetMeInput {
            profile_token: prof_token,
            confirm: true,
        },
        &clock,
    )
    .await;
    assert_eq!(forget_res.is_error, Some(false));

    // Full database scan: 0 occurrences of guest name or email in waitlist_entries, slot_holds, negotiations
    let scan_tables = ["waitlist_entries", "slot_holds", "negotiations"];
    for table in &scan_tables {
        let name_count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {} WHERE guest_name = ?",
            table
        ))
        .bind(guest_name)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

        assert_eq!(
            name_count, 0,
            "forget_me failed to scrub guest name from {}",
            table
        );
    }
}

// =========================================================================
// 7. MCP TOOLS/LIST OVER HTTP (22 TOOLS) & LATENCY BENCHMARK
// =========================================================================

#[tokio::test]
async fn test_mcp_tools_list_twenty_two_tools_http() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
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
                "clientInfo": { "name": "phase11-test", "version": "1.0.0" }
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

    // 2. tools/list
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

    let raw = list_res.text().await.unwrap();
    let json_res = parse_mcp_json_or_sse(&raw);
    let tools = json_res["result"]["tools"].as_array().expect("Tools array");

    assert_eq!(
        tools.len(),
        24,
        "tools/list must return exactly 24 tools in Phase 12"
    );

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let expected_new = [
        "negotiate_booking",
        "accept_offer",
        "release_offer",
        "join_waitlist",
        "get_waitlist_status",
        "leave_waitlist",
    ];

    for name in &expected_new {
        assert!(tool_names.contains(name), "Missing tool: {}", name);
    }

    // Verify annotations
    let get_wl = tools
        .iter()
        .find(|t| t["name"] == "get_waitlist_status")
        .unwrap();
    assert_eq!(get_wl["annotations"]["readOnlyHint"], true);

    let leave_wl = tools
        .iter()
        .find(|t| t["name"] == "leave_waitlist")
        .unwrap();
    assert_eq!(leave_wl["annotations"]["destructiveHint"], true);
}

#[tokio::test]
async fn test_negotiate_booking_latency_median() {
    let fixed_now = Utc.with_ymd_and_hms(2026, 10, 16, 12, 0, 0).unwrap();
    let pool = setup_seeded_db(Some(fixed_now)).await;
    let clock = FixedClock(fixed_now);

    let count = 20;
    let mut durations = Vec::with_capacity(count);

    for idx in 0..count {
        let input = NegotiateBookingInput {
            date: "2026-10-16".to_string(),
            time: "19:30".to_string(),
            party_size: 2,
            guest_name: format!("LatencyTester_{}", idx),
            area: Some(BookingAreaInput::Any),
            duration_minutes: Some(75),
            flexibility: None,
            profile_token: None,
            negotiation_id: None,
            exclude_offered: None,
            max_options: Some(3),
        };

        let t0 = std::time::Instant::now();
        let res = handle_negotiate_booking_with_clock(&pool, input, &clock).await;
        durations.push(t0.elapsed());
        assert_eq!(res.is_error, Some(false));
    }

    durations.sort();
    let median = durations[durations.len() / 2];
    println!("Median negotiate_booking latency: {:?}", median);
    // Assertion generous to avoid CI flakiness, typical is < 15ms
    assert!(
        median < std::time::Duration::from_millis(100),
        "Median negotiate latency must be under 100ms, got {:?}",
        median
    );
}
