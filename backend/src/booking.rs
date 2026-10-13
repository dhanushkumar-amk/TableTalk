use crate::{
    availability::{
        check_availability_engine, format_spoken_date, format_spoken_time,
        generate_availability_voice_text, AlternativeSlot, AreaFilter, AvailabilityConfig,
        AvailabilityQuery, Clock,
    },
    db::models::{
        Area, Booking, BookingStatus, DiningTable, OpeningHours, RestaurantConfig,
        SpecialClosure,
    },
};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Mutex, time::Instant};

// --- Confirmation Code Alphabet & Constants ---

/// Safe alphanumeric alphabet avoiding ambiguous characters (no '0', 'O', '1', 'I', 'L').
pub const CONFIRMATION_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
pub const CONFIRMATION_CODE_PREFIX: &str = "TT-";
pub const CONFIRMATION_CODE_BODY_LEN: usize = 5;

pub const MAX_ACTIVE_BOOKINGS_PER_GUEST: i64 = 3;
pub const MAX_CANCEL_ATTEMPTS: u32 = 5;
pub const CANCEL_ATTEMPT_WINDOW_SECS: u64 = 900; // 15 minutes
pub const IDEMPOTENCY_KEY_EXPIRY_HOURS: i64 = 24;

// --- State Machine ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TransitionError {
    #[error("Illegal booking status transition from {from:?} to {to:?}")]
    IllegalTransition {
        from: BookingStatus,
        to: BookingStatus,
    },
}

/// Validates whether a booking status transition is legal.
/// Allowed transitions:
/// - Pending -> Confirmed
/// - Pending -> Cancelled
/// - Confirmed -> Completed
/// - Confirmed -> NoShow
/// - Confirmed -> Cancelled
/// - Cancelled -> Cancelled (idempotent self-transition)
pub fn can_transition(from: BookingStatus, to: BookingStatus) -> bool {
    matches!(
        (from, to),
        (BookingStatus::Pending, BookingStatus::Confirmed)
            | (BookingStatus::Pending, BookingStatus::Cancelled)
            | (BookingStatus::Confirmed, BookingStatus::Completed)
            | (BookingStatus::Confirmed, BookingStatus::NoShow)
            | (BookingStatus::Confirmed, BookingStatus::Cancelled)
            | (BookingStatus::Cancelled, BookingStatus::Cancelled)
    )
}

/// Applies a status transition, returning an error on illegal transitions.
pub fn apply_transition(
    current: BookingStatus,
    target: BookingStatus,
) -> Result<BookingStatus, TransitionError> {
    if can_transition(current, target) {
        Ok(target)
    } else {
        Err(TransitionError::IllegalTransition {
            from: current,
            to: target,
        })
    }
}

// --- Confirmation Code Generator & Normalizer ---

/// Generates a random confirmation code in the format `TT-XXXXX` using the safe alphabet.
pub fn generate_confirmation_code() -> String {
    let mut rng = rand::thread_rng();
    let mut body = String::with_capacity(CONFIRMATION_CODE_BODY_LEN);
    for _ in 0..CONFIRMATION_CODE_BODY_LEN {
        let idx = rng.gen_range(0..CONFIRMATION_CODE_ALPHABET.len());
        body.push(CONFIRMATION_CODE_ALPHABET[idx] as char);
    }
    format!("{}{}", CONFIRMATION_CODE_PREFIX, body)
}

/// Normalizes an input confirmation code:
/// - Trims and converts to uppercase.
/// - Strips spaces, hyphens, and underscores.
/// - Ensures the `TT-` prefix is present.
/// Tolerates inputs like `tt 8k4mz`, `8k4mz`, `TT-8K4MZ`.
pub fn normalize_confirmation_code(input: &str) -> String {
    let upper = input.trim().to_ascii_uppercase();
    let clean: String = upper
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();

    if clean.starts_with("TT") {
        let body = &clean[2..];
        format!("{}{}", CONFIRMATION_CODE_PREFIX, body)
    } else {
        format!("{}{}", CONFIRMATION_CODE_PREFIX, clean)
    }
}

/// Formats a confirmation code for natural speech (e.g. `TT-8K4MZ` -> `"T T 8 K 4 M Z"`).
pub fn format_spoken_code(code: &str) -> String {
    let clean: Vec<char> = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    clean
        .into_iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(" ")
}

// --- Validation ---

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BookingValidationError {
    #[error("Invalid date '{0}'. Must be in ISO format YYYY-MM-DD (e.g. '2026-10-17').")]
    InvalidDate(String),
    #[error("Invalid time '{0}'. Must be in 24-hour HH:MM format (e.g. '19:30').")]
    InvalidTime(String),
    #[error("Party size must be between 1 and {0} guests. Received {1}.")]
    InvalidPartySize(i64, i64),
    #[error("Guest name is required and must be between 1 and 80 characters.")]
    InvalidGuestName,
    #[error("Special requests exceed maximum length of 300 characters.")]
    SpecialRequestsTooLong,
    #[error("Invalid idempotency key '{0}'. Must be 8-64 characters matching [a-zA-Z0-9_-].")]
    InvalidIdempotencyKey(String),
    #[error("Invalid phone number format.")]
    InvalidPhone,
    #[error("Invalid email address format.")]
    InvalidEmail,
}

