//! Simulated payment engine for booking deposits (Phase 12).
//!
//! This is a MOCK ledger. No real money moves, no real provider is called, and
//! no card data is ever accepted or stored. All money is integer minor units
//! (paise/cents) with checked arithmetic. Time comes only from the injected
//! [`Clock`]. All ledger-affecting operations run in `BEGIN IMMEDIATE`
//! transactions and re-check state inside the transaction.

use crate::availability::Clock;
use crate::db::models::{
    Booking, BookingStatus, DepositStatus, LedgerEntryType, PaymentStatus, RestaurantConfig,
    SessionStatus,
};
use chrono::{DateTime, Datelike, NaiveTime, Utc};
use chrono_tz::Tz;
use rand::RngCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, SqlitePool};
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// Constants (defaults; the effective values come from restaurant config)
// ---------------------------------------------------------------------------

pub const PROVIDER_NAME: &str = "mockpay";
pub const TOKEN_PREFIX: &str = "tt_pay_";
pub const DEFAULT_PAYMENT_DEADLINE_MINUTES: i64 = 15;
pub const MAX_PAYMENT_DEADLINE_MINUTES: i64 = 30;
pub const DEFAULT_MAX_DEPOSIT_MINOR: i64 = 400_000; // ₹4000
pub const DEFAULT_REFUND_FULL_BPS: i64 = 10_000; // 100%
pub const DEFAULT_REFUND_PARTIAL_BPS: i64 = 5_000; // 50%
pub const DEFAULT_REFUND_FULL_HOURS: i64 = 24;
pub const DEFAULT_REFUND_PARTIAL_HOURS: i64 = 2;
pub const DEFAULT_DECLINE_RETRY_LIMIT: i64 = 3;

/// Test-only fault injection point: when set, `complete_checkout` aborts after
/// writing ledger entries but before updating the booking, so tests can prove
/// the whole transaction rolls back. Inert in normal operation.
pub static FAIL_AFTER_LEDGER_INSERT: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("payment session not found")]
    SessionNotFound,
    #[error("payment session is not open (status {0})")]
    SessionNotOpen(String),
    #[error("payment deadline has passed")]
    DeadlinePassed,
    #[error("booking is not pending (status {0})")]
    BookingNotPending(String),
    #[error("ledger conflict: {0}")]
    LedgerConflict(String),
    #[error("money arithmetic overflow")]
    Overflow,
    #[error("illegal payment transition from {from:?} to {to:?}")]
    Transition { from: PaymentState, to: PaymentState },
    #[error("injected fault (test)")]
    FaultInjected,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

// ---------------------------------------------------------------------------
// Money
// ---------------------------------------------------------------------------

/// Structured money value: integer minor units plus a display string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Money {
    pub amount_minor: i64,
    pub currency: String,
    pub display: String,
}

/// Builds a [`Money`] using the restaurant's display convention (e.g. "₹2000").
pub fn money(amount_minor: i64, currency: &str) -> Money {
    let pd = crate::ordering::format_price_display(amount_minor, currency);
    Money {
        amount_minor: pd.amount_minor,
        currency: pd.currency,
        display: pd.display,
    }
}

/// Spoken English rendering of an INR amount in minor units (paise).
/// Reuses the Phase 9 converter so voice money wording is consistent.
pub fn spoken_amount_inr(amount_minor: i64) -> String {
    crate::ordering::paise_to_spoken_rupees(amount_minor)
}

/// Converts minor units to spoken words in the currency's natural unit.
pub fn spoken_money(amount_minor: i64, currency: &str) -> String {
    if currency.eq_ignore_ascii_case("INR") {
        spoken_amount_inr(amount_minor)
    } else {
        // Non-INR currencies fall back to a plain amount + code.
        format!("{} {}", amount_minor / 100, currency)
    }
}

const NUMBER_WORDS: [&str; 13] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve",
];

/// Spoken word for 0..=99 (hyphenated tens, e.g. forty-five).
fn number_word(n: i64) -> String {
    const ONES: [&str; 20] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 8] = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    if !(0..=99).contains(&n) {
        return n.to_string();
    }
    if n < 20 {
        return ONES[n as usize].to_string();
    }
    let t = (n / 10) as usize;
    let r = n % 10;
    if r == 0 {
        TENS[t - 2].to_string()
    } else {
        format!("{}-{}", TENS[t - 2], ONES[r as usize])
    }
}

/// Spoken form of a small count (used for party sizes and small minute counts).
pub fn spoken_count(n: i64) -> String {
    if (0..=12).contains(&n) {
        NUMBER_WORDS[n as usize].to_string()
    } else if (0..=99).contains(&n) {
        number_word(n)
    } else {
        n.to_string()
    }
}

/// Spoken minutes remaining, e.g. "fifteen minutes", "one minute".
pub fn spoken_minutes(minutes: i64) -> String {
    if minutes <= 0 {
        return "less than a minute".to_string();
    }
    if minutes == 1 {
        return "one minute".to_string();
    }
    format!("{} minutes", number_word(minutes))
}

// ---------------------------------------------------------------------------
// Deposit policy
// ---------------------------------------------------------------------------

/// Resolved deposit policy derived from the restaurant configuration row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositPolicy {
    pub large_party_threshold: i64,
    pub per_person_minor: i64,
    pub max_deposit_minor: i64,
    pub payment_deadline_minutes: i64,
    pub refund_full_bps: i64,
    pub refund_partial_bps: i64,
    pub refund_full_hours: i64,
    pub refund_partial_hours: i64,
    pub peak_days: Vec<u32>,
    pub peak_start: Option<NaiveTime>,
    pub peak_end: Option<NaiveTime>,
    pub demo_mode: bool,
    pub decline_retry_limit: i64,
}

impl DepositPolicy {
    pub fn from_config(config: &RestaurantConfig) -> Self {
        let peak_days: Vec<u32> = config
            .deposit_peak_days
            .split(',')
            .filter_map(|s| s.trim().parse::<u32>().ok())
            .filter(|d| *d <= 6)
            .collect();

        let peak_start = config
            .deposit_peak_start_time
            .as_deref()
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok());
        let peak_end = config
            .deposit_peak_end_time
            .as_deref()
            .and_then(|s| NaiveTime::parse_from_str(s, "%H:%M").ok());

        Self {
            large_party_threshold: config.deposit_required_party_size,
            per_person_minor: config.deposit_amount_per_person_cents,
            max_deposit_minor: config.deposit_max_amount_cents,
            payment_deadline_minutes: config
                .deposit_payment_deadline_minutes
                .clamp(1, MAX_PAYMENT_DEADLINE_MINUTES),
            refund_full_bps: config.deposit_refund_full_basis_points,
            refund_partial_bps: config.deposit_refund_partial_basis_points,
            refund_full_hours: config.deposit_refund_full_hours,
            refund_partial_hours: config.deposit_refund_partial_hours,
            peak_days,
            peak_start,
            peak_end,
            demo_mode: config.deposit_demo_mode,
            decline_retry_limit: config.deposit_decline_retry_limit,
        }
    }

    /// Defaults used when no configuration row exists (documented in the design doc).
    pub fn defaults() -> Self {
        Self {
            large_party_threshold: 6,
            per_person_minor: 25_000,
            max_deposit_minor: DEFAULT_MAX_DEPOSIT_MINOR,
            payment_deadline_minutes: DEFAULT_PAYMENT_DEADLINE_MINUTES,
            refund_full_bps: DEFAULT_REFUND_FULL_BPS,
            refund_partial_bps: DEFAULT_REFUND_PARTIAL_BPS,
            refund_full_hours: DEFAULT_REFUND_FULL_HOURS,
            refund_partial_hours: DEFAULT_REFUND_PARTIAL_HOURS,
            peak_days: vec![],
            peak_start: None,
            peak_end: None,
            demo_mode: true,
            decline_retry_limit: DEFAULT_DECLINE_RETRY_LIMIT,
        }
    }
}

