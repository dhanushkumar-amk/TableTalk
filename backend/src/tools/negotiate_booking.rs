use crate::{
    availability::{
        check_availability_engine, AreaFilter, AvailabilityConfig, AvailabilityQuery, Clock,
        SystemClock,
    },
    db::repo::{restaurant, tables},
    negotiation::{
        format_spoken_options, score_candidate, sort_scored_candidates, CandidateSlot,
        NegotiationOption, StoredOfferSlot, DEFAULT_NEGOTIATION_TTL_MINUTES,
        MAX_ACTIVE_NEGOTIATIONS_PER_GUEST, MAX_NEGOTIATION_ROUNDS,
    },
    tools::create_booking::BookingAreaInput,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rmcp::model::{CallToolResult, ContentBlock};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct NegotiationFlexibility {
    /// Search window around requested time in minutes (default 90, max 240).
    pub time_window_minutes: Option<i64>,
    /// Whether to allow tables in other seating areas if requested area is unavailable (default true).
    pub allow_other_areas: Option<bool>,
    /// Whether to search on subsequent days within the booking horizon (default false).
    pub allow_other_days: Option<bool>,
    /// Earliest acceptable local time in HH:MM format.
    pub earliest_time: Option<String>,
    /// Latest acceptable local time in HH:MM format.
    pub latest_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NegotiateBookingInput {
    /// Local restaurant date in YYYY-MM-DD format (e.g. '2026-10-16').
    pub date: String,
    /// Local 24-hour time in HH:MM format (e.g. '19:30').
    pub time: String,
    /// Party size (number of guests).
    pub party_size: i64,
    /// Name of the dining guest.
    pub guest_name: String,
    /// Optional seating area preference: 'indoor', 'patio', 'bar', or 'any'.
    pub area: Option<BookingAreaInput>,
    /// Optional dining duration in minutes.
    pub duration_minutes: Option<i64>,
    /// Optional search flexibility settings.
    pub flexibility: Option<NegotiationFlexibility>,
    /// Optional profile bearer token to apply soft seating preference.
    pub profile_token: Option<String>,
    /// Optional existing negotiation ID to request additional alternative options.
    pub negotiation_id: Option<i64>,
    /// Whether to exclude slots already offered in prior rounds of this negotiation (default true).
    pub exclude_offered: Option<bool>,
    /// Maximum number of ranked alternative options to return (1 to 4, default 3).
    pub max_options: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NegotiateBookingOutput {
    pub status: String,
    pub negotiation_id: Option<i64>,
    pub round: Option<i64>,
    pub slot: Option<crate::availability::AvailableSlot>,
    pub options: Vec<NegotiationOption>,
    pub profile_applied: Option<bool>,
    pub message: String,
}

fn tool_validation_error(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

pub async fn handle_negotiate_booking(
    pool: &SqlitePool,
    input: NegotiateBookingInput,
) -> CallToolResult {
    let clock = SystemClock;
    handle_negotiate_booking_with_clock(pool, input, &clock).await
}

pub async fn handle_negotiate_booking_with_clock(
    pool: &SqlitePool,
    input: NegotiateBookingInput,
    clock: &dyn Clock,
) -> CallToolResult {
    let guest_name = input.guest_name.trim();
    if guest_name.is_empty() {
        return tool_validation_error("Guest name is required.");
    }
    if input.party_size <= 0 {
        return tool_validation_error("Party size must be at least 1.");
    }

    let parsed_date = match NaiveDate::parse_from_str(input.date.trim(), "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return tool_validation_error("Invalid date format. Expected YYYY-MM-DD."),
    };

    let parsed_time = match NaiveTime::parse_from_str(input.time.trim(), "%H:%M") {
        Ok(t) => t,
        Err(_) => return tool_validation_error("Invalid time format. Expected HH:MM."),
    };

    let max_opts = input.max_options.unwrap_or(3).clamp(1, 4);
    let flex = input.flexibility.unwrap_or_default();
    let time_window = flex.time_window_minutes.unwrap_or(90).clamp(15, 240);
    let allow_other_areas = flex.allow_other_areas.unwrap_or(true);
    let allow_other_days = flex.allow_other_days.unwrap_or(false);
    let exclude_offered = input.exclude_offered.unwrap_or(true);

    let area_filter = match input.area.unwrap_or(BookingAreaInput::Any) {
        BookingAreaInput::Indoor => AreaFilter::Indoor,
        BookingAreaInput::Patio => AreaFilter::Patio,
        BookingAreaInput::Bar => AreaFilter::Bar,
        BookingAreaInput::Any => AreaFilter::Any,
    };

    // Load restaurant config and data
    let config = match restaurant::get_config(pool).await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("Failed to load restaurant config: {}", e)),
    };
    let opening_hours = match restaurant::get_opening_hours(pool).await {
        Ok(h) => h,
        Err(e) => return tool_validation_error(format!("Failed to load opening hours: {}", e)),
    };
    let special_closures = match restaurant::get_special_closures(pool).await {
        Ok(s) => s,
        Err(e) => return tool_validation_error(format!("Failed to load special closures: {}", e)),
    };
    let active_tables = match tables::list_active_tables(pool).await {
        Ok(t) => t,
        Err(e) => return tool_validation_error(format!("Failed to load tables: {}", e)),
    };

    let tz: Tz = match config.timezone.parse() {
        Ok(t) => t,
        Err(_) => return tool_validation_error("Invalid restaurant timezone."),
    };

    // Resolve profile token if provided
    let (profile_applied, stored_seat_pref) = if let Some(ref tok) = input.profile_token {
        let rate_limiter = crate::memory::get_token_rate_limiter();
        match crate::memory::resolve_profile_token(pool, tok, rate_limiter, clock).await {
            Ok(Some(cust)) => {
                let pref: Option<String> = sqlx::query_scalar(
                    "SELECT value FROM customer_preferences WHERE customer_id = ? AND kind = 'seat_pref' LIMIT 1"
                )
                .bind(cust.id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
                (Some(true), pref)
            }
            _ => (Some(false), None),
        }
    } else {
        (None, None)
    };

    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();

    // Load active bookings and holds in range
    let search_start_utc = match tz.from_local_datetime(&parsed_date.and_hms_opt(0, 0, 0).unwrap())
    {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(e, _) => e.with_timezone(&Utc),
        chrono::LocalResult::None => now_utc - Duration::days(1),
    };
    let search_end_utc = search_start_utc + Duration::days(7);

    let active_bookings = crate::db::repo::bookings::list_active_bookings_in_range(
        pool,
        &search_start_utc.to_rfc3339(),
        &search_end_utc.to_rfc3339(),
    )
    .await
    .unwrap_or_default();

    let active_holds: Vec<(i64, i64, String, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT id, table_id, start_at, end_at, guest_name, party_size
        FROM slot_holds
        WHERE status = 'active'
          AND expires_at > ?
          AND start_at < ?
          AND end_at > ?
        "#,
    )
    .bind(&now_iso)
    .bind(search_end_utc.to_rfc3339())
    .bind(search_start_utc.to_rfc3339())
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut combined_bookings = active_bookings.clone();
    for (hold_id, table_id, start_at, end_at, h_guest, h_party) in &active_holds {
        combined_bookings.push(crate::db::models::Booking {
            id: format!("hold_{}", hold_id),
            confirmation_code: format!("HOLD-{}", hold_id),
            customer_id: None,
            guest_name: h_guest.clone(),
            party_size: *h_party,
            start_at: start_at.clone(),
            end_at: end_at.clone(),
            status: crate::db::models::BookingStatus::Pending,
            table_id: Some(*table_id),
            special_requests: None,
            deposit_status: crate::db::models::DepositStatus::None,
            deposit_amount_cents: 0,
            created_at: now_iso.clone(),
            updated_at: now_iso.clone(),
            cancelled_at: None,
            guest_phone: None,
            guest_email: None,
        });
    }

    let duration_mins = input
        .duration_minutes
        .unwrap_or(config.dining_duration_minutes);

    // 1. If negotiation_id is None, check if the EXACT requested slot is available!
    if input.negotiation_id.is_none() {
        let exact_query = AvailabilityQuery {
            date: parsed_date,
            time: parsed_time,
            party_size: input.party_size,
            area: area_filter,
            duration_minutes: Some(duration_mins),
            alternatives_limit: 0,
        };
        let exact_res = check_availability_engine(
            exact_query,
            &config,
            &opening_hours,
            &special_closures,
            &active_tables,
            &combined_bookings,
            &AvailabilityConfig::default(),
            clock,
        );

        if exact_res.status == "available" {
            if let Some(slot) = exact_res.slot {
                let voice_text = format!(
                    "I have a table for {} on {} at {} {}. Would you like me to book it?",
                    input.party_size,
                    parsed_date.format("%A, %B %e"),
                    parsed_time.format("%l:%M %P").to_string().trim(),
                    slot.area
                );
                let output = NegotiateBookingOutput {
                status: "available_as_requested".to_string(),
                negotiation_id: None,
                round: None,
                slot: Some(slot),
                options: vec![],
                profile_applied,
                message: "The requested dining slot is currently available. You may proceed to create a booking or place a hold.".to_string(),
            };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text(voice_text)];
                return res;
            }
        }
    }

    // 2. Resolve or create negotiation record
    let mut conn = match pool.acquire().await {
        Ok(c) => c,
        Err(e) => return tool_validation_error(format!("DB error: {}", e)),
    };

    let (neg_id, round, already_offered): (i64, i64, Vec<StoredOfferSlot>) = if let Some(n_id) =
        input.negotiation_id
    {
        // Look up existing negotiation
        let neg: Option<crate::db::models::Negotiation> = match sqlx::query_as(
            "SELECT id, guest_name, date, time, party_size, area, duration_minutes, round, offered_slots, status, created_at, expires_at FROM negotiations WHERE id = ?"
        )
        .bind(n_id)
        .fetch_optional(&mut *conn)
        .await
        {
            Ok(Some(n)) => Some(n),
            _ => None,
        };

        let neg = match neg {
            Some(n) => n,
            None => {
                let output = NegotiateBookingOutput {
                    status: "negotiation_not_found".to_string(),
                    negotiation_id: Some(n_id),
                    round: None,
                    slot: None,
                    options: vec![],
                    profile_applied,
                    message: "Negotiation session not found or expired.".to_string(),
                };
                let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
                res.content = vec![ContentBlock::text("I couldn't find that active negotiation session. Would you like to check fresh availability?")];
                return res;
            }
        };

        if neg.guest_name.trim().to_lowercase() != guest_name.to_lowercase() {
            let output = NegotiateBookingOutput {
                status: "negotiation_not_found".to_string(),
                negotiation_id: Some(n_id),
                round: None,
                slot: None,
                options: vec![],
                profile_applied,
                message: "Negotiation session not found or expired.".to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text("I couldn't find that active negotiation session. Would you like to check fresh availability?")];
            return res;
        }

        if neg.expires_at <= now_iso || neg.status != crate::db::models::NegotiationStatus::Open {
            let output = NegotiateBookingOutput {
                status: "negotiation_expired".to_string(),
                negotiation_id: Some(n_id),
                round: Some(neg.round),
                slot: None,
                options: vec![],
                profile_applied,
                message:
                    "This negotiation session has expired. Please start a new reservation search."
                        .to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text(
                "That negotiation session has expired. Would you like to check fresh options?",
            )];
            return res;
        }

        if neg.round >= MAX_NEGOTIATION_ROUNDS {
            let output = NegotiateBookingOutput {
                status: "no_more_options".to_string(),
                negotiation_id: Some(n_id),
                round: Some(neg.round),
                slot: None,
                options: vec![],
                profile_applied,
                message:
                    "Maximum negotiation rounds reached for this search. You may join the waitlist."
                        .to_string(),
            };
            let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
            res.content = vec![ContentBlock::text("I don't have any further alternatives for this time. Would you like to join the waitlist?")];
            return res;
        }

        let offered: Vec<StoredOfferSlot> =
            serde_json::from_str(&neg.offered_slots).unwrap_or_default();
        (neg.id, neg.round + 1, offered)
    } else {
        // Check active negotiations count for guest
        let active_negs: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM negotiations WHERE guest_name = ? AND status = 'open' AND expires_at > ?"
        )
        .bind(guest_name)
        .bind(&now_iso)
        .fetch_one(&mut *conn)
        .await
        .unwrap_or(0);

        if active_negs >= MAX_ACTIVE_NEGOTIATIONS_PER_GUEST {
            return tool_validation_error("Maximum active negotiation sessions reached. Please complete or wait for existing sessions to conclude.");
        }

        let neg_ttl = now_utc + Duration::minutes(DEFAULT_NEGOTIATION_TTL_MINUTES);
        let n_id = match sqlx::query(
            r#"
            INSERT INTO negotiations (
                guest_name, date, time, party_size, area,
                duration_minutes, round, offered_slots, status,
                created_at, expires_at
            ) VALUES (?, ?, ?, ?, ?, ?, 1, '[]', 'open', ?, ?)
            "#,
        )
        .bind(guest_name)
        .bind(input.date.trim())
        .bind(input.time.trim())
        .bind(input.party_size)
        .bind(input.area.map(|a| a.as_str()))
        .bind(duration_mins)
        .bind(&now_iso)
        .bind(neg_ttl.to_rfc3339())
        .execute(&mut *conn)
        .await
        {
            Ok(r) => r.last_insert_rowid(),
            Err(e) => {
                return tool_validation_error(format!(
                    "Failed to create negotiation session: {}",
                    e
                ))
            }
        };

        (n_id, 1, Vec::new())
    };

    // 3. Search Candidate Slots
    let earliest_limit = flex
        .earliest_time
        .as_deref()
        .and_then(|t| NaiveTime::parse_from_str(t.trim(), "%H:%M").ok());
    let latest_limit = flex
        .latest_time
        .as_deref()
        .and_then(|t| NaiveTime::parse_from_str(t.trim(), "%H:%M").ok());

    let mut candidate_slots: Vec<CandidateSlot> = Vec::new();
    let num_days = if allow_other_days { 3 } else { 1 };

    for d_offset in 0..num_days {
        let curr_date = parsed_date + Duration::days(d_offset);
        let day_of_week = curr_date.weekday().num_days_from_sunday() as i64;

        // Check opening hours for day
        let hours_for_day: Vec<_> = opening_hours
            .iter()
            .filter(|h| h.day_of_week == day_of_week && !h.is_closed)
            .collect();
        if hours_for_day.is_empty() {
            continue;
        }

        // Check closures
        let is_closed = special_closures
            .iter()
            .any(|c| c.closure_date == curr_date.to_string() && c.is_all_day);
        if is_closed {
            continue;
        }

        for period in hours_for_day {
            let p_open = match NaiveTime::parse_from_str(&period.open_time, "%H:%M") {
                Ok(t) => t,
                Err(_) => continue,
            };
            let p_close = match NaiveTime::parse_from_str(&period.close_time, "%H:%M") {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Window around requested time
            let mut slot_time = p_open;
            while slot_time + Duration::minutes(duration_mins) <= p_close {
                // Check flexibility limits
                if let Some(e) = earliest_limit {
                    if slot_time < e {
                        slot_time += Duration::minutes(config.slot_granularity_minutes);
                        continue;
                    }
                }
                if let Some(l) = latest_limit {
                    if slot_time > l {
                        slot_time += Duration::minutes(config.slot_granularity_minutes);
                        continue;
                    }
                }

                // Check distance from requested time if same day
                if d_offset == 0 {
                    let req_mins = parsed_time.hour() as i64 * 60 + parsed_time.minute() as i64;
                    let curr_mins = slot_time.hour() as i64 * 60 + slot_time.minute() as i64;
                    if (curr_mins - req_mins).abs() > time_window {
                        slot_time += Duration::minutes(config.slot_granularity_minutes);
                        continue;
                    }
                }

                // Convert slot to UTC
                let slot_start_local = curr_date.and_time(slot_time);
                let slot_start_utc = match tz.from_local_datetime(&slot_start_local) {
                    chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
                    chrono::LocalResult::Ambiguous(e, _) => e.with_timezone(&Utc),
                    chrono::LocalResult::None => {
                        slot_time += Duration::minutes(config.slot_granularity_minutes);
                        continue;
                    }
                };
                let slot_end_utc = slot_start_utc + Duration::minutes(duration_mins);

                // Check if in past
                if slot_start_utc <= now_utc {
                    slot_time += Duration::minutes(config.slot_granularity_minutes);
                    continue;
                }

                let minutes_from_closing = {
                    let close_mins = p_close.hour() as i64 * 60 + p_close.minute() as i64;
                    let start_mins = slot_time.hour() as i64 * 60 + slot_time.minute() as i64;
                    close_mins - start_mins
                };

                // Check tables
                for tbl in &active_tables {
                    if tbl.capacity_min > input.party_size || tbl.capacity_max < input.party_size {
                        continue;
                    }
                    if !allow_other_areas && !area_filter.matches(tbl.area) {
                        continue;
                    }

                    // Check overlap
                    let overlaps = combined_bookings.iter().any(|b| {
                        if b.table_id != Some(tbl.id) {
                            return false;
                        }
                        if let (Ok(b_start), Ok(b_end)) = (
                            DateTime::parse_from_rfc3339(&b.start_at),
                            DateTime::parse_from_rfc3339(&b.end_at),
                        ) {
                            let b_start_utc = b_start.with_timezone(&Utc);
                            let b_end_utc = b_end.with_timezone(&Utc);
                            b_start_utc < slot_end_utc && b_end_utc > slot_start_utc
                        } else {
                            false
                        }
                    });

                    if !overlaps {
                        candidate_slots.push(CandidateSlot {
                            table_id: tbl.id,
                            table_label: tbl.label.clone(),
                            capacity_min: tbl.capacity_min,
                            capacity_max: tbl.capacity_max,
                            area: tbl.area,
                            start_at: slot_start_utc,
                            end_at: slot_end_utc,
                            start_local_time: slot_time,
                            end_local_time: slot_time + Duration::minutes(duration_mins),
                            local_date: curr_date,
                            minutes_from_closing,
                        });
                    }
                }

                slot_time += Duration::minutes(config.slot_granularity_minutes);
            }
        }
    }

    // 4. Score and filter candidates
    let mut scored: Vec<_> = candidate_slots
        .into_iter()
        .map(|c| {
            score_candidate(
                &c,
                parsed_date,
                parsed_time,
                input.party_size,
                area_filter,
                stored_seat_pref.as_deref(),
                45,
            )
        })
        .filter(|sc| {
            if !exclude_offered {
                return true;
            }
            !already_offered.iter().any(|o| {
                (o.table_id == sc.candidate.table_id
                    && o.start_at == sc.candidate.start_at.to_rfc3339())
                    || (o.date == sc.candidate.local_date.to_string()
                        && o.time == sc.candidate.start_local_time.format("%H:%M").to_string()
                        && o.area == sc.candidate.area.as_str())
            })
        })
        .collect();

    sort_scored_candidates(&mut scored);

    // Pick top unique time/area options
    let mut options = Vec::new();
    let mut new_stored_slots = already_offered;
    let mut seen_slots = std::collections::HashSet::new();

    for sc in scored {
        let key = (
            sc.candidate.local_date,
            sc.candidate.start_local_time,
            sc.candidate.area,
        );
        if seen_slots.contains(&key) {
            continue;
        }
        seen_slots.insert(key);

        let opt_num = options.len() + 1;
        let opt_id = format!("opt_{}", opt_num);
        let start_local_str = sc.candidate.start_local_time.format("%H:%M").to_string();
        let end_local_str = sc.candidate.end_local_time.format("%H:%M").to_string();

        options.push(NegotiationOption {
            option_id: opt_id.clone(),
            start_local: start_local_str.clone(),
            end_local: end_local_str.clone(),
            area: sc.candidate.area.as_str().to_string(),
            table_label: Some(sc.candidate.table_label.clone()),
            minutes_from_requested: sc.minutes_from_requested,
            score_reasons: sc.score_reasons,
            rank: opt_num,
        });

        new_stored_slots.push(StoredOfferSlot {
            option_id: opt_id,
            round,
            table_id: sc.candidate.table_id,
            table_label: sc.candidate.table_label,
            start_at: sc.candidate.start_at.to_rfc3339(),
            end_at: sc.candidate.end_at.to_rfc3339(),
            start_local: start_local_str,
            end_local: end_local_str,
            area: sc.candidate.area.as_str().to_string(),
            party_size: input.party_size,
            date: sc.candidate.local_date.to_string(),
            time: sc.candidate.start_local_time.format("%H:%M").to_string(),
        });

        if options.len() >= max_opts {
            break;
        }
    }

    if options.is_empty() {
        let output = NegotiateBookingOutput {
            status: "no_more_options".to_string(),
            negotiation_id: Some(neg_id),
            round: Some(round),
            slot: None,
            options: vec![],
            profile_applied,
            message: "No alternative dining options found matching your flexibility criteria. You may join the waitlist.".to_string(),
        };
        let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
        res.content = vec![ContentBlock::text("I don't have any tables available around that time. Would you like to join the waitlist?")];
        return res;
    }

    // Save updated offered slots and round in DB
    let offered_json = serde_json::to_string(&new_stored_slots).unwrap_or_default();
    let _ = sqlx::query("UPDATE negotiations SET round = ?, offered_slots = ? WHERE id = ?")
        .bind(round)
        .bind(&offered_json)
        .bind(neg_id)
        .execute(&mut *conn)
        .await;

    let voice_text = format_spoken_options(&options);
    let output = NegotiateBookingOutput {
        status: "alternatives_offered".to_string(),
        negotiation_id: Some(neg_id),
        round: Some(round),
        slot: None,
        options,
        profile_applied,
        message: format!(
            "Ranked alternative dining options generated (round {} of {}).",
            round, MAX_NEGOTIATION_ROUNDS
        ),
    };

    let mut res = CallToolResult::structured(serde_json::to_value(&output).unwrap());
    res.content = vec![ContentBlock::text(voice_text)];
    res
}