pub fn validate_idempotency_key(key: &str) -> bool {
    let len = key.len();
    if !(8..=64).contains(&len) {
        return false;
    }
    key.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

pub fn validate_create_booking_request(
    date_str: &str,
    time_str: &str,
    party_size: i64,
    guest_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    special_requests: Option<&str>,
    idempotency_key: Option<&str>,
    max_party_size: i64,
) -> Result<(NaiveDate, NaiveTime), BookingValidationError> {
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| BookingValidationError::InvalidDate(date_str.to_string()))?;

    let time = NaiveTime::parse_from_str(time_str, "%H:%M")
        .map_err(|_| BookingValidationError::InvalidTime(time_str.to_string()))?;

    if party_size < 1 || party_size > max_party_size {
        return Err(BookingValidationError::InvalidPartySize(
            max_party_size,
            party_size,
        ));
    }

    let trimmed_name = guest_name.trim();
    if trimmed_name.is_empty() || trimmed_name.len() > 80 {
        return Err(BookingValidationError::InvalidGuestName);
    }

    if let Some(reqs) = special_requests {
        if reqs.len() > 300 {
            return Err(BookingValidationError::SpecialRequestsTooLong);
        }
    }

    if let Some(key) = idempotency_key {
        if !validate_idempotency_key(key) {
            return Err(BookingValidationError::InvalidIdempotencyKey(
                key.to_string(),
            ));
        }
    }

    if let Some(p) = phone {
        let p_trimmed = p.trim();
        if !p_trimmed.is_empty() {
            let digit_count = p_trimmed.chars().filter(|c| c.is_ascii_digit()).count();
            if !(5..=20).contains(&digit_count)
                || !p_trimmed.chars().all(|c| {
                    c.is_ascii_digit() || c == '+' || c == '-' || c == ' ' || c == '(' || c == ')'
                })
            {
                return Err(BookingValidationError::InvalidPhone);
            }
        }
    }

    if let Some(e) = email {
        let e_trimmed = e.trim();
        if !e_trimmed.is_empty() {
            if !(3..=100).contains(&e_trimmed.len())
                || !e_trimmed.contains('@')
                || e_trimmed.starts_with('@')
                || e_trimmed.ends_with('@')
            {
                return Err(BookingValidationError::InvalidEmail);
            }
        }
    }

    Ok((date, time))
}

// --- Idempotency Hashing ---

#[derive(Debug, Clone, Serialize)]
struct CanonicalBookingRequest<'a> {
    date: &'a str,
    time: &'a str,
    party_size: i64,
    guest_name: &'a str,
    phone: Option<&'a str>,
    email: Option<&'a str>,
    area: Option<&'a str>,
    duration_minutes: Option<i64>,
    special_requests: Option<&'a str>,
}

pub fn compute_request_hash(
    date: &str,
    time: &str,
    party_size: i64,
    guest_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    area: Option<&str>,
    duration_minutes: Option<i64>,
    special_requests: Option<&str>,
) -> String {
    let canonical = CanonicalBookingRequest {
        date,
        time,
        party_size,
        guest_name: guest_name.trim(),
        phone: phone.map(|s| s.trim()),
        email: email.map(|s| s.trim()),
        area,
        duration_minutes,
        special_requests: special_requests.map(|s| s.trim()),
    };
    let json_bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&json_bytes);
    format!("{:x}", hasher.finalize())
}

// --- Cancellation Rate Limiter ---

#[derive(Debug, Default)]
pub struct CancellationRateLimiter {
    attempts: Mutex<HashMap<String, (u32, Instant)>>,
}

impl CancellationRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
        }
    }

    /// Checks if a confirmation code has exceeded failed verification attempts.
    /// Returns true if rate limited.
    pub fn is_rate_limited(&self, code: &str) -> bool {
        let mut map = self.attempts.lock().unwrap();
        if let Some((count, first_time)) = map.get(code) {
            if first_time.elapsed().as_secs() > CANCEL_ATTEMPT_WINDOW_SECS {
                map.remove(code);
                false
            } else {
                *count >= MAX_CANCEL_ATTEMPTS
            }
        } else {
            false
        }
    }

    pub fn record_failure(&self, code: &str) {
        let mut map = self.attempts.lock().unwrap();
        let now = Instant::now();
        if let Some((count, first_time)) = map.get_mut(code) {
            if first_time.elapsed().as_secs() > CANCEL_ATTEMPT_WINDOW_SECS {
                *count = 1;
                *first_time = now;
            } else {
                *count += 1;
            }
        } else {
            map.insert(code.to_string(), (1, now));
        }
    }

    pub fn record_success(&self, code: &str) {
        let mut map = self.attempts.lock().unwrap();
        map.remove(code);
    }
}

