use crate::availability::{AreaFilter, Clock};
use crate::db::models::{Area, WaitlistEntry, WaitlistStatus};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Timelike, Utc};
use rand::RngCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};

pub const DEFAULT_HOLD_MINUTES: i64 = 5;
pub const MAX_HOLD_MINUTES: i64 = 10;
pub const DEFAULT_NEGOTIATION_TTL_MINUTES: i64 = 15;
pub const MAX_NEGOTIATION_ROUNDS: i64 = 5;
pub const MAX_ACTIVE_NEGOTIATIONS_PER_GUEST: i64 = 3;
pub const MAX_ACTIVE_HOLDS_PER_GUEST: i64 = 2;
pub const MAX_ACTIVE_WAITLIST_PER_GUEST: i64 = 3;

/// Generate an opaque offer token (prefix `tt_offer_` + 32 hex chars = 128 bits CSPRNG).
/// Returns `(plain_token, token_hash)`.
pub fn generate_offer_token() -> (String, String) {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let hex_part: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let plain = format!("tt_offer_{}", hex_part);
    let hash = hash_token(&plain);
    (plain, hash)
}

/// Generate deterministic offer token for waitlist hold so caller of get_waitlist_status can receive it
/// without storing plain token in database.
pub fn generate_waitlist_offer_token(
    hold_id: i64,
    waitlist_code: &str,
    created_at: &str,
) -> (String, String) {
    let mut hasher = Sha256::new();
    hasher.update(b"tabletalk_waitlist_offer_salt:");
    hasher.update(hold_id.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(waitlist_code.as_bytes());
    hasher.update(b":");
    hasher.update(created_at.as_bytes());
    let result = hasher.finalize();
    let hex_part: String = result[..16].iter().map(|b| format!("{:02x}", b)).collect();
    let plain = format!("tt_offer_{}", hex_part);
    let hash = hash_token(&plain);
    (plain, hash)
}

/// Compute SHA-256 hash of a token as lowercase hex string.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Constant-time comparison between two byte slices to mitigate timing attacks.
pub fn constant_time_eq_slices(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Constant-time comparison of token hashes.
pub fn verify_token_hash(computed_hash: &str, stored_hash: &str) -> bool {
    constant_time_eq_slices(computed_hash.as_bytes(), stored_hash.as_bytes())
}

// --- Data Structures for Negotiation & Offers ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct NegotiationOption {
    pub option_id: String,
    pub start_local: String,
    pub end_local: String,
    pub area: String,
    pub table_label: Option<String>,
    pub minutes_from_requested: i64,
    pub score_reasons: Vec<String>,
    pub rank: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StoredOfferSlot {
    pub option_id: String,
    pub round: i64,
    pub table_id: i64,
    pub table_label: String,
    pub start_at: String, // UTC RFC3339
    pub end_at: String,   // UTC RFC3339
    pub start_local: String,
    pub end_local: String,
    pub area: String,
    pub party_size: i64,
    pub date: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct HoldSummary {
    pub hold_id: i64,
    pub expires_at_local: String,
    pub seconds_remaining: i64,
    pub area: String,
    pub start_local: String,
    pub end_local: String,
    pub table_label: String,
    pub party_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WaitlistSummary {
    pub code: String,
    pub guest_name: String,
    pub party_size: i64,
    pub date: String,
    pub window_start: String,
    pub window_end: String,
    pub area: String,
    pub status: String,
    pub queue_position: Option<i64>,
    pub expires_at_local: String,
    pub offer: Option<WaitlistOfferDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct WaitlistOfferDetail {
    pub offer_token: String,
    pub start_local: String,
    pub end_local: String,
    pub area: String,
    pub table_label: String,
    pub expires_at_local: String,
    pub seconds_remaining: i64,
}

// --- Scoring and Ranking Logic ---

#[derive(Debug, Clone)]
pub struct CandidateSlot {
    pub table_id: i64,
    pub table_label: String,
    pub capacity_min: i64,
    pub capacity_max: i64,
    pub area: Area,
    pub start_at: DateTime<Utc>,
    pub end_at: DateTime<Utc>,
    pub start_local_time: NaiveTime,
    pub end_local_time: NaiveTime,
    pub local_date: NaiveDate,
    pub minutes_from_closing: i64,
}

#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    pub candidate: CandidateSlot,
    pub score: i64,
    pub minutes_from_requested: i64,
    pub score_reasons: Vec<String>,
}

/// Deterministic integer scoring function.
/// Factors:
/// - Base: 1000
/// - Minutes from requested time (primary): -2 points per minute difference
/// - Preferred area match: +150 points; different area: -50 points
/// - Same day: +300 points; different day: -200 points - 50 points/day
/// - Party-size fit: -10 points per excess seat
/// - Stored seat preference: +75 points
/// - Last seating: -40 points if within 45 min of closing, +20 points otherwise
pub fn score_candidate(
    candidate: &CandidateSlot,
    requested_date: NaiveDate,
    requested_time: NaiveTime,
    requested_party_size: i64,
    requested_area: AreaFilter,
    profile_seat_pref: Option<&str>,
    last_seating_buffer_minutes: i64,
) -> ScoredCandidate {
    let mut score = 1000i64;
    let mut reasons = Vec::new();

    // 1. Same day vs different day
    let day_diff = (candidate.local_date - requested_date).num_days();
    if day_diff == 0 {
        score += 300;
        reasons.push("same_day".to_string());
    } else {
        score -= 200 + (day_diff.abs() * 50);
        reasons.push("different_day".to_string());
    }

    // 2. Minutes from requested time
    let minutes_from_requested = {
        let req_total_mins = requested_time.hour() as i64 * 60 + requested_time.minute() as i64;
        let cand_total_mins = candidate.start_local_time.hour() as i64 * 60
            + candidate.start_local_time.minute() as i64;
        (cand_total_mins - req_total_mins).abs()
    };

    score -= minutes_from_requested * 2;
    if minutes_from_requested == 0 {
        reasons.push("exact_time".to_string());
    } else {
        reasons.push("close_to_requested_time".to_string());
    }

    // 3. Area matching
    if requested_area != AreaFilter::Any {
        if requested_area.matches(candidate.area) {
            score += 150;
            reasons.push("preferred_area_match".to_string());
        } else {
            score -= 50;
            reasons.push("alternative_area".to_string());
        }
    }

    // 4. Party-size fit (waste of capacity)
    let excess_capacity = candidate.capacity_max - requested_party_size;
    score -= excess_capacity * 10;
    if excess_capacity == 0 {
        reasons.push("exact_capacity_fit".to_string());
    } else if excess_capacity <= 2 {
        reasons.push("good_capacity_fit".to_string());
    } else {
        reasons.push("larger_table".to_string());
    }

    // 5. Stored seat preference (soft match)
    if let Some(pref) = profile_seat_pref {
        if candidate.area.as_str().eq_ignore_ascii_case(pref) {
            score += 75;
            reasons.push("profile_seat_preference_matched".to_string());
        }
    }

    // 6. Avoiding last seating
    if candidate.minutes_from_closing <= last_seating_buffer_minutes {
        score -= 40;
        reasons.push("near_closing_time".to_string());
    } else {
        score += 20;
        reasons.push("comfortable_dining_window".to_string());
    }

    ScoredCandidate {
        candidate: candidate.clone(),
        score,
        minutes_from_requested,
        score_reasons: reasons,
    }
}

/// Sort scored candidates deterministically with tie-breaks:
/// 1. Score DESC
/// 2. Earlier start_at ASC
/// 3. Smaller capacity_max ASC
/// 4. Alphabetical table_label ASC
pub fn sort_scored_candidates(candidates: &mut [ScoredCandidate]) {
    candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.candidate.start_at.cmp(&b.candidate.start_at))
            .then_with(|| a.candidate.capacity_max.cmp(&b.candidate.capacity_max))
            .then_with(|| a.candidate.table_label.cmp(&b.candidate.table_label))
    });
}

// --- Voice Text Formatting Helpers ---

/// Format options for voice response: at most three options in natural language with ONE question.
/// Example: "Seven in the evening indoors, or seven thirty on the patio. Which would you like?"
pub fn format_spoken_options(options: &[NegotiationOption]) -> String {
    if options.is_empty() {
        return "I don't have any tables available around that time. Would you like to join the waitlist?".to_string();
    }

    let mut parts = Vec::new();
    for opt in options.iter().take(3) {
        let time_part = format_time_voice(&opt.start_local);
        let area_part = match opt.area.as_str() {
            "indoor" => "indoors",
            "patio" => "on the patio",
            "bar" => "at the bar",
            other => other,
        };
        parts.push(format!("{} {}", time_part, area_part));
    }

    let joined = match parts.len() {
        1 => parts[0].clone(),
        2 => format!("{}, or {}", parts[0], parts[1]),
        _ => format!("{}, {}, or {}", parts[0], parts[1], parts[2]),
    };

    // Capitalize first letter
    let mut chars = joined.chars();
    let capitalized = match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    };

    format!("{}. Which would you like?", capitalized)
}