/// A deposit is required for a large party, or when the slot falls inside a
/// configured peak window (weekday + time-of-day range).
pub fn deposit_required(
    policy: &DepositPolicy,
    party_size: i64,
    slot_start_local: &DateTime<Tz>,
) -> bool {
    if party_size >= policy.large_party_threshold {
        return true;
    }
    if let (Some(start), Some(end)) = (policy.peak_start, policy.peak_end) {
        let weekday = slot_start_local.weekday().num_days_from_sunday();
        let time = slot_start_local.time();
        if policy.peak_days.contains(&weekday) && time >= start && time < end {
            return true;
        }
    }
    false
}

/// amount = per_person_minor * party_size, capped at the policy maximum.
/// Uses checked arithmetic; never panics and never returns a float.
pub fn compute_deposit_amount(policy: &DepositPolicy, party_size: i64) -> Result<i64, PaymentError> {
    let total = policy
        .per_person_minor
        .checked_mul(party_size)
        .ok_or(PaymentError::Overflow)?;
    Ok(total.min(policy.max_deposit_minor))
}

// ---------------------------------------------------------------------------
// Refund schedule (integer, basis points, round half up)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefundQuote {
    pub refund_minor: i64,
    pub retained_minor: i64,
    pub bps: i64,
}

/// Round half up of `amount * bps / 10000` using 128-bit intermediate arithmetic.
pub fn apply_basis_points_round_half_up(amount_minor: i64, bps: i64) -> i64 {
    if amount_minor <= 0 || bps <= 0 {
        return 0;
    }
    if bps >= 10_000 {
        return amount_minor;
    }
    let numerator = amount_minor as i128 * bps as i128 + 5_000;
    (numerator / 10_000) as i64
}

/// Refund for cancelling with `minutes_until_start` remaining before the booking
/// start. Boundaries are inclusive at the top of each window:
///   >= refund_full_hours   -> full_bps
///   >= refund_partial_hours-> partial_bps
///   otherwise              -> 0
pub fn compute_refund(
    deposit_minor: i64,
    minutes_until_start: i64,
    policy: &DepositPolicy,
) -> RefundQuote {
    let bps = if minutes_until_start >= policy.refund_full_hours * 60 {
        policy.refund_full_bps
    } else if minutes_until_start >= policy.refund_partial_hours * 60 {
        policy.refund_partial_bps
    } else {
        0
    };
    let refund_minor = apply_basis_points_round_half_up(deposit_minor, bps);
    RefundQuote {
        refund_minor,
        retained_minor: deposit_minor - refund_minor,
        bps,
    }
}

// ---------------------------------------------------------------------------
// Payment state machine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentState {
    RequiresAction,
    Authorized,
    Captured,
    PartiallyRefunded,
    Refunded,
    Failed,
    Expired,
    Voided,
}

impl PaymentState {
    pub fn from_status(status: PaymentStatus) -> Self {
        match status {
            PaymentStatus::RequiresAction => PaymentState::RequiresAction,
            PaymentStatus::Authorized => PaymentState::Authorized,
            PaymentStatus::Captured => PaymentState::Captured,
            PaymentStatus::PartiallyRefunded => PaymentState::PartiallyRefunded,
            PaymentStatus::Refunded => PaymentState::Refunded,
            PaymentStatus::Failed => PaymentState::Failed,
            PaymentStatus::Expired => PaymentState::Expired,
            PaymentStatus::Voided => PaymentState::Voided,
        }
    }

    pub fn to_status(self) -> PaymentStatus {
        match self {
            PaymentState::RequiresAction => PaymentStatus::RequiresAction,
            PaymentState::Authorized => PaymentStatus::Authorized,
            PaymentState::Captured => PaymentStatus::Captured,
            PaymentState::PartiallyRefunded => PaymentStatus::PartiallyRefunded,
            PaymentState::Refunded => PaymentStatus::Refunded,
            PaymentState::Failed => PaymentStatus::Failed,
            PaymentState::Expired => PaymentStatus::Expired,
            PaymentState::Voided => PaymentStatus::Voided,
        }
    }
}

/// Explicit allowed transitions:
///   requires_action -> authorized | failed | expired
///   authorized      -> captured | voided
///   captured        -> partially_refunded | refunded
///   partially_refunded -> refunded
pub fn can_transition_payment(from: PaymentState, to: PaymentState) -> bool {
    use PaymentState::*;
    matches!(
        (from, to),
        (RequiresAction, Authorized)
            | (RequiresAction, Failed)
            | (RequiresAction, Expired)
            | (Authorized, Captured)
            | (Authorized, Voided)
            | (Captured, PartiallyRefunded)
            | (Captured, Refunded)
            | (PartiallyRefunded, Refunded)
    )
}

pub fn apply_payment_transition(
    from: PaymentState,
    to: PaymentState,
) -> Result<PaymentState, PaymentError> {
    if can_transition_payment(from, to) {
        Ok(to)
    } else {
        Err(PaymentError::Transition { from, to })
    }
}

// ---------------------------------------------------------------------------
// Payment session tokens
// ---------------------------------------------------------------------------

/// Generate an opaque payment token: prefix `tt_pay_` + 32 hex chars (128 bits
/// from the OS CSPRNG). Returns `(plain_token, sha256_hash)`.
pub fn generate_payment_token() -> (String, String) {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let hex_part: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let plain = format!("{}{}", TOKEN_PREFIX, hex_part);
    let hash = hash_payment_token(&plain);
    (plain, hash)
}

/// SHA-256 hash (lowercase hex) of a payment token. Only the hash is ever stored.
pub fn hash_payment_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

/// True when a string looks like a payment token (used for log redaction).
pub fn looks_like_payment_token(s: &str) -> bool {
    s.starts_with(TOKEN_PREFIX)
}

// ---------------------------------------------------------------------------
// Card-data safeguard (defense in depth; no card fields are ever accepted)
// ---------------------------------------------------------------------------