// --- Booking Domain Types & Outcomes ---

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BookingSummary {
    pub confirmation_code: String,
    pub guest_name: String,
    pub party_size: i64,
    pub start_local: String,
    pub end_local: String,
    pub area: String,
    pub table_label: String,
    pub status: String,
    pub special_requests: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CreateBookingOutput {
    pub status: String,
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
    pub booking: Option<BookingSummary>,
    pub idempotent_replay: bool,
    pub alternatives: Vec<AlternativeSlot>,
    pub cancellation_policy: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CancelBookingOutput {
    pub status: String,
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
    pub booking: Option<BookingSummary>,
    pub already_cancelled: bool,
    pub policy_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetBookingOutput {
    pub status: String,
    pub booking: Option<BookingSummary>,
    pub message: String,
}

#[derive(Debug)]
pub enum CreateBookingOutcome {
    Created {
        output: CreateBookingOutput,
        voice_text: String,
    },
    IdempotentReplay {
        output: CreateBookingOutput,
        voice_text: String,
    },
    Unavailable {
        output: CreateBookingOutput,
        voice_text: String,
    },
    LimitExceeded {
        output: CreateBookingOutput,
        voice_text: String,
    },
    IdempotencyConflict {
        output: CreateBookingOutput,
        voice_text: String,
    },
}

#[derive(Debug)]
pub enum CancelBookingOutcome {
    Cancelled {
        output: CancelBookingOutput,
        voice_text: String,
    },
    AlreadyCancelled {
        output: CancelBookingOutput,
        voice_text: String,
    },
    NotFound {
        output: CancelBookingOutput,
        voice_text: String,
    },
    NotCancellable {
        output: CancelBookingOutput,
        voice_text: String,
    },
    LimitExceeded {
        output: CancelBookingOutput,
        voice_text: String,
    },
}

#[derive(Debug)]
pub enum GetBookingOutcome {
    Found {
        output: GetBookingOutput,
        voice_text: String,
    },
    NotFound {
        output: GetBookingOutput,
        voice_text: String,
    },
}

// --- Request DTOs ---

pub struct CreateBookingTxRequest<'a> {
    pub date: &'a str,
    pub time: &'a str,
    pub party_size: i64,
    pub guest_name: &'a str,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    pub area: Option<&'a str>,
    pub duration_minutes: Option<i64>,
    pub special_requests: Option<&'a str>,
    pub idempotency_key: Option<&'a str>,
    pub customer_id: Option<i64>,
}

pub struct CancelBookingTxRequest<'a> {
    pub confirmation_code: &'a str,
    pub guest_name: &'a str,
    pub reason: Option<&'a str>,
}

// --- Transactional Operations ---

/// Creates a booking inside an immediate write transaction (`BEGIN IMMEDIATE`).
/// Re-evaluates availability engine against the locked database state, ensuring
/// zero race conditions under high concurrency.
pub async fn create_booking_tx(
    pool: &SqlitePool,
    req: CreateBookingTxRequest<'_>,
    engine_config: &AvailabilityConfig,
    clock: &dyn Clock,
) -> Result<CreateBookingOutcome, sqlx::Error> {
    let now = clock.now();
    let now_utc_iso = now.to_rfc3339();

    // 1. Acquire connection and start exclusive write transaction
    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    // Closure to run transactional logic with automatic rollback on error
    let res: Result<CreateBookingOutcome, sqlx::Error> = async {
        // 2. Load restaurant config inside transaction
        let restaurant: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents
            FROM restaurant
            WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;

        // 3. Check idempotency key if provided
        let request_hash = compute_request_hash(
            req.date,
            req.time,
            req.party_size,
            req.guest_name,
            req.phone,
            req.email,
            req.area,
            req.duration_minutes,
            req.special_requests,
        );

        if let Some(key) = req.idempotency_key {
            let existing_record: Option<(String, String)> = sqlx::query_as(
                "SELECT request_hash, response_json FROM idempotency_keys WHERE key = ?",
            )
            .bind(key)
            .fetch_optional(&mut *conn)
            .await?;

            if let Some((stored_hash, cached_json)) = existing_record {
                if stored_hash == request_hash {
                    // Identical request retry -> return cached output with idempotent_replay: true
                    if let Ok(mut cached_out) =
                        serde_json::from_str::<CreateBookingOutput>(&cached_json)
                    {
                        cached_out.idempotent_replay = true;
                        let voice_text = generate_create_booking_voice_text(&cached_out);
                        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                        return Ok(CreateBookingOutcome::IdempotentReplay {
                            output: cached_out,
                            voice_text,
                        });
                    }
                } else {
                    // Same key with different parameters -> conflict!
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                    let output = CreateBookingOutput {
                        status: "idempotency_conflict".to_string(),
                        reason: Some("idempotency_key_reused_with_different_parameters".to_string()),
                        reason_detail: Some(format!(
                            "Idempotency key '{}' was previously used with different booking parameters.",
                            key
                        )),
                        booking: None,
                        idempotent_replay: false,
                        alternatives: vec![],
                        cancellation_policy: None,
                        note: "availability is a snapshot, not a reservation".to_string(),
                    };
                    let voice_text = "This idempotency key was previously used with different booking details. Please use a new key or check your request.".to_string();
                    return Ok(CreateBookingOutcome::IdempotencyConflict {
                        output,
                        voice_text,
                    });
                }
            }
        }

        // 4. Abuse prevention: Check active future bookings count for guest name / phone / email
        let active_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM bookings
            WHERE status IN ('pending', 'confirmed')
              AND start_at >= ?
              AND (
                LOWER(TRIM(guest_name)) = LOWER(TRIM(?))
                OR (? IS NOT NULL AND guest_phone IS NOT NULL AND guest_phone = ?)
                OR (? IS NOT NULL AND guest_email IS NOT NULL AND LOWER(guest_email) = LOWER(?))
              )
            "#,
        )
        .bind(&now_utc_iso)
        .bind(req.guest_name)
        .bind(req.phone)
        .bind(req.phone)
        .bind(req.email)
        .bind(req.email)
        .fetch_one(&mut *conn)
        .await?;

        if active_count >= MAX_ACTIVE_BOOKINGS_PER_GUEST {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = CreateBookingOutput {
                status: "limit_exceeded".to_string(),
                reason: Some("max_active_bookings_exceeded".to_string()),
                reason_detail: Some(format!(
                    "Guest '{}' already has {} active upcoming reservations, which is the maximum allowed. Please cancel an existing reservation before booking another.",
                    req.guest_name, active_count
                )),
                booking: None,
                idempotent_replay: false,
                alternatives: vec![],
                cancellation_policy: None,
                note: "availability is a snapshot, not a reservation".to_string(),
            };
            let voice_text = format!(
                "You already have {} active reservations, which is the maximum allowed. Please cancel an existing booking before making a new one.",
                active_count
            );
            return Ok(CreateBookingOutcome::LimitExceeded { output, voice_text });
        }

        // 5. Load remaining operational data for availability evaluation
        let opening_hours: Vec<OpeningHours> = sqlx::query_as(
            "SELECT id, day_of_week, period_name, open_time, close_time, is_closed FROM opening_hours ORDER BY day_of_week ASC, open_time ASC"
        )
        .fetch_all(&mut *conn)
        .await?;

        let special_closures: Vec<SpecialClosure> = sqlx::query_as(
            "SELECT id, closure_date, reason, is_all_day, open_time, close_time FROM special_closures ORDER BY closure_date ASC"
        )
        .fetch_all(&mut *conn)
        .await?;

        let tables: Vec<DiningTable> = sqlx::query_as(
            "SELECT id, label, capacity_min, capacity_max, area, is_active, created_at FROM dining_tables WHERE is_active = 1 ORDER BY capacity_max ASC, id ASC"
        )
        .fetch_all(&mut *conn)
        .await?;

        // Determine relevant search range for bookings on the target day
        let target_date = NaiveDate::parse_from_str(req.date, "%Y-%m-%d").map_err(|e| {
            sqlx::Error::Protocol(format!("Invalid date format: {}", e))
        })?;
        let search_start = target_date - Duration::days(1);
        let search_end = target_date + Duration::days(2);
        let search_start_iso = search_start.and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339();
        let search_end_iso = search_end.and_hms_opt(0, 0, 0).unwrap().and_utc().to_rfc3339();

        let day_bookings: Vec<Booking> = sqlx::query_as(
            r#"
            SELECT id, confirmation_code, customer_id, guest_name, party_size,
                   start_at, end_at, status, table_id, special_requests,
                   deposit_status, deposit_amount_cents, created_at, updated_at, cancelled_at,
                   guest_phone, guest_email
            FROM bookings
            WHERE status IN ('pending', 'confirmed')
              AND start_at < ?
              AND end_at > ?
            ORDER BY start_at ASC
            "#,
        )
        .bind(&search_end_iso)
        .bind(&search_start_iso)
        .fetch_all(&mut *conn)
        .await?;

        // 6. Run pure availability engine inside the lock
        let parsed_time = NaiveTime::parse_from_str(req.time, "%H:%M")
            .map_err(|e| sqlx::Error::Protocol(format!("Invalid time format: {}", e)))?;
        let area_filter = match req.area {
            Some("indoor") => AreaFilter::Indoor,
            Some("patio") => AreaFilter::Patio,
            Some("bar") => AreaFilter::Bar,
            _ => AreaFilter::Any,
        };

        let avail_query = AvailabilityQuery {
            date: target_date,
            time: parsed_time,
            party_size: req.party_size,
            area: area_filter,
            duration_minutes: req.duration_minutes,
            alternatives_limit: 3,
        };

        let avail_res = check_availability_engine(
            avail_query,
            &restaurant,
            &opening_hours,
            &special_closures,
            &tables,
            &day_bookings,
            engine_config,
            clock,
        );

        if avail_res.status != "available" || avail_res.slot.is_none() {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = CreateBookingOutput {
                status: avail_res.status.clone(),
                reason: avail_res.reason.clone(),
                reason_detail: avail_res.reason_detail.clone(),
                booking: None,
                idempotent_replay: false,
                alternatives: avail_res.alternatives.clone(),
                cancellation_policy: None,
                note: "availability is a snapshot, not a reservation".to_string(),
            };
            let voice_text = generate_availability_voice_text(&avail_res);
            return Ok(CreateBookingOutcome::Unavailable { output, voice_text });
        }

        let assigned_slot = avail_res.slot.unwrap();

        // Find table ID corresponding to assigned label
        let table_id: i64 = tables
            .iter()
            .find(|t| t.label == assigned_slot.table_label)
            .map(|t| t.id)
            .ok_or_else(|| {
                sqlx::Error::Protocol("Assigned table not found in restaurant tables".to_string())
            })?;

        // Convert slot local times to stored UTC strings
        let rest_tz: Tz = restaurant
            .timezone
            .parse()
            .map_err(|_| sqlx::Error::Protocol("Invalid timezone".to_string()))?;
        let start_local_naive = DateTime::parse_from_rfc3339(&assigned_slot.start_local)
            .map_err(|e| sqlx::Error::Protocol(format!("Invalid start_local: {}", e)))?
            .naive_local();
        let end_local_naive = DateTime::parse_from_rfc3339(&assigned_slot.end_local)
            .map_err(|e| sqlx::Error::Protocol(format!("Invalid end_local: {}", e)))?
            .naive_local();

        let start_utc_iso = rest_tz
            .from_local_datetime(&start_local_naive)
            .unwrap()
            .with_timezone(&Utc)
            .to_rfc3339();
        let end_utc_iso = rest_tz
            .from_local_datetime(&end_local_naive)
            .unwrap()
            .with_timezone(&Utc)
            .to_rfc3339();

        // 7. Generate collision-free confirmation code (retry on rare collision)
        let mut confirmation_code = generate_confirmation_code();
        for _ in 0..5 {
            let collision: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM bookings WHERE confirmation_code = ?",
            )
            .bind(&confirmation_code)
            .fetch_one(&mut *conn)
            .await?;

            if collision == 0 {
                break;
            }
            confirmation_code = generate_confirmation_code();
        }

        let booking_id = uuid::Uuid::new_v4().to_string();

        // 8. Insert new booking row
        sqlx::query(
            r#"
            INSERT INTO bookings (
                id, confirmation_code, customer_id, guest_name, party_size,
                start_at, end_at, status, table_id, special_requests,
                deposit_status, deposit_amount_cents, created_at, updated_at,
                guest_phone, guest_email
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, 'confirmed', ?, ?, 'none', 0, ?, ?, ?, ?)
            "#,
        )
        .bind(&booking_id)
        .bind(&confirmation_code)
        .bind(req.customer_id)
        .bind(req.guest_name.trim())
        .bind(req.party_size)
        .bind(&start_utc_iso)
        .bind(&end_utc_iso)
        .bind(table_id)
        .bind(req.special_requests.map(|s| s.trim()))
        .bind(&now_utc_iso)
        .bind(&now_utc_iso)
        .bind(req.phone.map(|s| s.trim()))
        .bind(req.email.map(|s| s.trim()))
        .execute(&mut *conn)
        .await?;

        // 9. Write audit event
        let audit_payload = serde_json::json!({
            "confirmation_code": confirmation_code,
            "party_size": req.party_size,
            "start_local": assigned_slot.start_local,
            "end_local": assigned_slot.end_local,
            "table_label": assigned_slot.table_label,
            "area": assigned_slot.area,
        });

        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('booking', ?, 'created', 'mcp', ?, ?)
            "#,
        )
        .bind(&booking_id)
        .bind(audit_payload.to_string())
        .bind(&now_utc_iso)
        .execute(&mut *conn)
        .await?;

        // 10. Assemble success output
        let cancellation_policy = format!(
            "Free cancellation up to {} hours prior to reservation.",
            restaurant.cancellation_cutoff_hours
        );

        let booking_summary = BookingSummary {
            confirmation_code: confirmation_code.clone(),
            guest_name: req.guest_name.trim().to_string(),
            party_size: req.party_size,
            start_local: assigned_slot.start_local.clone(),
            end_local: assigned_slot.end_local.clone(),
            area: assigned_slot.area.clone(),
            table_label: assigned_slot.table_label.clone(),
            status: "confirmed".to_string(),
            special_requests: req.special_requests.map(|s| s.trim().to_string()),
            created_at: now_utc_iso.clone(),
        };

        let output = CreateBookingOutput {
            status: "confirmed".to_string(),
            reason: None,
            reason_detail: None,
            booking: Some(booking_summary),
            idempotent_replay: false,
            alternatives: vec![],
            cancellation_policy: Some(cancellation_policy),
            note: "availability is a snapshot, not a reservation".to_string(),
        };

        // 11. Record idempotency key if provided
        if let Some(key) = req.idempotency_key {
            let key_id = uuid::Uuid::new_v4().to_string();
            let expires_at_iso = (now + Duration::hours(IDEMPOTENCY_KEY_EXPIRY_HOURS)).to_rfc3339();
            let response_json = serde_json::to_string(&output).unwrap_or_default();

            sqlx::query(
                r#"
                INSERT INTO idempotency_keys (id, key, tool_name, request_hash, response_json, booking_id, created_at, expires_at)
                VALUES (?, ?, 'create_booking', ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&key_id)
            .bind(key)
            .bind(&request_hash)
            .bind(&response_json)
            .bind(&booking_id)
            .bind(&now_utc_iso)
            .bind(&expires_at_iso)
            .execute(&mut *conn)
            .await?;
        }

        // 12. Commit transaction
        sqlx::query("COMMIT").execute(&mut *conn).await?;

        let voice_text = generate_create_booking_voice_text(&output);
        Ok(CreateBookingOutcome::Created { output, voice_text })
    }
    .await;

    match res {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(err)
        }
    }
}