/// Format hold voice text:
/// Example: "I'm holding the patio at seven thirty for five minutes. Shall I confirm it?"
pub fn format_spoken_hold(area: &str, start_local: &str, minutes: i64) -> String {
    let area_phrase = match area {
        "indoor" => "the indoor table",
        "patio" => "the patio",
        "bar" => "the bar",
        other => other,
    };
    let time_phrase = format_time_voice(start_local);
    let min_word = match minutes {
        1 => "one minute",
        2 => "two minutes",
        3 => "three minutes",
        4 => "four minutes",
        5 => "five minutes",
        10 => "ten minutes",
        n => &format!("{} minutes", n),
    };
    format!(
        "I'm holding {} at {} for {}. Shall I confirm it?",
        area_phrase, time_phrase, min_word
    )
}

/// Format waitlist offer voice text:
/// Example: "A table for eight on the patio at seven thirty has opened up from the waitlist. I'm holding it for five minutes. Would you like to confirm?"
pub fn format_spoken_waitlist_offer(
    party_size: i64,
    area: &str,
    start_local: &str,
    minutes: i64,
) -> String {
    let party_phrase = match party_size {
        1 => "one person".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        n => n.to_string(),
    };
    let area_phrase = match area {
        "indoor" => "indoors",
        "patio" => "on the patio",
        "bar" => "at the bar",
        other => other,
    };
    let time_phrase = format_time_voice(start_local);
    let min_word = match minutes {
        1 => "one minute",
        2 => "two minutes",
        5 => "five minutes",
        n => &format!("{} minutes", n),
    };
    format!(
        "A table for {} {} at {} has opened up from the waitlist. I'm holding it for {}. Would you like to confirm?",
        party_phrase, area_phrase, time_phrase, min_word
    )
}