/// Standard Luhn checksum over a digit slice.
pub fn luhn_valid(digits: &[u8]) -> bool {
    if digits.is_empty() {
        return false;
    }
    let mut sum: u32 = 0;
    let mut double = false;
    for &d in digits.iter().rev() {
        if d > 9 {
            return false;
        }
        let mut v = d as u32;
        if double {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
        double = !double;
    }
    sum % 10 == 0
}

/// Detects a card-like digit sequence (13-19 digits, Luhn-valid) in a string,
/// tolerating spaces and dashes as digit separators.
pub fn contains_card_number(value: &str) -> bool {
    let mut run: Vec<u8> = Vec::new();
    for c in value.chars() {
        if c.is_ascii_digit() {
            run.push(c as u8 - b'0');
        } else if c == ' ' || c == '-' || c == '\t' {
            continue;
        } else {
            run.clear();
            continue;
        }
        if run.len() > 19 {
            run.clear();
        }
        if run.len() >= 13 && luhn_valid(&run) {
            return true;
        }
    }
    false
}

/// Returns true if a parsed JSON body contains a card-like field name
/// (card/cvv/pan) or any Luhn-valid 13-19 digit sequence.
pub fn body_contains_card_data(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let k = key.to_ascii_lowercase();
                if k.contains("card") || k.contains("cvv") || k.contains("pan") {
                    return true;
                }
                if body_contains_card_data(val) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(items) => items.iter().any(body_contains_card_data),
        serde_json::Value::String(s) => contains_card_number(s),
        serde_json::Value::Number(n) => contains_card_number(&n.to_string()),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Structured output types
// ---------------------------------------------------------------------------

/// Payment object embedded in create_booking / accept_offer / refresh responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PaymentObject {
    pub required: bool,
    pub simulated: bool,
    pub status: String,
    pub amount: Money,
    pub deadline_local: String,
    pub minutes_remaining: i64,
    pub checkout_url: String,
}

/// Refund outcome embedded in cancel_booking responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RefundSummary {
    pub amount: Money,
    pub percent_bps: i64,
    pub percent_display: String,
    pub explanation: String,
}

/// Deposit details surfaced by get_booking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DepositInfo {
    pub status: String,
    pub amount: Money,
    pub deadline_local: Option<String>,
    pub minutes_remaining: Option<i64>,
}

/// Outcome selector accepted by the mock checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckoutOutcome {
    Success,
    Decline,
    NeedsVerification,
}

impl CheckoutOutcome {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "success" => Some(CheckoutOutcome::Success),
            "decline" => Some(CheckoutOutcome::Decline),
            "needs_verification" => Some(CheckoutOutcome::NeedsVerification),
            _ => None,
        }
    }
}

/// Result of a checkout completion attempt (also the HTTP response payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CheckoutResult {
    pub status: String,
    pub simulated: bool,
    pub message: String,
    #[serde(skip)]
    pub booking_confirmed: bool,
}

impl CheckoutResult {
    fn new(status: &str, message: String) -> Self {
        Self {
            status: status.to_string(),
            simulated: true,
            message,
            booking_confirmed: false,
        }
    }
}

/// Configuration for building checkout URLs.
#[derive(Debug, Clone)]
pub struct CheckoutConfig {
    pub checkout_base_url: String,
}