/// Cancels an existing booking inside an immediate transaction.
/// Enforces case-insensitive guest_name verification to prevent enumeration.
pub async fn cancel_booking_tx(
    pool: &SqlitePool,
    req: CancelBookingTxRequest<'_>,
    rate_limiter: &CancellationRateLimiter,
    clock: &dyn Clock,
) -> Result<CancelBookingOutcome, sqlx::Error> {
    let normalized_code = normalize_confirmation_code(req.confirmation_code);
    let now = clock.now();
    let now_utc_iso = now.to_rfc3339();

    // 1. Check rate limiter
    if rate_limiter.is_rate_limited(&normalized_code) {
        let output = CancelBookingOutput {
            status: "not_found".to_string(),
            reason: Some("rate_limit_exceeded".to_string()),
            reason_detail: Some(
                "Too many failed verification attempts for this reservation. Please try again later."
                    .to_string(),
            ),
            booking: None,
            already_cancelled: false,
            policy_note: None,
        };
        let voice_text =
            "Too many failed verification attempts for this reservation. Please try again later."
                .to_string();
        return Ok(CancelBookingOutcome::LimitExceeded { output, voice_text });
    }

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<CancelBookingOutcome, sqlx::Error> = async {
        // 2. Fetch booking and restaurant config
        let booking_opt: Option<Booking> = sqlx::query_as(
            r#"
            SELECT id, confirmation_code, customer_id, guest_name, party_size,
                   start_at, end_at, status, table_id, special_requests,
                   deposit_status, deposit_amount_cents, created_at, updated_at, cancelled_at,
                   guest_phone, guest_email
            FROM bookings
            WHERE confirmation_code = ?
            "#,
        )
        .bind(&normalized_code)
        .fetch_optional(&mut *conn)
        .await?;

        // 3. Verification check: does booking exist and does guest_name match?
        let verified = if let Some(ref b) = booking_opt {
            b.guest_name
                .trim()
                .eq_ignore_ascii_case(req.guest_name.trim())
        } else {
            false
        };

        if !verified {
            rate_limiter.record_failure(&normalized_code);
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;

            // Identical generic response for unknown code and mismatched guest name
            let output = CancelBookingOutput {
                status: "not_found".to_string(),
                reason: Some("not_found".to_string()),
                reason_detail: Some(
                    "No matching booking found for the provided confirmation code and guest name. Please check your confirmation details."
                        .to_string(),
                ),
                booking: None,
                already_cancelled: false,
                policy_note: None,
            };
            let voice_text = "I couldn't find a booking with that confirmation code and name. Please check the code and the name on the booking.".to_string();
            return Ok(CancelBookingOutcome::NotFound { output, voice_text });
        }

        let booking = booking_opt.unwrap();
        rate_limiter.record_success(&normalized_code);

        // 4. Load restaurant config for timezone and cancellation cutoff
        let restaurant: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents
            FROM restaurant
            WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;

        // 5. State machine checks
        if booking.status == BookingStatus::Cancelled {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let summary = build_booking_summary(&booking, &restaurant, &mut conn).await?;
            let output = CancelBookingOutput {
                status: "already_cancelled".to_string(),
                reason: None,
                reason_detail: Some("This reservation was already cancelled.".to_string()),
                booking: Some(summary),
                already_cancelled: true,
                policy_note: None,
            };
            let voice_text = format!(
                "This reservation under code {} was already cancelled. No further action is needed.",
                format_spoken_code(&booking.confirmation_code)
            );
            return Ok(CancelBookingOutcome::AlreadyCancelled { output, voice_text });
        }

        if booking.status == BookingStatus::Completed || booking.status == BookingStatus::NoShow {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = CancelBookingOutput {
                status: "not_cancellable".to_string(),
                reason: Some("already_completed".to_string()),
                reason_detail: Some(
                    "Completed or past reservations cannot be cancelled.".to_string(),
                ),
                booking: None,
                already_cancelled: false,
                policy_note: None,
            };
            let voice_text =
                "This reservation cannot be cancelled because it has already been completed."
                    .to_string();
            return Ok(CancelBookingOutcome::NotCancellable { output, voice_text });
        }

        // Check if booking is in the past
        let start_dt = DateTime::parse_from_rfc3339(&booking.start_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(now);
        if start_dt <= now {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = CancelBookingOutput {
                status: "not_cancellable".to_string(),
                reason: Some("past_booking".to_string()),
                reason_detail: Some("Past reservations cannot be cancelled.".to_string()),
                booking: None,
                already_cancelled: false,
                policy_note: None,
            };
            let voice_text = "This reservation cannot be cancelled because it is in the past."
                .to_string();
            return Ok(CancelBookingOutcome::NotCancellable { output, voice_text });
        }

        // 6. Update booking status to cancelled
        sqlx::query(
            r#"
            UPDATE bookings
            SET status = 'cancelled',
                cancelled_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now_utc_iso)
        .bind(&now_utc_iso)
        .bind(&booking.id)
        .execute(&mut *conn)
        .await?;

        // 7. Write audit event
        let audit_payload = serde_json::json!({
            "confirmation_code": booking.confirmation_code,
            "guest_name": booking.guest_name,
            "reason": req.reason,
            "cancelled_at": now_utc_iso,
        });

        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('booking', ?, 'cancelled', 'mcp', ?, ?)
            "#,
        )
        .bind(&booking.id)
        .bind(audit_payload.to_string())
        .bind(&now_utc_iso)
        .execute(&mut *conn)
        .await?;

        // 8. Calculate policy note
        let hours_until_start = (start_dt - now).num_hours();
        let is_late = hours_until_start < restaurant.cancellation_cutoff_hours;
        let policy_note = if is_late {
            Some(format!(
                "Late cancellation: cancelled less than {} hours prior to reservation.",
                restaurant.cancellation_cutoff_hours
            ))
        } else {
            Some("Cancelled within the free cancellation window.".to_string())
        };

        // 9. Commit transaction
        sqlx::query("COMMIT").execute(&mut *conn).await?;

        let mut updated_booking = booking;
        updated_booking.status = BookingStatus::Cancelled;
        updated_booking.cancelled_at = Some(now_utc_iso.clone());
        let summary = build_booking_summary(&updated_booking, &restaurant, &mut conn).await?;

        let output = CancelBookingOutput {
            status: "cancelled".to_string(),
            reason: None,
            reason_detail: None,
            booking: Some(summary),
            already_cancelled: false,
            policy_note,
        };

        let voice_text = generate_cancel_booking_voice_text(&output);
        Ok(CancelBookingOutcome::Cancelled { output, voice_text })
    }
    .await;

    match res {
        Ok(outcome) => Ok(outcome),
        Err(err) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(err)
        }
    }
}