fn format_time_voice(time_str: &str) -> String {
    if let Ok(naive) = NaiveTime::parse_from_str(time_str, "%H:%M") {
        let hour = naive.hour();
        let minute = naive.minute();
        let (h12, ampm) = if hour == 0 {
            (12, "at night")
        } else if hour < 12 {
            (hour, "in the morning")
        } else if hour == 12 {
            (12, "in the afternoon")
        } else if hour < 17 {
            (hour - 12, "in the afternoon")
        } else {
            (hour - 12, "in the evening")
        };

        let h_word = match h12 {
            1 => "one",
            2 => "two",
            3 => "three",
            4 => "four",
            5 => "five",
            6 => "six",
            7 => "seven",
            8 => "eight",
            9 => "nine",
            10 => "ten",
            11 => "eleven",
            12 => "twelve",
            _ => "unknown",
        };

        if minute == 0 {
            format!("{} {}", h_word, ampm)
        } else if minute == 15 {
            format!("{} fifteen", h_word)
        } else if minute == 30 {
            format!("{} thirty", h_word)
        } else if minute == 45 {
            format!("{} forty five", h_word)
        } else {
            format!("{} {:02}", h_word, minute)
        }
    } else {
        time_str.to_string()
    }
}

/// Format waitlist code as spaced characters ("W L 1 2 3 4 5").
pub fn format_spoken_waitlist_code(code: &str) -> String {
    code.chars()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

// --- Waitlist Matcher Engine ---

/// Match the first fitting waiting waitlist entry for a freed dining table and slot.
/// Must be run inside an open write transaction (`conn`).
/// Converts at most ONE fitting entry to 'offered' status and creates an active hold.
#[allow(clippy::too_many_arguments)]
pub async fn match_waitlist_for_slot(
    conn: &mut SqliteConnection,
    freed_table_id: i64,
    freed_start_at: &str,
    freed_end_at: &str,
    _freed_area: &str,
    target_date: &str,
    slot_start_time: &str, // HH:MM
    clock: &dyn Clock,
) -> Result<Option<WaitlistEntry>, sqlx::Error> {
    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();

    // Query waiting entries for that date ordered FIFO by created_at, id
    let entries: Vec<WaitlistEntry> = sqlx::query_as(
        r#"
        SELECT id, code, guest_name, contact, party_size, date,
               window_start, window_end, area, duration_minutes, status,
               customer_id, created_at, expires_at
        FROM waitlist_entries
        WHERE date = ?
          AND status = 'waiting'
          AND expires_at > ?
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(target_date)
    .bind(&now_iso)
    .fetch_all(&mut *conn)
    .await?;

    if entries.is_empty() {
        return Ok(None);
    }

    // Load freed table capacity
    let table: Option<(i64, i64, String)> = sqlx::query_as(
        "SELECT capacity_min, capacity_max, area FROM dining_tables WHERE id = ? AND is_active = 1",
    )
    .bind(freed_table_id)
    .fetch_optional(&mut *conn)
    .await?;

    let (cap_min, cap_max, table_area) = match table {
        Some(t) => t,
        None => return Ok(None),
    };

    let slot_time = match NaiveTime::parse_from_str(slot_start_time, "%H:%M") {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };

    // FIFO evaluation: inspect each waiting entry in strict created_at order
    for entry in entries {
        // 1. Capacity check
        if entry.party_size < cap_min || entry.party_size > cap_max {
            continue; // Skip too-large or too-small entry without affecting its position
        }

        // 2. Area check
        if entry.area != "any" && !entry.area.eq_ignore_ascii_case(&table_area) {
            continue;
        }

        // 3. Window check
        let win_start = match NaiveTime::parse_from_str(&entry.window_start, "%H:%M") {
            Ok(t) => t,
            Err(_) => continue,
        };
        let win_end = match NaiveTime::parse_from_str(&entry.window_end, "%H:%M") {
            Ok(t) => t,
            Err(_) => continue,
        };

        if slot_time < win_start || slot_time > win_end {
            continue;
        }

        // Check if table is actually free right now for [freed_start_at, freed_end_at)
        let overlapping_bookings: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bookings
            WHERE table_id = ?
              AND status IN ('pending', 'confirmed')
              AND start_at < ?
              AND end_at > ?
            "#,
        )
        .bind(freed_table_id)
        .bind(freed_end_at)
        .bind(freed_start_at)
        .fetch_one(&mut *conn)
        .await?;

        if overlapping_bookings > 0 {
            return Ok(None);
        }

        let overlapping_holds: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM slot_holds
            WHERE table_id = ?
              AND status = 'active'
              AND expires_at > ?
              AND start_at < ?
              AND end_at > ?
            "#,
        )
        .bind(freed_table_id)
        .bind(&now_iso)
        .bind(freed_end_at)
        .bind(freed_start_at)
        .fetch_one(&mut *conn)
        .await?;

        if overlapping_holds > 0 {
            return Ok(None);
        }

        // Matched! Create hold for this waitlist entry
        let hold_expires = now_utc + Duration::minutes(DEFAULT_HOLD_MINUTES);
        let hold_expires_iso = hold_expires.to_rfc3339();

        // Generate deterministic offer token bound to hold
        // First insert hold with dummy hash to get hold_id
        let dummy_hash = format!("pending_waitlist_{}_{}", entry.id, now_iso);
        let hold_id = sqlx::query(
            r#"
            INSERT INTO slot_holds (
                table_id, start_at, end_at, status, expires_at,
                guest_name, party_size, area, token_hash,
                waitlist_entry_id, created_at, updated_at
            ) VALUES (?, ?, ?, 'active', ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(freed_table_id)
        .bind(freed_start_at)
        .bind(freed_end_at)
        .bind(&hold_expires_iso)
        .bind(&entry.guest_name)
        .bind(entry.party_size)
        .bind(&table_area)
        .bind(&dummy_hash)
        .bind(entry.id)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?
        .last_insert_rowid();

        let (_plain_token, real_hash) =
            generate_waitlist_offer_token(hold_id, &entry.code, &now_iso);
        sqlx::query("UPDATE slot_holds SET token_hash = ? WHERE id = ?")
            .bind(&real_hash)
            .bind(hold_id)
            .execute(&mut *conn)
            .await?;

        // Update waitlist entry to 'offered'
        sqlx::query("UPDATE waitlist_entries SET status = 'offered' WHERE id = ?")
            .bind(entry.id)
            .execute(&mut *conn)
            .await?;

        let mut matched_entry = entry;
        matched_entry.status = WaitlistStatus::Offered;
        return Ok(Some(matched_entry));
    }

    Ok(None)
}

// --- Sweeper Function for Expired Holds, Negotiations, and Waitlist Entries ---

pub async fn sweep_expired_holds_and_negotiations(
    pool: &SqlitePool,
    clock: &dyn Clock,
) -> Result<usize, sqlx::Error> {
    let now_iso = clock.now().to_rfc3339();
    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    // 1. Find active holds that expired
    let expired_holds: Vec<(i64, Option<i64>, i64, String, String, String)> = sqlx::query_as(
        r#"
        SELECT id, waitlist_entry_id, table_id, start_at, end_at, area
        FROM slot_holds
        WHERE status = 'active' AND expires_at <= ?
        "#,
    )
    .bind(&now_iso)
    .fetch_all(&mut *conn)
    .await?;

    let mut swept_count = 0;
    for (hold_id, waitlist_entry_id, table_id, start_at, end_at, area) in expired_holds {
        sqlx::query("UPDATE slot_holds SET status = 'expired', updated_at = ? WHERE id = ?")
            .bind(&now_iso)
            .bind(hold_id)
            .execute(&mut *conn)
            .await?;
        swept_count += 1;

        // If this hold was linked to a waitlist entry, revert entry to waiting at original position!
        if let Some(w_id) = waitlist_entry_id {
            sqlx::query("UPDATE waitlist_entries SET status = 'waiting' WHERE id = ? AND status = 'offered'")
                .bind(w_id)
                .execute(&mut *conn)
                .await?;

            // Try to match next waiting entry for the freed slot
            // Extract local date and time from start_at UTC
            if let Ok(dt) = DateTime::parse_from_rfc3339(&start_at) {
                let date_str = dt.format("%Y-%m-%d").to_string();
                let time_str = dt.format("%H:%M").to_string();
                let _ = match_waitlist_for_slot(
                    &mut conn, table_id, &start_at, &end_at, &area, &date_str, &time_str, clock,
                )
                .await;
            }
        }
    }

    // 2. Expire open negotiations
    let expired_negs = sqlx::query(
        "UPDATE negotiations SET status = 'expired' WHERE status = 'open' AND expires_at <= ?",
    )
    .bind(&now_iso)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    swept_count += expired_negs as usize;

    // 3. Expire waiting waitlist entries
    let expired_wl = sqlx::query(
        "UPDATE waitlist_entries SET status = 'expired' WHERE status = 'waiting' AND expires_at <= ?",
    )
    .bind(&now_iso)
    .execute(&mut *conn)
    .await?
    .rows_affected();
    swept_count += expired_wl as usize;

    sqlx::query("COMMIT").execute(&mut *conn).await?;
    Ok(swept_count)
}