impl Default for CheckoutConfig {
    fn default() -> Self {
        Self {
            checkout_base_url: std::env::var("CHECKOUT_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        }
    }
}

impl CheckoutConfig {
    pub fn checkout_url(&self, token: &str) -> String {
        format!(
            "{}/checkout/{}",
            self.checkout_base_url.trim_end_matches('/'),
            token
        )
    }
}

/// Whole minutes remaining until `deadline`, rounded up, never negative.
pub fn minutes_remaining(deadline: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    let secs = (deadline - now).num_seconds();
    if secs <= 0 {
        0
    } else {
        (secs + 59) / 60
    }
}

// ---------------------------------------------------------------------------
// Voice text
// ---------------------------------------------------------------------------

pub fn voice_deposit_required(
    policy: &DepositPolicy,
    amount_minor: i64,
    currency: &str,
    party_size: i64,
    deadline_minutes: i64,
    confirmation_code: &str,
) -> String {
    let amount = spoken_money(amount_minor, currency);
    let party = spoken_count(party_size);
    let mut text = format!(
        "A table for {} needs a deposit of {}. I've put the payment on your screen, and I'll hold the table for {}.",
        party,
        amount,
        spoken_minutes(deadline_minutes)
    );
    if policy.demo_mode {
        text.push_str(" This is a simulated payment.");
    }
    text.push_str(&format!(
        " Your confirmation code is {}.",
        crate::booking::format_spoken_code(confirmation_code)
    ));
    text
}

pub fn voice_payment_confirmed(
    amount_minor: i64,
    currency: &str,
    confirmation_code: &str,
) -> String {
    let amount = spoken_money(amount_minor, currency);
    format!(
        "Your deposit of {} is confirmed and your table is booked. Your confirmation code is {}.",
        amount,
        crate::booking::format_spoken_code(confirmation_code)
    )
}

pub fn voice_payment_expired() -> String {
    "The payment deadline has passed, so I've released the hold on your table. Would you like me to look for a new time?"
        .to_string()
}

pub fn voice_payment_declined(minutes_left: i64) -> String {
    format!(
        "That payment didn't go through. You can try again on your screen, and I'll keep holding the table for {}.",
        spoken_minutes(minutes_left)
    )
}

pub fn voice_deposit_pending(amount_minor: i64, currency: &str, minutes_left: i64) -> String {
    let amount = spoken_money(amount_minor, currency);
    format!(
        "Your deposit of {} is still pending. You have {} to complete the payment.",
        amount,
        spoken_minutes(minutes_left)
    )
}

pub fn voice_deposit_paid(amount_minor: i64, currency: &str) -> String {
    let amount = spoken_money(amount_minor, currency);
    format!(
        "Your deposit of {} is already paid, and your table is confirmed.",
        amount
    )
}

pub fn voice_deposit_not_required() -> String {
    "No deposit is required for this booking.".to_string()
}

pub fn voice_refund_summary(summary: &RefundSummary) -> String {
    if summary.amount.amount_minor <= 0 {
        "Because it's less than two hours before your reservation, no refund applies.".to_string()
    } else if summary.percent_bps >= 10_000 {
        format!(
            "You'll get a full refund of {}.",
            spoken_money(summary.amount.amount_minor, &summary.amount.currency)
        )
    } else {
        format!(
            "Because your reservation is between two and twenty four hours away, you'll receive a partial refund of {}.",
            spoken_money(summary.amount.amount_minor, &summary.amount.currency)
        )
    }
}

// ---------------------------------------------------------------------------
// Transactional operations
// ---------------------------------------------------------------------------

/// Creates a payment session for a booking inside an open transaction.
/// The plain token is returned to the caller and never stored.
#[allow(clippy::too_many_arguments)]
pub async fn create_payment_session_in_conn(
    conn: &mut SqliteConnection,
    booking_id: &str,
    amount_minor: i64,
    currency: &str,
    deadline_utc: DateTime<Utc>,
    now_utc: DateTime<Utc>,
    tz: &Tz,
    checkout_config: &CheckoutConfig,
) -> Result<PaymentObject, PaymentError> {
    let (plain_token, token_hash) = generate_payment_token();
    let session_id = uuid::Uuid::new_v4().to_string();
    let expires_iso = deadline_utc.to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO payment_sessions (
            id, booking_id, token_hash, amount_minor, currency, status, expires_at, decline_count, created_at
        ) VALUES (?, ?, ?, ?, ?, 'open', ?, 0, ?)
        "#,
    )
    .bind(&session_id)
    .bind(booking_id)
    .bind(&token_hash)
    .bind(amount_minor)
    .bind(currency)
    .bind(&expires_iso)
    .bind(now_utc.to_rfc3339())
    .execute(&mut *conn)
    .await?;

    Ok(PaymentObject {
        required: true,
        simulated: true,
        status: "requires_payment".to_string(),
        amount: money(amount_minor, currency),
        deadline_local: deadline_utc.with_timezone(tz).to_rfc3339(),
        minutes_remaining: minutes_remaining(deadline_utc, now_utc),
        checkout_url: checkout_config.checkout_url(&plain_token),
    })
}

/// Closes any open sessions for a booking (used when cancelling an unpaid booking).
pub async fn close_open_sessions_in_conn(
    conn: &mut SqliteConnection,
    booking_id: &str,
) -> Result<u64, PaymentError> {
    let res = sqlx::query(
        "UPDATE payment_sessions SET status = 'cancelled' WHERE booking_id = ? AND status = 'open'",
    )
    .bind(booking_id)
    .execute(&mut *conn)
    .await?;
    Ok(res.rows_affected())
}

/// Lazily expires a single overdue pending deposit booking inside an open
/// transaction. Returns true if the booking was transitioned to cancelled.
/// Idempotent: a second call on an already-cancelled booking does nothing.
pub async fn lazy_expire_booking_in_conn(
    conn: &mut SqliteConnection,
    booking: &Booking,
    now_utc: DateTime<Utc>,
    tz: &Tz,
) -> Result<bool, PaymentError> {
    if booking.status != BookingStatus::Pending
        || booking.deposit_status != DepositStatus::RequiresPayment
    {
        return Ok(false);
    }
    let deadline = match booking.payment_deadline_at.as_deref() {
        Some(s) => DateTime::parse_from_rfc3339(s)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(now_utc),
        None => return Ok(false),
    };
    if deadline > now_utc {
        return Ok(false);
    }
    expire_pending_booking_in_conn(conn, booking, now_utc, tz).await
}

/// Transitions a pending deposit booking to cancelled / payment_timeout,
/// expires its sessions, writes an audit event and runs the waitlist matcher.
/// Caller must have verified the booking is pending and overdue.
async fn expire_pending_booking_in_conn(
    conn: &mut SqliteConnection,
    booking: &Booking,
    now_utc: DateTime<Utc>,
    tz: &Tz,
) -> Result<bool, PaymentError> {
    let now_iso = now_utc.to_rfc3339();

    let updated = sqlx::query(
        r#"
        UPDATE bookings
        SET status = 'cancelled', deposit_status = 'expired', cancel_reason = 'payment_timeout',
            cancelled_at = ?, updated_at = ?
        WHERE id = ? AND status = 'pending' AND deposit_status = 'requires_payment'
        "#,
    )
    .bind(&now_iso)
    .bind(&now_iso)
    .bind(&booking.id)
    .execute(&mut *conn)
    .await?
    .rows_affected();

    if updated == 0 {
        return Ok(false);
    }

    sqlx::query("UPDATE payment_sessions SET status = 'expired' WHERE booking_id = ? AND status = 'open'")
        .bind(&booking.id)
        .execute(&mut *conn)
        .await?;

    let payload = serde_json::json!({
        "confirmation_code_prefix": booking.confirmation_code.get(..6).unwrap_or(&booking.confirmation_code),
        "reason": "payment_timeout",
    });
    sqlx::query(
        r#"
        INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
        VALUES ('booking', ?, 'payment_timeout', 'system', ?, ?)
        "#,
    )
    .bind(&booking.id)
    .bind(payload.to_string())
    .bind(&now_iso)
    .execute(&mut *conn)
    .await?;

    // Free the slot for the waitlist in the same transaction.
    run_waitlist_matcher_for_slot(
        conn,
        booking.table_id,
        &booking.start_at,
        &booking.end_at,
        tz,
        now_utc,
    )
    .await?;

    Ok(true)
}

/// Runs the Phase 11 waitlist matcher for a freed slot, deriving local date/time
/// from the booking times using the restaurant timezone.
async fn run_waitlist_matcher_for_slot(
    conn: &mut SqliteConnection,
    table_id: Option<i64>,
    start_at: &str,
    end_at: &str,
    tz: &Tz,
    now_utc: DateTime<Utc>,
) -> Result<(), PaymentError> {
    let Some(tid) = table_id else {
        return Ok(());
    };
    if let Ok(dt_utc) = DateTime::parse_from_rfc3339(start_at) {
        let local = dt_utc.with_timezone(tz);
        let date_str = local.format("%Y-%m-%d").to_string();
        let time_str = local.format("%H:%M").to_string();
        let area_str = sqlx::query_scalar::<_, String>("SELECT area FROM dining_tables WHERE id = ?")
            .bind(tid)
            .fetch_optional(&mut *conn)
            .await?
            .unwrap_or_else(|| "indoor".to_string());

        let clock = crate::availability::FixedClock(now_utc);
        let _ = crate::negotiation::match_waitlist_for_slot(
            conn, tid, start_at, end_at, &area_str, &date_str, &time_str, &clock,
        )
        .await?;
    }
    Ok(())
}

/// Expires every overdue pending deposit booking. Returns the number expired.
/// Idempotent: subsequent runs return 0. Runs in a single BEGIN IMMEDIATE.
pub async fn expire_overdue_deposit_bookings(
    pool: &SqlitePool,
    clock: &dyn Clock,
) -> Result<usize, PaymentError> {
    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<usize, PaymentError> = async {
        let restaurant: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents,
                   tax_rate_basis_points, last_order_before_close_minutes, phone,
                   deposit_max_amount_cents, deposit_payment_deadline_minutes,
                   deposit_refund_full_basis_points, deposit_refund_partial_basis_points,
                   deposit_refund_full_hours, deposit_refund_partial_hours,
                   deposit_peak_days, deposit_peak_start_time, deposit_peak_end_time,
                   deposit_demo_mode, deposit_decline_retry_limit
            FROM restaurant WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;
        let tz: Tz = restaurant.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);

        let overdue: Vec<Booking> = sqlx::query_as(
            r#"
            SELECT id, confirmation_code, customer_id, guest_name, party_size,
                   start_at, end_at, status, table_id, special_requests,
                   deposit_status, deposit_amount_minor, payment_deadline_at, cancel_reason,
                   created_at, updated_at, cancelled_at, guest_phone, guest_email
            FROM bookings
            WHERE status = 'pending' AND deposit_status = 'requires_payment'
              AND payment_deadline_at IS NOT NULL AND payment_deadline_at <= ?
            "#,
        )
        .bind(&now_iso)
        .fetch_all(&mut *conn)
        .await?;

        let mut count = 0usize;
        for booking in overdue {
            if expire_pending_booking_in_conn(&mut conn, &booking, now_utc, &tz).await? {
                count += 1;
            }
        }
        Ok(count)
    }
    .await;