/// Reads a booking summary, verifying guest_name match to prevent enumeration.
pub async fn get_booking_tx(
    pool: &SqlitePool,
    confirmation_code: &str,
    guest_name: &str,
) -> Result<GetBookingOutcome, sqlx::Error> {
    let normalized_code = normalize_confirmation_code(confirmation_code);
    let mut conn = pool.acquire().await?;

    let booking_opt: Option<Booking> = sqlx::query_as(
        r#"
        SELECT id, confirmation_code, customer_id, guest_name, party_size,
               start_at, end_at, status, table_id, special_requests,
               deposit_status, deposit_amount_cents, created_at, updated_at, cancelled_at,
               guest_phone, guest_email
        FROM bookings
        WHERE confirmation_code = ?
        "#,
    )
    .bind(&normalized_code)
    .fetch_optional(&mut *conn)
    .await?;

    let verified = if let Some(ref b) = booking_opt {
        b.guest_name.trim().eq_ignore_ascii_case(guest_name.trim())
    } else {
        false
    };

    if !verified {
        let output = GetBookingOutput {
            status: "not_found".to_string(),
            booking: None,
            message: "No matching booking found for the provided confirmation code and guest name."
                .to_string(),
        };
        let voice_text = "I couldn't find a booking with that confirmation code and name. Please check the code and the name on the booking.".to_string();
        return Ok(GetBookingOutcome::NotFound { output, voice_text });
    }

    let booking = booking_opt.unwrap();
    let restaurant: RestaurantConfig = sqlx::query_as(
        r#"
        SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
               max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
               deposit_amount_per_person_cents, created_at, updated_at,
               cancellation_cutoff_hours, late_cancellation_fee_cents
        FROM restaurant
        WHERE id = 1
        "#,
    )
    .fetch_one(&mut *conn)
    .await?;

    let summary = build_booking_summary(&booking, &restaurant, &mut conn).await?;
    let output = GetBookingOutput {
        status: "found".to_string(),
        booking: Some(summary.clone()),
        message: "Booking retrieved successfully.".to_string(),
    };
    let voice_text = generate_get_booking_voice_text(&output);
    Ok(GetBookingOutcome::Found { output, voice_text })
}

// --- Helper: Build BookingSummary with local times and table details ---

async fn build_booking_summary(
    booking: &Booking,
    restaurant: &RestaurantConfig,
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
) -> Result<BookingSummary, sqlx::Error> {
    let rest_tz: Tz = restaurant.timezone.parse().unwrap_or(chrono_tz::UTC);

    let start_utc = DateTime::parse_from_rfc3339(&booking.start_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let end_utc = DateTime::parse_from_rfc3339(&booking.end_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let start_local = start_utc.with_timezone(&rest_tz).to_rfc3339();
    let end_local = end_utc.with_timezone(&rest_tz).to_rfc3339();

    let (area_str, table_label) = if let Some(t_id) = booking.table_id {
        let table_info: Option<(String, Area)> =
            sqlx::query_as("SELECT label, area FROM dining_tables WHERE id = ?")
                .bind(t_id)
                .fetch_optional(&mut **conn)
                .await?;
        if let Some((label, area)) = table_info {
            (area.as_str().to_string(), label)
        } else {
            ("indoor".to_string(), "Unassigned".to_string())
        }
    } else {
        ("indoor".to_string(), "Unassigned".to_string())
    };

    let status_str = match booking.status {
        BookingStatus::Pending => "pending",
        BookingStatus::Confirmed => "confirmed",
        BookingStatus::Cancelled => "cancelled",
        BookingStatus::Completed => "completed",
        BookingStatus::NoShow => "no_show",
    };

    Ok(BookingSummary {
        confirmation_code: booking.confirmation_code.clone(),
        guest_name: booking.guest_name.clone(),
        party_size: booking.party_size,
        start_local,
        end_local,
        area: area_str,
        table_label,
        status: status_str.to_string(),
        special_requests: booking.special_requests.clone(),
        created_at: booking.created_at.clone(),
    })
}

// --- Voice Text Generators ---

pub fn generate_create_booking_voice_text(out: &CreateBookingOutput) -> String {
    if let Some(ref b) = out.booking {
        let spoken_code = format_spoken_code(&b.confirmation_code);
        if let Ok(start_dt) = DateTime::parse_from_rfc3339(&b.start_local) {
            let spoken_date = format_spoken_date(start_dt.date_naive());
            let spoken_time = format_spoken_time(start_dt.time());
            let guest_phrase = if b.party_size == 1 {
                "1 guest".to_string()
            } else {
                format!("{} guests", b.party_size)
            };

            format!(
                "I have confirmed your table for {} on {} at {} under the name {}. Your confirmation code is {}. If you need to cancel or change your reservation, just mention this code.",
                guest_phrase, spoken_date, spoken_time, b.guest_name, spoken_code
            )
        } else {
            format!(
                "I have confirmed your reservation for {} under the name {}. Your confirmation code is {}.",
                b.party_size, b.guest_name, spoken_code
            )
        }
    } else {
        "I'm sorry, I was unable to complete your reservation.".to_string()
    }
}

pub fn generate_cancel_booking_voice_text(out: &CancelBookingOutput) -> String {
    if out.already_cancelled {
        if let Some(ref b) = out.booking {
            format!(
                "This reservation under code {} was already cancelled. No further action is needed.",
                format_spoken_code(&b.confirmation_code)
            )
        } else {
            "This reservation was already cancelled.".to_string()
        }
    } else if let Some(ref b) = out.booking {
        let spoken_code = format_spoken_code(&b.confirmation_code);
        if let Ok(start_dt) = DateTime::parse_from_rfc3339(&b.start_local) {
            let spoken_date = format_spoken_date(start_dt.date_naive());
            let policy_clause = match out.policy_note.as_deref() {
                Some(note) if note.contains("Late cancellation") => {
                    " Please note that this was cancelled past our standard cutoff."
                }
                _ => "",
            };
            format!(
                "Your reservation for {} under code {} has been cancelled.{} We hope to welcome you another time.",
                spoken_date, spoken_code, policy_clause
            )
        } else {
            format!(
                "Your reservation under code {} has been cancelled.",
                spoken_code
            )
        }
    } else {
        "Your reservation has been cancelled.".to_string()
    }
}

pub fn generate_get_booking_voice_text(out: &GetBookingOutput) -> String {
    if let Some(ref b) = out.booking {
        let spoken_code = format_spoken_code(&b.confirmation_code);
        if let Ok(start_dt) = DateTime::parse_from_rfc3339(&b.start_local) {
            let spoken_date = format_spoken_date(start_dt.date_naive());
            let spoken_time = format_spoken_time(start_dt.time());
            format!(
                "I found your reservation for {} guests on {} at {} under the name {}. Confirmation code is {}.",
                b.party_size, spoken_date, spoken_time, b.guest_name, spoken_code
            )
        } else {
            format!(
                "I found your reservation under the name {} with confirmation code {}.",
                b.guest_name, spoken_code
            )
        }
    } else {
        "I couldn't find a booking with that confirmation code and name. Please check the code and the name on the booking.".to_string()
    }
}

// --- Unit Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_allowed_and_illegal_transitions() {
        // Allowed transitions
        assert!(can_transition(
            BookingStatus::Pending,
            BookingStatus::Confirmed
        ));
        assert!(can_transition(
            BookingStatus::Pending,
            BookingStatus::Cancelled
        ));
        assert!(can_transition(
            BookingStatus::Confirmed,
            BookingStatus::Completed
        ));
        assert!(can_transition(
            BookingStatus::Confirmed,
            BookingStatus::NoShow
        ));
        assert!(can_transition(
            BookingStatus::Confirmed,
            BookingStatus::Cancelled
        ));
        assert!(can_transition(
            BookingStatus::Cancelled,
            BookingStatus::Cancelled
        ));

        // Illegal transitions
        assert!(!can_transition(
            BookingStatus::Cancelled,
            BookingStatus::Confirmed
        ));
        assert!(!can_transition(
            BookingStatus::Cancelled,
            BookingStatus::Pending
        ));
        assert!(!can_transition(
            BookingStatus::Completed,
            BookingStatus::Cancelled
        ));
        assert!(!can_transition(
            BookingStatus::Completed,
            BookingStatus::Confirmed
        ));
        assert!(!can_transition(
            BookingStatus::NoShow,
            BookingStatus::Cancelled
        ));
        assert!(!can_transition(
            BookingStatus::Confirmed,
            BookingStatus::Pending
        ));

        assert!(apply_transition(BookingStatus::Pending, BookingStatus::Confirmed).is_ok());
        assert!(apply_transition(BookingStatus::Confirmed, BookingStatus::Cancelled).is_ok());
        assert!(apply_transition(BookingStatus::Completed, BookingStatus::Cancelled).is_err());
    }

    #[test]
    fn test_confirmation_code_alphabet_and_properties() {
        // Safe alphabet must not contain ambiguous characters: '0', 'O', '1', 'I', 'L'
        for &ch in CONFIRMATION_CODE_ALPHABET {
            let c = ch as char;
            assert_ne!(c, '0');
            assert_ne!(c, 'O');
            assert_ne!(c, '1');
            assert_ne!(c, 'I');
            assert_ne!(c, 'L');
        }

        // Generate 100 codes, ensure format TT-XXXXX and all chars in alphabet
        let mut codes = std::collections::HashSet::new();
        for _ in 0..100 {
            let code = generate_confirmation_code();
            assert!(code.starts_with("TT-"));
            assert_eq!(code.len(), 8); // "TT-" (3) + 5 = 8
            for ch in code[3..].chars() {
                assert!(
                    CONFIRMATION_CODE_ALPHABET.contains(&(ch as u8)),
                    "Char '{}' not in alphabet",
                    ch
                );
            }
            codes.insert(code);
        }
        // Collisions among 100 codes in a 31^5 keyspace should be 0
        assert_eq!(codes.len(), 100);
    }

    #[test]
    fn test_confirmation_code_normalization_and_lookalikes() {
        assert_eq!(normalize_confirmation_code("tt-8k4mz"), "TT-8K4MZ");
        assert_eq!(normalize_confirmation_code("TT-8K4MZ"), "TT-8K4MZ");
        assert_eq!(normalize_confirmation_code("8k4mz"), "TT-8K4MZ");
        assert_eq!(normalize_confirmation_code("tt 8k4mz"), "TT-8K4MZ");
        assert_eq!(normalize_confirmation_code("  TT - 8k_4mz "), "TT-8K4MZ");
        assert_eq!(normalize_confirmation_code("8K-4M-Z"), "TT-8K4MZ");
    }

    #[test]
    fn test_spoken_code_formatting() {
        assert_eq!(format_spoken_code("TT-8K4MZ"), "T T 8 K 4 M Z");
        assert_eq!(format_spoken_code("TT-8101"), "T T 8 1 0 1");
    }

    #[test]
    fn test_request_validation_rules() {
        // Valid request
        assert!(validate_create_booking_request(
            "2026-10-17",
            "19:30",
            4,
            "Alice Smith",
            Some("+1-555-0199"),
            Some("alice@example.com"),
            Some("Window seat please"),
            Some("idemp_key_12345"),
            12
        )
        .is_ok());

        // Invalid date
        assert!(matches!(
            validate_create_booking_request(
                "tomorrow", "19:30", 4, "Alice", None, None, None, None, 12
            ),
            Err(BookingValidationError::InvalidDate(_))
        ));

        // Invalid time
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17", "7:30pm", 4, "Alice", None, None, None, None, 12
            ),
            Err(BookingValidationError::InvalidTime(_))
        ));

        // Party size zero
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17", "19:30", 0, "Alice", None, None, None, None, 12
            ),
            Err(BookingValidationError::InvalidPartySize(_, _))
        ));

        // Party size exceeding max
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17", "19:30", 25, "Alice", None, None, None, None, 12
            ),
            Err(BookingValidationError::InvalidPartySize(_, _))
        ));

        // Empty guest name
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17", "19:30", 2, "   ", None, None, None, None, 12
            ),
            Err(BookingValidationError::InvalidGuestName)
        ));

        // Special requests too long (>300)
        let long_requests = "A".repeat(301);
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17",
                "19:30",
                2,
                "Alice",
                None,
                None,
                Some(&long_requests),
                None,
                12
            ),
            Err(BookingValidationError::SpecialRequestsTooLong)
        ));

        // Idempotency key too short (<8)
        assert!(matches!(
            validate_create_booking_request(
                "2026-10-17", "19:30", 2, "Alice", None, None, None, Some("short"), 12
            ),
            Err(BookingValidationError::InvalidIdempotencyKey(_))
        ));
    }

    #[test]
    fn test_idempotency_key_hashing_and_validation() {
        assert!(validate_idempotency_key("req_key_12345678"));
        assert!(validate_idempotency_key("uuid-9f8e7d6c-5b4a"));
        assert!(!validate_idempotency_key("short")); // <8
        assert!(!validate_idempotency_key(&"A".repeat(65))); // >64
        assert!(!validate_idempotency_key("invalid key spaces!"));

        // Same parameters yield exact same hash
        let hash1 = compute_request_hash(
            "2026-10-17",
            "19:30",
            4,
            "Alice Smith",
            Some("1234567"),
            None,
            Some("indoor"),
            Some(90),
            Some("None"),
        );
        let hash2 = compute_request_hash(
            "2026-10-17",
            "19:30",
            4,
            "Alice Smith",
            Some("1234567"),
            None,
            Some("indoor"),
            Some(90),
            Some("None"),
        );
        assert_eq!(hash1, hash2);

        // Different party size yields different hash
        let hash3 = compute_request_hash(
            "2026-10-17",
            "19:30",
            5,
            "Alice Smith",
            Some("1234567"),
            None,
            Some("indoor"),
            Some(90),
            Some("None"),
        );
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cancellation_rate_limiter() {
        let limiter = CancellationRateLimiter::new();
        let code = "TT-8K4MZ";

        for _ in 0..MAX_CANCEL_ATTEMPTS {
            assert!(!limiter.is_rate_limited(code));
            limiter.record_failure(code);
        }
        assert!(limiter.is_rate_limited(code));

        limiter.record_success(code);
        assert!(!limiter.is_rate_limited(code));
    }

    #[test]
    fn test_spoken_voice_text_generation() {
        let out = CreateBookingOutput {
            status: "confirmed".to_string(),
            reason: None,
            reason_detail: None,
            booking: Some(BookingSummary {
                confirmation_code: "TT-8K4MZ".to_string(),
                guest_name: "Alice".to_string(),
                party_size: 2,
                start_local: "2026-10-16T19:30:00+05:30".to_string(),
                end_local: "2026-10-16T20:45:00+05:30".to_string(),
                area: "indoor".to_string(),
                table_label: "T01".to_string(),
                status: "confirmed".to_string(),
                special_requests: None,
                created_at: "2026-10-13T12:00:00Z".to_string(),
            }),
            idempotent_replay: false,
            alternatives: vec![],
            cancellation_policy: Some(
                "Free cancellation up to 6 hours prior to reservation.".to_string(),
            ),
            note: "availability is a snapshot, not a reservation".to_string(),
        };

        let voice_text = generate_create_booking_voice_text(&out);
        assert!(voice_text.contains("Alice"));
        assert!(voice_text.contains("Friday the sixteenth"));
        assert!(voice_text.contains("seven thirty in the evening"));
        assert!(voice_text.contains("T T 8 K 4 M Z"));
        assert!(!voice_text.contains("19:30"));
        assert!(!voice_text.contains("#"));
        assert!(!voice_text.contains("*"));
    }
}