    match res {
        Ok(count) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(count)
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Lazily expires a single booking if it is a pending deposit booking past its
/// deadline. Opens its own `BEGIN IMMEDIATE` transaction. Returns true if the
/// booking was expired by this call.
pub async fn lazy_expire_booking_by_id(
    pool: &SqlitePool,
    booking_id: &str,
    clock: &dyn Clock,
) -> Result<bool, PaymentError> {
    let now_utc = clock.now();
    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<bool, PaymentError> = async {
        let booking: Option<Booking> = sqlx::query_as(&format!(
            "SELECT {} FROM bookings WHERE id = ?",
            crate::db::models::BOOKING_SELECT_COLUMNS
        ))
        .bind(booking_id)
        .fetch_optional(&mut *conn)
        .await?;

        let Some(booking) = booking else {
            return Ok(false);
        };

        let tz: Tz = sqlx::query_scalar::<_, String>("SELECT timezone FROM restaurant WHERE id = 1")
            .fetch_optional(&mut *conn)
            .await?
            .and_then(|s| s.parse().ok())
            .unwrap_or(chrono_tz::Asia::Kolkata);

        lazy_expire_booking_in_conn(&mut conn, &booking, now_utc, &tz).await
    }
    .await;

    match res {
        Ok(expired) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(expired)
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Applies the refund schedule to a paid deposit when a booking is cancelled,
/// inside an open transaction. Returns None when no refund applies (e.g. the
/// booking was never paid, or has no captured deposit).
pub async fn refund_deposit_on_cancel_in_conn(
    conn: &mut SqliteConnection,
    booking: &Booking,
    policy: &DepositPolicy,
    currency: &str,
    now_utc: DateTime<Utc>,
) -> Result<Option<RefundSummary>, PaymentError> {
    if booking.deposit_status != DepositStatus::Paid {
        return Ok(None);
    }

    let deposit: Option<(String, i64, String)> = sqlx::query_as(
        r#"
        SELECT id, amount_minor, status FROM payments
        WHERE booking_id = ? AND kind = 'deposit' AND status IN ('captured', 'partially_refunded', 'refunded')
        ORDER BY created_at ASC LIMIT 1
        "#,
    )
    .bind(&booking.id)
    .fetch_optional(&mut *conn)
    .await?;

    let Some((payment_id, deposit_minor, _status)) = deposit else {
        return Ok(None);
    };

    let start = DateTime::parse_from_rfc3339(&booking.start_at)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(now_utc);
    let minutes_until_start = (start - now_utc).num_minutes();
    let quote = compute_refund(deposit_minor, minutes_until_start, policy);
    let now_iso = now_utc.to_rfc3339();

    let (new_booking_status, explanation, percent_display) = if quote.refund_minor <= 0 {
        (
            DepositStatus::Forfeited,
            format!(
                "No refund applies. The reservation was cancelled less than {} hours before its start time.",
                policy.refund_partial_hours
            ),
            "0%".to_string(),
        )
    } else if quote.refund_minor >= deposit_minor {
        (
            DepositStatus::Refunded,
            format!(
                "Full refund of {}. The reservation was cancelled at least {} hours before its start time.",
                money(quote.refund_minor, currency).display,
                policy.refund_full_hours
            ),
            "100%".to_string(),
        )
    } else {
        (
            DepositStatus::PartiallyRefunded,
            format!(
                "Partial refund of {} ({}% of the deposit). The reservation was cancelled within {} hours of its start time.",
                money(quote.refund_minor, currency).display,
                quote.bps / 100,
                policy.refund_full_hours
            ),
            format!("{}%", quote.bps / 100),
        )
    };

    sqlx::query(
        r#"
        UPDATE bookings SET deposit_status = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(new_booking_status)
    .bind(&now_iso)
    .bind(&booking.id)
    .execute(&mut *conn)
    .await?;

    if quote.refund_minor <= 0 {
        // Nothing moved: the captured deposit is retained (forfeited).
        return Ok(Some(RefundSummary {
            amount: money(0, currency),
            percent_bps: quote.bps,
            percent_display,
            explanation,
        }));
    }

    // Mark the original deposit's refund progress.
    let original_status = if quote.refund_minor >= deposit_minor {
        PaymentStatus::Refunded
    } else {
        PaymentStatus::PartiallyRefunded
    };
    sqlx::query("UPDATE payments SET status = ?, updated_at = ? WHERE id = ?")
        .bind(original_status)
        .bind(&now_iso)
        .bind(&payment_id)
        .execute(&mut *conn)
        .await?;

    // Record the refund as its own append-only payment + ledger entry.
    let refund_id = uuid::Uuid::new_v4().to_string();
    let provider_reference = format!("{}_{}", PROVIDER_NAME, uuid::Uuid::new_v4().simple());
    sqlx::query(
        r#"
        INSERT INTO payments (
            id, booking_id, order_id, session_id, kind, amount_minor, currency,
            provider, provider_reference, status, payer_name, payer_contact, created_at, updated_at
        ) VALUES (?, ?, NULL, NULL, 'refund', ?, ?, ?, ?, 'refunded', ?, ?, ?, ?)
        "#,
    )
    .bind(&refund_id)
    .bind(&booking.id)
    .bind(quote.refund_minor)
    .bind(currency)
    .bind(PROVIDER_NAME)
    .bind(&provider_reference)
    .bind(&booking.guest_name)
    .bind(&booking.guest_phone)
    .bind(&now_iso)
    .bind(&now_iso)
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO ledger_entries (payment_id, entry_type, amount_minor, idempotency_key, created_at)
        VALUES (?, 'refund', ?, ?, ?)
        "#,
    )
    .bind(&refund_id)
    .bind(-quote.refund_minor)
    .bind(format!("refund:{}", booking.id))
    .bind(&now_iso)
    .execute(&mut *conn)
    .await?;

    Ok(Some(RefundSummary {
        amount: money(quote.refund_minor, currency),
        percent_bps: quote.bps,
        percent_display,
        explanation,
    }))
}

/// Completes a mock checkout for a session token. Idempotent across retries:
/// a completed session always returns the same final confirmation.
pub async fn complete_checkout(
    pool: &SqlitePool,
    raw_token: &str,
    outcome: CheckoutOutcome,
    clock: &dyn Clock,
) -> Result<CheckoutResult, PaymentError> {
    let now_utc = clock.now();
    let now_iso = now_utc.to_rfc3339();
    let token_hash = hash_payment_token(raw_token);

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<CheckoutResult, PaymentError> = async {
        // 1. Load session by token hash.
        let session: Option<crate::db::models::PaymentSession> = sqlx::query_as(
            "SELECT id, booking_id, token_hash, amount_minor, currency, status, expires_at, decline_count, created_at
             FROM payment_sessions WHERE token_hash = ?",
        )
        .bind(&token_hash)
        .fetch_optional(&mut *conn)
        .await?;

        let Some(session) = session else {
            return Ok(CheckoutResult::new(
                "not_found",
                "That payment link is no longer valid.".to_string(),
            ));
        };

        // 2. Single-use / idempotency: completed sessions replay the same result.
        if session.status == SessionStatus::Completed {
            return Ok(CheckoutResult {
                status: "success".to_string(),
                simulated: true,
                message: "This payment was already completed.".to_string(),
                booking_confirmed: true,
            });
        }
        if session.status != SessionStatus::Open {
            return Ok(CheckoutResult::new(
                "not_found",
                "That payment link is no longer valid.".to_string(),
            ));
        }

        // 3. Load booking and re-check state inside the transaction.
        let booking: Option<Booking> = sqlx::query_as(
            r#"
            SELECT id, confirmation_code, customer_id, guest_name, party_size,
                   start_at, end_at, status, table_id, special_requests,
                   deposit_status, deposit_amount_minor, payment_deadline_at, cancel_reason,
                   created_at, updated_at, cancelled_at, guest_phone, guest_email
            FROM bookings WHERE id = ?
            "#,
        )
        .bind(&session.booking_id)
        .fetch_optional(&mut *conn)
        .await?;

        let Some(booking) = booking else {
            return Ok(CheckoutResult::new(
                "not_found",
                "That payment link is no longer valid.".to_string(),
            ));
        };

        let restaurant: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents,
                   tax_rate_basis_points, last_order_before_close_minutes, phone,
                   deposit_max_amount_cents, deposit_payment_deadline_minutes,
                   deposit_refund_full_basis_points, deposit_refund_partial_basis_points,
                   deposit_refund_full_hours, deposit_refund_partial_hours,
                   deposit_peak_days, deposit_peak_start_time, deposit_peak_end_time,
                   deposit_demo_mode, deposit_decline_retry_limit
            FROM restaurant WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;
        let policy = DepositPolicy::from_config(&restaurant);
        let tz: Tz = restaurant.timezone.parse().unwrap_or(chrono_tz::Asia::Kolkata);

        // 4. Expire if the booking is past its payment deadline (lazy expiry).
        let session_deadline = DateTime::parse_from_rfc3339(&session.expires_at)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(now_utc);
        if now_utc >= session_deadline || lazy_expire_booking_in_conn(&mut conn, &booking, now_utc, &tz).await? {
            let expired = lazy_expire_booking_in_conn(&mut conn, &booking, now_utc, &tz).await?;
            if expired || now_utc >= session_deadline {
                // Ensure the session itself is closed.
                sqlx::query("UPDATE payment_sessions SET status = 'expired' WHERE id = ? AND status = 'open'")
                    .bind(&session.id)
                    .execute(&mut *conn)
                    .await?;
                return Ok(CheckoutResult::new(
                    "expired",
                    "The payment deadline has passed and the hold was released.".to_string(),
                ));
            }
        }

        if booking.status != BookingStatus::Pending
            || booking.deposit_status != DepositStatus::RequiresPayment
        {
            sqlx::query("UPDATE payment_sessions SET status = 'cancelled' WHERE id = ? AND status = 'open'")
                .bind(&session.id)
                .execute(&mut *conn)
                .await?;
            return Ok(CheckoutResult::new(
                "not_found",
                "That payment link is no longer valid.".to_string(),
            ));
        }

        match outcome {
            CheckoutOutcome::Decline => {
                if session.decline_count >= policy.decline_retry_limit {
                    return Ok(CheckoutResult::new(
                        "attempts_exhausted",
                        "That payment link has reached its retry limit.".to_string(),
                    ));
                }
                sqlx::query("UPDATE payment_sessions SET decline_count = decline_count + 1 WHERE id = ?")
                    .bind(&session.id)
                    .execute(&mut *conn)
                    .await?;

                let payment_id = uuid::Uuid::new_v4().to_string();
                let provider_reference = format!("{}_decl_{}", PROVIDER_NAME, uuid::Uuid::new_v4().simple());
                sqlx::query(
                    r#"
                    INSERT INTO payments (
                        id, booking_id, order_id, session_id, kind, amount_minor, currency,
                        provider, provider_reference, status, payer_name, payer_contact, created_at, updated_at
                    ) VALUES (?, ?, NULL, ?, 'deposit', ?, ?, ?, ?, 'failed', ?, ?, ?, ?)
                    "#,
                )
                .bind(&payment_id)
                .bind(&booking.id)
                .bind(&session.id)
                .bind(session.amount_minor)
                .bind(&session.currency)
                .bind(PROVIDER_NAME)
                .bind(&provider_reference)
                .bind(&booking.guest_name)
                .bind(&booking.guest_phone)
                .bind(&now_iso)
                .bind(&now_iso)
                .execute(&mut *conn)
                .await?;

                let left = policy.decline_retry_limit - (session.decline_count + 1);
                return Ok(CheckoutResult::new(
                    "declined",
                    format!(
                        "The payment was declined. The session stays open; {} attempt(s) remaining.",
                        left.max(0)
                    ),
                ));
            }
            CheckoutOutcome::NeedsVerification => {
                // Simulated step-up: move the payment to requires_action once.
                let existing: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM payments WHERE session_id = ? AND kind = 'deposit'
                     AND status = 'requires_action' LIMIT 1",
                )
                .bind(&session.id)
                .fetch_optional(&mut *conn)
                .await?;

                if existing.is_none() {
                    let payment_id = uuid::Uuid::new_v4().to_string();
                    let provider_reference =
                        format!("{}_auth_{}", PROVIDER_NAME, uuid::Uuid::new_v4().simple());
                    sqlx::query(
                        r#"
                        INSERT INTO payments (
                            id, booking_id, order_id, session_id, kind, amount_minor, currency,
                            provider, provider_reference, status, payer_name, payer_contact, created_at, updated_at
                        ) VALUES (?, ?, NULL, ?, 'deposit', ?, ?, ?, ?, 'requires_action', ?, ?, ?, ?)
                        "#,
                    )
                    .bind(&payment_id)
                    .bind(&booking.id)
                    .bind(&session.id)
                    .bind(session.amount_minor)
                    .bind(&session.currency)
                    .bind(PROVIDER_NAME)
                    .bind(&provider_reference)
                    .bind(&booking.guest_name)
                    .bind(&booking.guest_phone)
                    .bind(&now_iso)
                    .bind(&now_iso)
                    .execute(&mut *conn)
                    .await?;
                }

                return Ok(CheckoutResult::new(
                    "needs_verification",
                    "Extra verification is required. Confirm again to complete the payment.".to_string(),
                ));
            }
            CheckoutOutcome::Success => {
                // 5. Authorize + capture (authorize-then-capture, immediately).
                let existing: Option<(String, PaymentStatus)> = sqlx::query_as(
                    "SELECT id, status FROM payments WHERE session_id = ? AND kind = 'deposit'
                     AND status IN ('requires_action', 'authorized', 'captured')
                     ORDER BY created_at DESC LIMIT 1",
                )
                .bind(&session.id)
                .fetch_optional(&mut *conn)
                .await?;

                let (payment_id, initial_state) = match existing {
                    Some((id, status)) => (id, PaymentState::from_status(status)),
                    None => {
                        let id = uuid::Uuid::new_v4().to_string();
                        let provider_reference =
                            format!("{}_pay_{}", PROVIDER_NAME, uuid::Uuid::new_v4().simple());
                        sqlx::query(
                            r#"
                            INSERT INTO payments (
                                id, booking_id, order_id, session_id, kind, amount_minor, currency,
                                provider, provider_reference, status, payer_name, payer_contact, created_at, updated_at
                            ) VALUES (?, ?, NULL, ?, 'deposit', ?, ?, ?, ?, 'requires_action', ?, ?, ?, ?)
                            "#,
                        )
                        .bind(&id)
                        .bind(&booking.id)
                        .bind(&session.id)
                        .bind(session.amount_minor)
                        .bind(&session.currency)
                        .bind(PROVIDER_NAME)
                        .bind(&provider_reference)
                        .bind(&booking.guest_name)
                        .bind(&booking.guest_phone)
                        .bind(&now_iso)
                        .bind(&now_iso)
                        .execute(&mut *conn)
                        .await?;
                        (id, PaymentState::RequiresAction)
                    }
                };

                if initial_state != PaymentState::Captured {
                    let authorized = apply_payment_transition(initial_state, PaymentState::Authorized)?;
                    let captured = apply_payment_transition(authorized, PaymentState::Captured)?;
                    sqlx::query("UPDATE payments SET status = ?, updated_at = ? WHERE id = ?")
                        .bind(captured.to_status())
                        .bind(&now_iso)
                        .bind(&payment_id)
                        .execute(&mut *conn)
                        .await?;

                    insert_ledger_entry(
                        &mut conn,
                        &payment_id,
                        LedgerEntryType::Authorize,
                        session.amount_minor,
                        &format!("authorize:{}", payment_id),
                        &now_iso,
                    )
                    .await?;
                    insert_ledger_entry(
                        &mut conn,
                        &payment_id,
                        LedgerEntryType::Capture,
                        session.amount_minor,
                        &format!("capture:{}", booking.id),
                        &now_iso,
                    )
                    .await?;
                }

                // Fault-injection point: everything above must roll back.
                if FAIL_AFTER_LEDGER_INSERT.load(Ordering::SeqCst) {
                    return Err(PaymentError::FaultInjected);
                }

                // 6. Confirm the booking and complete the session.
                sqlx::query(
                    r#"
                    UPDATE bookings
                    SET status = 'confirmed', deposit_status = 'paid', updated_at = ?
                    WHERE id = ? AND status = 'pending'
                    "#,
                )
                .bind(&now_iso)
                .bind(&booking.id)
                .execute(&mut *conn)
                .await?;

                sqlx::query("UPDATE payment_sessions SET status = 'completed' WHERE id = ?")
                    .bind(&session.id)
                    .execute(&mut *conn)
                    .await?;

                let payload = serde_json::json!({
                    "payment_id": payment_id,
                    "amount_minor": session.amount_minor,
                    "currency": session.currency,
                    "provider": PROVIDER_NAME,
                });
                sqlx::query(
                    r#"
                    INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
                    VALUES ('payment', ?, 'captured', 'guest', ?, ?)
                    "#,
                )
                .bind(&payment_id)
                .bind(payload.to_string())
                .bind(&now_iso)
                .execute(&mut *conn)
                .await?;

                return Ok(CheckoutResult {
                    status: "success".to_string(),
                    simulated: true,
                    message: "Payment completed (simulated). Your table is confirmed.".to_string(),
                    booking_confirmed: true,
                });
            }
        }
    }
    .await;

    match res {
        Ok(result) => {
            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(result)
        }
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

async fn insert_ledger_entry(
    conn: &mut SqliteConnection,
    payment_id: &str,
    entry_type: LedgerEntryType,
    amount_minor: i64,
    idempotency_key: &str,
    created_at: &str,
) -> Result<(), PaymentError> {
    sqlx::query(
        r#"
        INSERT INTO ledger_entries (payment_id, entry_type, amount_minor, idempotency_key, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(payment_id)
    .bind(entry_type)
    .bind(amount_minor)
    .bind(idempotency_key)
    .bind(created_at)
    .execute(&mut *conn)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            PaymentError::LedgerConflict(idempotency_key.to_string())
        }
        _ => PaymentError::Db(e),
    })?;
    Ok(())
}

/// Derived ledger balance for a payment: sum of all append-only entries.
/// Never edited in place; always recomputed from entries.
pub async fn payment_balance(
    conn: &mut SqliteConnection,
    payment_id: &str,
) -> Result<i64, PaymentError> {
    let sum: Option<i64> =
        sqlx::query_scalar("SELECT COALESCE(SUM(amount_minor), 0) FROM ledger_entries WHERE payment_id = ?")
            .bind(payment_id)
            .fetch_one(&mut *conn)
            .await?;
    Ok(sum.unwrap_or(0))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn policy() -> DepositPolicy {
        DepositPolicy::defaults()
    }

    fn local(dt: &str) -> DateTime<Tz> {
        let utc = DateTime::parse_from_rfc3339(dt).unwrap().with_timezone(&Utc);
        utc.with_timezone(&chrono_tz::Asia::Kolkata)
    }

    #[test]
    fn test_deposit_threshold_and_peak_window() {
        let p = policy();
        // Party of 5 off-peak -> no deposit; party of 6 -> deposit.
        assert!(!deposit_required(&p, 5, &local("2026-10-21T19:30:00Z")));
        assert!(deposit_required(&p, 6, &local("2026-10-21T19:30:00Z")));
        assert!(deposit_required(&p, 8, &local("2026-10-21T13:00:00Z")));

        // Configured peak window: Friday/Saturday dinner requires a deposit.

        let peak = DepositPolicy {
            peak_days: vec![4, 5],
            peak_start: Some(NaiveTime::from_hms_opt(19, 0, 0).unwrap()),
            peak_end: Some(NaiveTime::from_hms_opt(23, 30, 0).unwrap()),
            ..policy()
        };
        // 2026-10-16 is a Friday. 19:30 IST -> 14:00 UTC.
        assert!(deposit_required(&peak, 2, &local("2026-10-16T14:00:00Z")));
        // Friday lunch -> no peak deposit.
        assert!(!deposit_required(&peak, 2, &local("2026-10-16T07:00:00Z")));
        // Wednesday dinner -> no peak deposit.
        assert!(!deposit_required(&peak, 2, &local("2026-10-21T14:00:00Z")));
    }

    #[test]
    fn test_compute_deposit_amount_per_person_and_cap() {
        let p = policy(); // 25000 per person, cap 400000
        assert_eq!(compute_deposit_amount(&p, 1).unwrap(), 25_000);
        assert_eq!(compute_deposit_amount(&p, 8).unwrap(), 200_000); // ₹2000 demo moment
        assert_eq!(compute_deposit_amount(&p, 10).unwrap(), 250_000);
        // Cap applies at party of 16 (16 * 25000 = 400000 exactly) and beyond.
        assert_eq!(compute_deposit_amount(&p, 20).unwrap(), 400_000);
    }

    #[test]
    fn test_compute_deposit_amount_overflow() {
        let p = DepositPolicy {
            per_person_minor: i64::MAX,
            max_deposit_minor: i64::MAX,
            ..policy()
        };
        assert!(matches!(
            compute_deposit_amount(&p, 2),
            Err(PaymentError::Overflow)
        ));
    }

    #[test]
    fn test_refund_schedule_boundaries() {
        let p = policy(); // full >= 24h, partial >= 2h, else none
        // Exactly 24 hours -> full refund.
        assert_eq!(compute_refund(200_000, 24 * 60, &p).refund_minor, 200_000);
        // Just under 24 hours -> partial.
        assert_eq!(
            compute_refund(200_000, 24 * 60 - 1, &p).refund_minor,
            100_000
        );
        // Exactly 2 hours -> partial.
        assert_eq!(compute_refund(200_000, 2 * 60, &p).refund_minor, 100_000);
        // Just under 2 hours -> none.
        assert_eq!(compute_refund(200_000, 2 * 60 - 1, &p).refund_minor, 0);
        // Far out -> full.
        assert_eq!(compute_refund(200_000, 100 * 60, &p).refund_minor, 200_000);
    }

    #[test]
    fn test_refund_round_half_up() {
        // 50% of an odd amount rounds half up: 501 * 5000 / 10000 = 250.5 -> 251
        assert_eq!(apply_basis_points_round_half_up(501, 5000), 251);
        assert_eq!(apply_basis_points_round_half_up(500, 5000), 250);
        assert_eq!(apply_basis_points_round_half_up(1, 5000), 1); // 0.5 -> 1
        assert_eq!(apply_basis_points_round_half_up(1, 3333), 0); // 0.333 -> 0
        assert_eq!(apply_basis_points_round_half_up(0, 5000), 0);
        assert_eq!(apply_basis_points_round_half_up(1000, 10000), 1000);
    }

    #[test]
    fn test_refund_plus_retained_equals_deposit_property() {
        // Deterministic pseudo-random sweep over amounts and percentages.
        let mut seed: i64 = 12345;
        for bps in [0, 1, 2500, 3333, 5000, 6667, 9000, 10000] {
            for _ in 0..200 {
                seed = (seed * 1103515245 + 12345) & 0x7fffffff;
                let amount = seed % 1_000_000;
                let refund = apply_basis_points_round_half_up(amount, bps);
                let retained = amount - refund;
                assert_eq!(refund + retained, amount);
                assert!(refund >= 0);
                assert!(retained >= 0);
                assert!(refund <= amount);
            }
        }
    }

    #[test]
    fn test_payment_state_machine_allowed_and_illegal() {
        use PaymentState::*;
        assert!(can_transition_payment(RequiresAction, Authorized));
        assert!(can_transition_payment(RequiresAction, Failed));
        assert!(can_transition_payment(RequiresAction, Expired));
        assert!(can_transition_payment(Authorized, Captured));
        assert!(can_transition_payment(Authorized, Voided));
        assert!(can_transition_payment(Captured, PartiallyRefunded));
        assert!(can_transition_payment(Captured, Refunded));
        assert!(can_transition_payment(PartiallyRefunded, Refunded));

        // Illegal transitions
        assert!(!can_transition_payment(Captured, Authorized));
        assert!(!can_transition_payment(Refunded, Captured));
        assert!(!can_transition_payment(Failed, Captured));
        assert!(!can_transition_payment(Expired, Authorized));
        assert!(!can_transition_payment(Voided, Captured));
        assert!(!can_transition_payment(RequiresAction, Captured));

        assert!(apply_payment_transition(RequiresAction, Authorized).is_ok());
        assert!(apply_payment_transition(Captured, Authorized).is_err());
    }

    #[test]
    fn test_token_format_and_hashing() {
        let (plain, hash) = generate_payment_token();
        assert!(plain.starts_with("tt_pay_"));
        assert_eq!(plain.len(), "tt_pay_".len() + 32);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, hash_payment_token(&plain));
        // Two tokens differ.
        let (plain2, _) = generate_payment_token();
        assert_ne!(plain, plain2);
        // Token is not stored in plain form anywhere derivable from the hash.
        assert!(!hash.contains(&plain));
        assert!(looks_like_payment_token(&plain));
        assert!(!looks_like_payment_token("nope"));
    }

    #[test]
    fn test_luhn_and_card_detection() {
        // A known Luhn-valid synthetic test number (not a real card).
        let test_number = generate_luhn_valid_16();
        assert!(contains_card_number(&test_number));
        // Spaced and dashed forms.
        let spaced = test_number
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        let dashed = test_number
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect::<Vec<_>>()
            .join("-");
        assert!(contains_card_number(&spaced));
        assert!(contains_card_number(&dashed));

        // Luhn-invalid 16 digits are not flagged.
        assert!(!contains_card_number("1234567812345678"));
        // Short numbers are not flagged.
        assert!(!contains_card_number("1234 5678 9012"));

        // Field-name detection.
        let body = serde_json::json!({"outcome": "success", "cvv": "123"});
        assert!(body_contains_card_data(&body));
        let body2 = serde_json::json!({"card_number": "4111"});
        assert!(body_contains_card_data(&body2));
        let ok = serde_json::json!({"outcome": "success"});
        assert!(!body_contains_card_data(&ok));
        let nested = serde_json::json!({"outcome": "success", "meta": {"pan": "x"}});
        assert!(body_contains_card_data(&nested));
    }

    /// Generates a random 16-digit Luhn-valid number using only synthetic digits.
    fn generate_luhn_valid_16() -> String {
        // Deterministic pseudo-random base digits + correct check digit.
        let mut seed: u64 = 42;
        let mut digits: Vec<u8> = Vec::with_capacity(15);
        for _ in 0..15 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            digits.push(((seed >> 33) % 10) as u8);
        }
        // Find check digit so full 16-digit number is Luhn valid.
        for check in 0..=9u8 {
            let mut candidate = digits.clone();
            candidate.push(check);
            if luhn_valid(&candidate) {
                return candidate.iter().map(|d| char::from(b'0' + d)).collect();
            }
        }
        unreachable!("a check digit always exists")
    }

    #[test]
    fn test_spoken_money_and_minutes() {
        assert_eq!(spoken_money(200_000, "INR"), "two thousand rupees");
        assert_eq!(spoken_money(50, "INR"), "fifty paise");
        assert_eq!(
            spoken_money(12_345, "INR"),
            "one hundred twenty-three rupees and forty-five paise"
        );
        assert_eq!(
            spoken_money(1_250_000, "INR"),
            "twelve thousand five hundred rupees"
        );
        assert_eq!(spoken_minutes(15), "fifteen minutes");
        assert_eq!(spoken_minutes(1), "one minute");
        assert_eq!(spoken_minutes(0), "less than a minute");
        assert_eq!(spoken_count(8), "eight");
    }

    #[test]
    fn test_voice_texts_are_natural_and_hide_tokens() {
        let p = policy();
        let text = voice_deposit_required(&p, 200_000, "INR", 8, 15, "TT-8K4MZ");
        assert!(text.contains("A table for eight needs a deposit of two thousand rupees."));
        assert!(text.contains("hold the table for fifteen minutes"));
        assert!(text.contains("simulated"));
        assert!(text.contains("T T 8 K 4 M Z"));
        assert!(!text.contains("tt_pay_"));
        assert!(!text.contains("http"));

        let expired = voice_payment_expired();
        assert!(!expired.contains("http"));
        assert!(!expired.contains("tt_pay_"));
    }

    #[test]
    fn test_minutes_remaining_rounds_up() {
        let now = Utc.with_ymd_and_hms(2026, 10, 13, 12, 0, 0).unwrap();
        assert_eq!(minutes_remaining(now + Duration::seconds(1), now), 1);
        assert_eq!(minutes_remaining(now + Duration::seconds(60), now), 1);
        assert_eq!(minutes_remaining(now + Duration::seconds(61), now), 2);
        assert_eq!(minutes_remaining(now - Duration::seconds(5), now), 0);
    }
}
