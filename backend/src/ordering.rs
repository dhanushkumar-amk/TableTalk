use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use rand::Rng;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{
    availability::{format_spoken_time, Clock},
    db::models::{
        MenuItem, OpeningHours, Order, OrderItem, OrderStatus, RestaurantConfig, SpecialClosure,
    },
    menu_search::{AllergenFilter, PriceDisplay, ALLERGEN_NOTICE},
};

// --- Constants ---

pub const ORDER_CODE_PREFIX: &str = "TO-";
pub const ORDER_CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
pub const ORDER_CODE_BODY_LEN: usize = 5;

pub const MAX_ORDER_LINES: usize = 20;
pub const MAX_LINE_QUANTITY: i64 = 20;
pub const MAX_ORDER_TOTAL_QUANTITY: i64 = 50;
pub const MAX_LINE_NOTES_LEN: usize = 140;
pub const MAX_ORDER_NOTES_LEN: usize = 300;
pub const MAX_GUEST_NAME_LEN: usize = 80;
pub const MAX_ORDER_TOTAL_CENTS: i64 = 2_000_000; // ₹20,000 maximum order
pub const MAX_ACTIVE_ORDERS_PER_GUEST: i64 = 3;
pub const IDEMPOTENCY_KEY_EXPIRY_HOURS: i64 = 24;

// --- State Machine ---

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderTransitionError {
    #[error("Illegal order status transition from '{from:?}' to '{to:?}'")]
    IllegalTransition { from: OrderStatus, to: OrderStatus },
}

/// Enforces the explicit state machine transitions for orders:
/// - Draft -> Placed, Cancelled
/// - Placed -> Preparing, Cancelled
/// - Preparing -> Ready
/// - Ready -> Completed
///
/// All other transitions (including reviving cancelled or completed orders) are illegal.
pub fn can_transition(current: OrderStatus, target: OrderStatus) -> bool {
    matches!(
        (current, target),
        (OrderStatus::Draft, OrderStatus::Placed)
            | (OrderStatus::Draft, OrderStatus::Cancelled)
            | (OrderStatus::Placed, OrderStatus::Preparing)
            | (OrderStatus::Placed, OrderStatus::Cancelled)
            | (OrderStatus::Preparing, OrderStatus::Ready)
            | (OrderStatus::Ready, OrderStatus::Completed)
    )
}

pub fn apply_transition(
    current: OrderStatus,
    target: OrderStatus,
) -> Result<OrderStatus, OrderTransitionError> {
    if can_transition(current, target) {
        Ok(target)
    } else {
        Err(OrderTransitionError::IllegalTransition {
            from: current,
            to: target,
        })
    }
}

// --- Order Code Generator & Normalizer ---

/// Generates a random human-friendly order code in format `TO-XXXXX`.
pub fn generate_order_code() -> String {
    let mut rng = rand::thread_rng();
    let mut body = String::with_capacity(ORDER_CODE_BODY_LEN);
    for _ in 0..ORDER_CODE_BODY_LEN {
        let idx = rng.gen_range(0..ORDER_CODE_ALPHABET.len());
        body.push(ORDER_CODE_ALPHABET[idx] as char);
    }
    format!("{}{}", ORDER_CODE_PREFIX, body)
}

/// Normalizes an order code input:
/// - Trims and converts to uppercase
/// - Strips spaces and hyphens
/// - Ensures `TO-` prefix is present
pub fn normalize_order_code(input: &str) -> String {
    let upper = input.trim().to_ascii_uppercase();
    let clean: String = upper
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();

    if let Some(body) = clean.strip_prefix("TO") {
        format!("{}{}", ORDER_CODE_PREFIX, body)
    } else {
        format!("{}{}", ORDER_CODE_PREFIX, clean)
    }
}

/// Formats an order code for natural speech (e.g. `TO-8K4MZ` -> `"T O 8 K 4 M Z"`).
pub fn format_spoken_order_code(code: &str) -> String {
    let clean: Vec<char> = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    clean
        .into_iter()
        .map(|c| c.to_string())
        .collect::<Vec<String>>()
        .join(" ")
}

// --- Money & Spoken Currency Formatting ---

/// Formats integer paise into a human-readable PriceDisplay.
pub fn format_price_display(amount_minor: i64, currency: &str) -> PriceDisplay {
    let rupees = amount_minor / 100;
    let paise = amount_minor % 100;
    let display = if paise == 0 {
        format!("₹{}", rupees)
    } else {
        format!("₹{}.{:02}", rupees, paise)
    };
    PriceDisplay {
        amount_minor,
        currency: currency.to_string(),
        display,
    }
}

/// Converts minor units (paise) into natural spoken English rupees.
/// Handles zeroes, singular vs plural, and paise if present.
pub fn paise_to_spoken_rupees(cents: i64) -> String {
    if cents == 0 {
        return "zero rupees".to_string();
    }
    let rupees = cents / 100;
    let paise = cents % 100;

    let rupee_words = if rupees > 0 {
        let words = number_to_words(rupees);
        let unit = if rupees == 1 { "rupee" } else { "rupees" };
        format!("{} {}", words, unit)
    } else {
        String::new()
    };

    let paise_words = if paise > 0 {
        let words = number_to_words(paise);
        let unit = if paise == 1 { "paisa" } else { "paise" };
        format!("{} {}", words, unit)
    } else {
        String::new()
    };

    if !rupee_words.is_empty() && !paise_words.is_empty() {
        format!("{} and {}", rupee_words, paise_words)
    } else if !rupee_words.is_empty() {
        rupee_words
    } else {
        paise_words
    }
}

fn number_to_words(n: i64) -> String {
    if n == 0 {
        return "zero".to_string();
    }
    let mut words = String::new();
    let mut num = n;

    if num >= 1000 {
        let thousands = num / 1000;
        words.push_str(&format!("{} thousand", number_to_words(thousands)));
        num %= 1000;
        if num > 0 {
            words.push(' ');
        }
    }

    if num >= 100 {
        let hundreds = num / 100;
        words.push_str(&format!("{} hundred", number_to_words(hundreds)));
        num %= 100;
        if num > 0 {
            words.push(' ');
        }
    }

    if num > 0 {
        let under_100 = match num {
            1 => "one".to_string(),
            2 => "two".to_string(),
            3 => "three".to_string(),
            4 => "four".to_string(),
            5 => "five".to_string(),
            6 => "six".to_string(),
            7 => "seven".to_string(),
            8 => "eight".to_string(),
            9 => "nine".to_string(),
            10 => "ten".to_string(),
            11 => "eleven".to_string(),
            12 => "twelve".to_string(),
            13 => "thirteen".to_string(),
            14 => "fourteen".to_string(),
            15 => "fifteen".to_string(),
            16 => "sixteen".to_string(),
            17 => "seventeen".to_string(),
            18 => "eighteen".to_string(),
            19 => "nineteen".to_string(),
            20..=29 => {
                let rest = num % 10;
                if rest == 0 {
                    "twenty".to_string()
                } else {
                    format!("twenty-{}", number_to_words(rest))
                }
            }
            30..=39 => {
                let rest = num % 10;
                if rest == 0 {
                    "thirty".to_string()
                } else {
                    format!("thirty-{}", number_to_words(rest))
                }
            }
            40..=49 => {
                let rest = num % 10;
                if rest == 0 {
                    "forty".to_string()
                } else {
                    format!("forty-{}", number_to_words(rest))
                }
            }
            50..=59 => {
                let rest = num % 10;
                if rest == 0 {
                    "fifty".to_string()
                } else {
                    format!("fifty-{}", number_to_words(rest))
                }
            }
            60..=69 => {
                let rest = num % 10;
                if rest == 0 {
                    "sixty".to_string()
                } else {
                    format!("sixty-{}", number_to_words(rest))
                }
            }
            70..=79 => {
                let rest = num % 10;
                if rest == 0 {
                    "seventy".to_string()
                } else {
                    format!("seventy-{}", number_to_words(rest))
                }
            }
            80..=89 => {
                let rest = num % 10;
                if rest == 0 {
                    "eighty".to_string()
                } else {
                    format!("eighty-{}", number_to_words(rest))
                }
            }
            90..=99 => {
                let rest = num % 10;
                if rest == 0 {
                    "ninety".to_string()
                } else {
                    format!("ninety-{}", number_to_words(rest))
                }
            }
            _ => String::new(),
        };
        words.push_str(&under_100);
    }

    words
}

// --- Tool Input & Output Types ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OrderLineInput {
    pub item_id: Option<i64>,
    pub item_name: Option<String>,
    pub quantity: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AllergenAcknowledgement {
    pub allergens_to_avoid: Vec<AllergenFilter>,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OrderLineSummary {
    pub item_id: i64,
    pub name: String,
    pub quantity: i64,
    pub unit_price: PriceDisplay,
    pub line_total: PriceDisplay,
    pub notes: Option<String>,
    pub allergens: Vec<String>,
    pub dietary_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OrderSummary {
    pub order_code: String,
    pub guest_name: String,
    pub status: String,
    pub pickup_time_local: String,
    pub lines: Vec<OrderLineSummary>,
    pub subtotal: PriceDisplay,
    pub tax: PriceDisplay,
    pub total: PriceDisplay,
    pub payment_status: String,
    pub stated_allergens: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ItemCandidate {
    pub item_id: i64,
    pub name: String,
    pub price: PriceDisplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ConflictingLine {
    pub requested_name: String,
    pub matched_item_id: i64,
    pub matched_item_name: String,
    pub allergens_found: Vec<String>,
    pub unverified_allergen_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PriceChange {
    pub item_id: i64,
    pub item_name: String,
    pub current_price: PriceDisplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PlaceOrderOutput {
    pub status: String,
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
    pub order: Option<OrderSummary>,
    pub price_changes: Vec<PriceChange>,
    pub candidates: Vec<ItemCandidate>,
    pub alternatives: Vec<String>,
    pub earliest_pickup_local: Option<String>,
    pub allergen_notice: String,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct CancelOrderOutput {
    pub status: String,
    pub already_cancelled: bool,
    pub order: Option<OrderSummary>,
    pub policy_note: Option<String>,
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GetOrderOutput {
    pub status: String,
    pub order: Option<OrderSummary>,
    pub message: String,
}

// --- Outcome Enums ---

pub enum PlaceOrderOutcome {
    Placed {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    ItemUnavailable {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    UnknownItem {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    AmbiguousItem {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    PickupTooSoon {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    Closed {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    AllergenConflict {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    LimitExceeded {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    IdempotentReplay {
        output: PlaceOrderOutput,
        voice_text: String,
    },
    IdempotencyConflict {
        output: PlaceOrderOutput,
        voice_text: String,
    },
}

pub enum CancelOrderOutcome {
    Cancelled {
        output: CancelOrderOutput,
        voice_text: String,
    },
    AlreadyCancelled {
        output: CancelOrderOutput,
        voice_text: String,
    },
    NotCancellable {
        output: CancelOrderOutput,
        voice_text: String,
    },
    NotFound {
        output: CancelOrderOutput,
        voice_text: String,
    },
    RateLimited {
        output: CancelOrderOutput,
        voice_text: String,
    },
}

pub enum GetOrderOutcome {
    Found {
        output: GetOrderOutput,
        voice_text: String,
    },
    NotFound {
        output: GetOrderOutput,
        voice_text: String,
    },
}

// --- Validation Errors ---

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderValidationError {
    #[error("Order must contain between 1 and 20 items. Received {0}.")]
    InvalidLineCount(usize),
    #[error("Line quantity must be between 1 and 20. Received {0}.")]
    InvalidLineQuantity(i64),
    #[error("Total order quantity exceeds limit of 50 items. Received {0}.")]
    TotalQuantityTooHigh(i64),
    #[error("Line notes exceed maximum length of 140 characters.")]
    LineNotesTooLong,
    #[error("Order notes exceed maximum length of 300 characters.")]
    OrderNotesTooLong,
    #[error("Guest name is required and must be between 1 and 80 characters.")]
    InvalidGuestName,
    #[error("Either 'item_id' or 'item_name' must be provided for each item line.")]
    MissingItemIdentifier,
    #[error("Invalid idempotency key '{0}'. Must be 8-64 characters matching [a-zA-Z0-9_-].")]
    InvalidIdempotencyKey(String),
    #[error("Invalid phone number format.")]
    InvalidPhone,
    #[error("Invalid email address format.")]
    InvalidEmail,
    #[error("Invalid pickup time format '{0}'. Must be 'HH:MM' or ISO date-time.")]
    InvalidPickupTime(String),
    #[error("Arithmetic overflow calculating pricing totals.")]
    PricingOverflow,
}

/// Calculates pricing totals (subtotal, tax, total) using integer arithmetic with round-half-up tax calculation.
pub fn calculate_order_pricing(
    line_prices: &[(i64, i64)],
    tax_rate_basis_points: i64,
) -> Result<(i64, i64, i64), OrderValidationError> {
    let mut subtotal: i64 = 0;
    for &(unit_price, qty) in line_prices {
        let line_total = unit_price
            .checked_mul(qty)
            .ok_or(OrderValidationError::PricingOverflow)?;
        subtotal = subtotal
            .checked_add(line_total)
            .ok_or(OrderValidationError::PricingOverflow)?;
    }
    let tax = if tax_rate_basis_points > 0 {
        let num = subtotal
            .checked_mul(tax_rate_basis_points)
            .ok_or(OrderValidationError::PricingOverflow)?
            + 5000;
        num / 10000
    } else {
        0
    };
    let total = subtotal
        .checked_add(tax)
        .ok_or(OrderValidationError::PricingOverflow)?;
    Ok((subtotal, tax, total))
}

pub fn validate_order_request(
    items: &[OrderLineInput],
    guest_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    order_notes: Option<&str>,
    idempotency_key: Option<&str>,
) -> Result<(), OrderValidationError> {
    if items.is_empty() || items.len() > MAX_ORDER_LINES {
        return Err(OrderValidationError::InvalidLineCount(items.len()));
    }

    let mut total_qty = 0i64;
    for line in items {
        if line.quantity < 1 || line.quantity > MAX_LINE_QUANTITY {
            return Err(OrderValidationError::InvalidLineQuantity(line.quantity));
        }
        total_qty += line.quantity;

        if line.item_id.is_none() && line.item_name.as_deref().unwrap_or("").trim().is_empty() {
            return Err(OrderValidationError::MissingItemIdentifier);
        }

        if let Some(n) = &line.notes {
            if n.len() > MAX_LINE_NOTES_LEN {
                return Err(OrderValidationError::LineNotesTooLong);
            }
        }
    }

    if total_qty > MAX_ORDER_TOTAL_QUANTITY {
        return Err(OrderValidationError::TotalQuantityTooHigh(total_qty));
    }

    let trimmed_name = guest_name.trim();
    if trimmed_name.is_empty() || trimmed_name.len() > MAX_GUEST_NAME_LEN {
        return Err(OrderValidationError::InvalidGuestName);
    }

    if let Some(notes) = order_notes {
        if notes.len() > MAX_ORDER_NOTES_LEN {
            return Err(OrderValidationError::OrderNotesTooLong);
        }
    }

    if let Some(key) = idempotency_key {
        let len = key.len();
        if !(8..=64).contains(&len)
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(OrderValidationError::InvalidIdempotencyKey(key.to_string()));
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
                return Err(OrderValidationError::InvalidPhone);
            }
        }
    }

    if let Some(e) = email {
        let e_trimmed = e.trim();
        if !e_trimmed.is_empty()
            && (!(3..=100).contains(&e_trimmed.len())
                || !e_trimmed.contains('@')
                || e_trimmed.starts_with('@')
                || e_trimmed.ends_with('@'))
        {
            return Err(OrderValidationError::InvalidEmail);
        }
    }

    Ok(())
}

// --- Idempotency Hash ---

pub fn compute_order_request_hash(
    items: &[OrderLineInput],
    pickup_time: Option<&str>,
    guest_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    allergen_ack: Option<&AllergenAcknowledgement>,
    order_notes: Option<&str>,
) -> String {
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct CanonicalOrderRequest<'a> {
        items: &'a [OrderLineInput],
        pickup_time: Option<&'a str>,
        guest_name: &'a str,
        phone: Option<&'a str>,
        email: Option<&'a str>,
        allergen_ack: Option<&'a AllergenAcknowledgement>,
        order_notes: Option<&'a str>,
    }

    let canonical = CanonicalOrderRequest {
        items,
        pickup_time,
        guest_name: guest_name.trim(),
        phone: phone.map(str::trim),
        email: email.map(str::trim),
        allergen_ack,
        order_notes: order_notes.map(str::trim),
    };

    let serialized = serde_json::to_string(&canonical).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(serialized.as_bytes());
    format!("{:x}", hasher.finalize())
}

// --- In-Memory Rate Limiter ---

#[derive(Clone, Default)]
pub struct OrderCancellationRateLimiter {
    attempts: Arc<Mutex<HashMap<String, Vec<DateTime<Utc>>>>>,
}

impl OrderCancellationRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn check_and_record(&self, code: &str, now: DateTime<Utc>) -> bool {
        let normalized = normalize_order_code(code);
        let mut map = self.attempts.lock().unwrap();
        let cutoff = now - Duration::minutes(15);
        let entry = map.entry(normalized).or_default();
        entry.retain(|&t| t > cutoff);

        if entry.len() >= 5 {
            return false;
        }
        entry.push(now);
        true
    }
}

// --- Transactional Operations ---

pub struct PlaceOrderTxRequest<'a> {
    pub items: &'a [OrderLineInput],
    pub pickup_time: Option<&'a str>,
    pub guest_name: &'a str,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    pub allergen_ack: Option<&'a AllergenAcknowledgement>,
    pub order_notes: Option<&'a str>,
    pub idempotency_key: Option<&'a str>,
}

pub struct CancelOrderTxRequest<'a> {
    pub order_code: &'a str,
    pub guest_name: &'a str,
    pub reason: Option<&'a str>,
}

/// Executes transactional order placement inside `BEGIN IMMEDIATE`.
pub async fn place_order_tx(
    pool: &SqlitePool,
    req: PlaceOrderTxRequest<'_>,
    clock: &dyn Clock,
) -> Result<PlaceOrderOutcome, sqlx::Error> {
    let now = clock.now();
    let now_utc_iso = now.to_rfc3339();

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<PlaceOrderOutcome, sqlx::Error> = async {
        // 1. Load restaurant configuration
        let restaurant: RestaurantConfig = sqlx::query_as(
            r#"
            SELECT id, name, timezone, currency, dining_duration_minutes, slot_granularity_minutes,
                   max_party_size, lead_time_minutes, booking_horizon_days, deposit_required_party_size,
                   deposit_amount_per_person_cents, created_at, updated_at,
                   cancellation_cutoff_hours, late_cancellation_fee_cents,
                   tax_rate_basis_points, last_order_before_close_minutes, phone
            FROM restaurant
            WHERE id = 1
            "#,
        )
        .fetch_one(&mut *conn)
        .await?;

        let rest_tz: Tz = restaurant
            .timezone
            .parse()
            .unwrap_or(chrono_tz::Asia::Kolkata);
        let now_local = now.with_timezone(&rest_tz);

        // 2. Check idempotency key if supplied
        let request_hash = compute_order_request_hash(
            req.items,
            req.pickup_time,
            req.guest_name,
            req.phone,
            req.email,
            req.allergen_ack,
            req.order_notes,
        );

        if let Some(key) = req.idempotency_key {
            let existing: Option<(String, String)> = sqlx::query_as(
                "SELECT request_hash, response_json FROM idempotency_keys WHERE tool_name = 'place_order' AND key = ?",
            )
            .bind(key)
            .fetch_optional(&mut *conn)
            .await?;

            if let Some((stored_hash, cached_json)) = existing {
                if stored_hash == request_hash {
                    if let Ok(mut cached_out) = serde_json::from_str::<PlaceOrderOutput>(&cached_json) {
                        cached_out.idempotent_replay = true;
                        let voice_text = generate_place_order_voice_text(&cached_out);
                        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                        return Ok(PlaceOrderOutcome::IdempotentReplay {
                            output: cached_out,
                            voice_text,
                        });
                    }
                } else {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                    let output = PlaceOrderOutput {
                        status: "idempotency_conflict".to_string(),
                        reason: Some("idempotency_key_reused_with_different_parameters".to_string()),
                        reason_detail: Some(format!(
                            "The idempotency key '{}' was already used with different parameters.",
                            key
                        )),
                        order: None,
                        price_changes: vec![],
                        candidates: vec![],
                        alternatives: vec![],
                        earliest_pickup_local: None,
                        allergen_notice: ALLERGEN_NOTICE.to_string(),
                        idempotent_replay: false,
                    };
                    let voice_text = "That idempotency key was already used for a different request. Please generate a new key or reuse the original parameters.".to_string();
                    return Ok(PlaceOrderOutcome::IdempotencyConflict { output, voice_text });
                }
            }
        }

        // 3. Abuse prevention: max active orders per guest
        let guest_name_trimmed = req.guest_name.trim();
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM orders WHERE LOWER(guest_name) = LOWER(?) AND status IN ('placed', 'preparing')",
        )
        .bind(guest_name_trimmed)
        .fetch_one(&mut *conn)
        .await?;

        if active_count >= MAX_ACTIVE_ORDERS_PER_GUEST {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "limit_exceeded".to_string(),
                reason: Some("active_orders_limit".to_string()),
                reason_detail: Some(format!(
                    "Guest '{}' has {} active orders. Maximum allowed active orders is {}.",
                    guest_name_trimmed, active_count, MAX_ACTIVE_ORDERS_PER_GUEST
                )),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: None,
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = format!(
                "You already have {} active orders, which is our limit. Please wait until your current order is completed before placing another.",
                active_count
            );
            return Ok(PlaceOrderOutcome::LimitExceeded { output, voice_text });
        }

        // 4. Resolve items from DB
        let all_menu_items: Vec<MenuItem> = sqlx::query_as(
            r#"
            SELECT id, category_id, name, description, price_cents, currency,
                   is_available, prep_time_minutes, image_url, spice_level,
                   calories, is_chefs_special, allergens_verified
            FROM menu_items
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        let mut resolved_items: Vec<(&OrderLineInput, MenuItem)> = Vec::new();

        for line in req.items {
            if let Some(id) = line.item_id {
                if let Some(m) = all_menu_items.iter().find(|it| it.id == id) {
                    resolved_items.push((line, m.clone()));
                } else {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                    let output = PlaceOrderOutput {
                        status: "unknown_item".to_string(),
                        reason: Some("unknown_item".to_string()),
                        reason_detail: Some(format!("No menu item found with id {}.", id)),
                        order: None,
                        price_changes: vec![],
                        candidates: vec![],
                        alternatives: vec![],
                        earliest_pickup_local: None,
                        allergen_notice: ALLERGEN_NOTICE.to_string(),
                        idempotent_replay: false,
                    };
                    let voice_text = format!("I couldn't find an item with ID {}. Would you like me to search the menu for you?", id);
                    return Ok(PlaceOrderOutcome::UnknownItem { output, voice_text });
                }
            } else if let Some(name_query) = &line.item_name {
                let trimmed_query = name_query.trim().to_lowercase();
                let exact_matches: Vec<&MenuItem> = all_menu_items
                    .iter()
                    .filter(|it| it.name.trim().to_lowercase() == trimmed_query)
                    .collect();

                if exact_matches.len() == 1 {
                    resolved_items.push((line, (*exact_matches[0]).clone()));
                } else if exact_matches.len() > 1 {
                    // Ambiguous
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                    let candidates: Vec<ItemCandidate> = exact_matches
                        .into_iter()
                        .map(|it| ItemCandidate {
                            item_id: it.id,
                            name: it.name.clone(),
                            price: format_price_display(it.price_cents, &it.currency),
                        })
                        .collect();
                    let output = PlaceOrderOutput {
                        status: "ambiguous_item".to_string(),
                        reason: Some("ambiguous_item".to_string()),
                        reason_detail: Some(format!("Query '{}' matched multiple items.", name_query)),
                        order: None,
                        price_changes: vec![],
                        candidates,
                        alternatives: vec![],
                        earliest_pickup_local: None,
                        allergen_notice: ALLERGEN_NOTICE.to_string(),
                        idempotent_replay: false,
                    };
                    let voice_text = generate_ambiguous_item_voice_text(name_query, &output.candidates);
                    return Ok(PlaceOrderOutcome::AmbiguousItem { output, voice_text });
                } else {
                    // Substring match
                    let substr_matches: Vec<&MenuItem> = all_menu_items
                        .iter()
                        .filter(|it| it.name.to_lowercase().contains(&trimmed_query))
                        .collect();

                    if substr_matches.len() == 1 {
                        resolved_items.push((line, (*substr_matches[0]).clone()));
                    } else if substr_matches.len() > 1 {
                        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                        let candidates: Vec<ItemCandidate> = substr_matches
                            .into_iter()
                            .map(|it| ItemCandidate {
                                item_id: it.id,
                                name: it.name.clone(),
                                price: format_price_display(it.price_cents, &it.currency),
                            })
                            .collect();
                        let output = PlaceOrderOutput {
                            status: "ambiguous_item".to_string(),
                            reason: Some("ambiguous_item".to_string()),
                            reason_detail: Some(format!("Query '{}' matched multiple items.", name_query)),
                            order: None,
                            price_changes: vec![],
                            candidates,
                            alternatives: vec![],
                            earliest_pickup_local: None,
                            allergen_notice: ALLERGEN_NOTICE.to_string(),
                            idempotent_replay: false,
                        };
                        let voice_text = generate_ambiguous_item_voice_text(name_query, &output.candidates);
                        return Ok(PlaceOrderOutcome::AmbiguousItem { output, voice_text });
                    } else {
                        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                        let output = PlaceOrderOutput {
                            status: "unknown_item".to_string(),
                            reason: Some("unknown_item".to_string()),
                            reason_detail: Some(format!("No menu item matched '{}'.", name_query)),
                            order: None,
                            price_changes: vec![],
                            candidates: vec![],
                            alternatives: vec![],
                            earliest_pickup_local: None,
                            allergen_notice: ALLERGEN_NOTICE.to_string(),
                            idempotent_replay: false,
                        };
                        let voice_text = format!("I couldn't find '{}' on the menu. Would you like me to check what's available?", name_query);
                        return Ok(PlaceOrderOutcome::UnknownItem { output, voice_text });
                    }
                }
            }
        }

        // 5. Check item availability
        for (_line, item) in &resolved_items {
            if !item.is_available {
                sqlx::query("ROLLBACK").execute(&mut *conn).await?;
                let output = PlaceOrderOutput {
                    status: "item_unavailable".to_string(),
                    reason: Some("item_unavailable".to_string()),
                    reason_detail: Some(format!("'{}' is currently sold out or unavailable.", item.name)),
                    order: None,
                    price_changes: vec![],
                    candidates: vec![],
                    alternatives: vec![],
                    earliest_pickup_local: None,
                    allergen_notice: ALLERGEN_NOTICE.to_string(),
                    idempotent_replay: false,
                };
                let voice_text = format!("I'm sorry, but {} is currently sold out. Would you like to pick something else?", item.name);
                return Ok(PlaceOrderOutcome::ItemUnavailable { output, voice_text });
            }
        }

        // 6. Allergen safety evaluation
        let mut item_allergens_map: HashMap<i64, Vec<String>> = HashMap::new();
        let mut item_dietary_map: HashMap<i64, Vec<String>> = HashMap::new();

        for (_, item) in &resolved_items {
            let allergens: Vec<(String,)> = sqlx::query_as(
                r#"
                SELECT a.name
                FROM allergens a
                JOIN menu_item_allergens mia ON mia.allergen_id = a.id
                WHERE mia.menu_item_id = ?
                "#,
            )
            .bind(item.id)
            .fetch_all(&mut *conn)
            .await?;
            item_allergens_map.insert(item.id, allergens.into_iter().map(|(n,)| n).collect());

            let dietary: Vec<(String,)> = sqlx::query_as(
                r#"
                SELECT dt.name
                FROM dietary_tags dt
                JOIN menu_item_dietary_tags midt ON midt.tag_id = dt.id
                WHERE midt.menu_item_id = ?
                "#,
            )
            .bind(item.id)
            .fetch_all(&mut *conn)
            .await?;
            item_dietary_map.insert(item.id, dietary.into_iter().map(|(n,)| n).collect());
        }

        if let Some(ack) = req.allergen_ack {
            if !ack.allergens_to_avoid.is_empty() {
                let mut conflicting_lines: Vec<ConflictingLine> = Vec::new();
                let severe_allergens = [
                    AllergenFilter::Peanuts,
                    AllergenFilter::TreeNuts,
                    AllergenFilter::Shellfish,
                    AllergenFilter::Fish,
                ];

                for (line, item) in &resolved_items {
                    let item_allergens = item_allergens_map.get(&item.id).cloned().unwrap_or_default();
                    let unverified = !item.allergens_verified;

                    let mut matched_avoided: Vec<String> = Vec::new();
                    for avoid in &ack.allergens_to_avoid {
                        let avoid_str = avoid.as_db_str();
                        if item_allergens.iter().any(|a| a.eq_ignore_ascii_case(avoid_str)) {
                            matched_avoided.push(avoid_str.to_string());
                        }
                    }

                    let has_severe_avoidance = ack
                        .allergens_to_avoid
                        .iter()
                        .any(|a| severe_allergens.contains(a));

                    // Rule b: unverified allergen data for severe allergen category CANNOT be overridden by acknowledged: true
                    let cannot_override_unverified = unverified && has_severe_avoidance;

                    let is_conflict = if cannot_override_unverified {
                        true
                    } else if unverified || !matched_avoided.is_empty() {
                        !ack.acknowledged
                    } else {
                        false
                    };

                    if is_conflict {
                        conflicting_lines.push(ConflictingLine {
                            requested_name: line
                                .item_name
                                .clone()
                                .unwrap_or_else(|| item.name.clone()),
                            matched_item_id: item.id,
                            matched_item_name: item.name.clone(),
                            allergens_found: matched_avoided,
                            unverified_allergen_data: unverified,
                        });
                    }
                }

                if !conflicting_lines.is_empty() {
                    sqlx::query("ROLLBACK").execute(&mut *conn).await?;

                    // Propose alternatives from the same categories avoiding those allergens
                    let first_conflict = &conflicting_lines[0];
                    let avoid_strs: Vec<&'static str> = ack
                        .allergens_to_avoid
                        .iter()
                        .map(|a| a.as_db_str())
                        .collect();

                    let alternatives: Vec<String> = all_menu_items
                        .iter()
                        .filter(|it| {
                            it.is_available
                                && it.allergens_verified
                                && it.id != first_conflict.matched_item_id
                        })
                        .filter(|it| {
                            let allergens = item_allergens_map.get(&it.id);
                            if let Some(list) = allergens {
                                !list.iter().any(|a| avoid_strs.contains(&a.as_str()))
                            } else {
                                true
                            }
                        })
                        .take(3)
                        .map(|it| it.name.clone())
                        .collect();

                    let output = PlaceOrderOutput {
                        status: "allergen_conflict".to_string(),
                        reason: Some("allergen_conflict".to_string()),
                        reason_detail: Some(format!(
                            "Order contains items conflicting with requested allergen restrictions: {:?}",
                            conflicting_lines
                                .iter()
                                .map(|c| &c.matched_item_name)
                                .collect::<Vec<_>>()
                        )),
                        order: None,
                        price_changes: vec![],
                        candidates: vec![],
                        alternatives: alternatives.clone(),
                        earliest_pickup_local: None,
                        allergen_notice: ALLERGEN_NOTICE.to_string(),
                        idempotent_replay: false,
                    };
                    let voice_text = generate_allergen_conflict_voice_text(&conflicting_lines, &alternatives);
                    return Ok(PlaceOrderOutcome::AllergenConflict { output, voice_text });
                }
            }
        }

        // 7. Pricing calculations (checked integer arithmetic)
        let mut subtotal_cents = 0i64;
        let mut line_summaries: Vec<OrderLineSummary> = Vec::new();

        for (line, item) in &resolved_items {
            let unit_price = item.price_cents;
            let line_total = unit_price
                .checked_mul(line.quantity)
                .ok_or_else(|| sqlx::Error::Protocol("Arithmetic overflow calculating line total".to_string()))?;

            subtotal_cents = subtotal_cents
                .checked_add(line_total)
                .ok_or_else(|| sqlx::Error::Protocol("Arithmetic overflow calculating subtotal".to_string()))?;

            line_summaries.push(OrderLineSummary {
                item_id: item.id,
                name: item.name.clone(),
                quantity: line.quantity,
                unit_price: format_price_display(unit_price, &item.currency),
                line_total: format_price_display(line_total, &item.currency),
                notes: line.notes.clone(),
                allergens: item_allergens_map.get(&item.id).cloned().unwrap_or_default(),
                dietary_tags: item_dietary_map.get(&item.id).cloned().unwrap_or_default(),
            });
        }

        if subtotal_cents > MAX_ORDER_TOTAL_CENTS {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "limit_exceeded".to_string(),
                reason: Some("max_total_exceeded".to_string()),
                reason_detail: Some(format!(
                    "Order subtotal of ₹{} exceeds the maximum permitted total of ₹{}.",
                    subtotal_cents / 100,
                    MAX_ORDER_TOTAL_CENTS / 100
                )),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: None,
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = format!(
                "This order total exceeds our maximum limit of ₹{}. Please contact the restaurant directly for large catering orders.",
                MAX_ORDER_TOTAL_CENTS / 100
            );
            return Ok(PlaceOrderOutcome::LimitExceeded { output, voice_text });
        }

        let tax_cents = if restaurant.tax_rate_basis_points > 0 {
            let numerator = subtotal_cents
                .checked_mul(restaurant.tax_rate_basis_points)
                .ok_or_else(|| sqlx::Error::Protocol("Overflow in tax calculation".to_string()))?
                + 5000;
            numerator / 10000
        } else {
            0i64
        };

        let total_cents = subtotal_cents
            .checked_add(tax_cents)
            .ok_or_else(|| sqlx::Error::Protocol("Overflow in total calculation".to_string()))?;

        // 8. Kitchen capacity and earliest feasible pickup time
        let max_prep_minutes = resolved_items
            .iter()
            .map(|(_, item)| item.prep_time_minutes)
            .max()
            .unwrap_or(15);

        let active_orders_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM orders WHERE status IN ('placed', 'preparing')",
        )
        .fetch_one(&mut *conn)
        .await?;

        let queue_allowance_minutes = active_orders_total * 3;
        let total_prep_minutes = max_prep_minutes + queue_allowance_minutes;

        let earliest_feasible_naive = now_local.naive_local() + Duration::minutes(total_prep_minutes);
        let snapped_earliest_naive = snap_pickup_time_up(
            earliest_feasible_naive,
            restaurant.slot_granularity_minutes,
        );

        let earliest_feasible_local = rest_tz
            .from_local_datetime(&snapped_earliest_naive)
            .unwrap();

        // 9. Operating hours and pickup time validation
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

        let target_pickup_naive = if let Some(pt_str) = req.pickup_time {
            parse_pickup_time_str(pt_str, now_local.date_naive())
                .map_err(|e| sqlx::Error::Protocol(e.to_string()))?
        } else {
            snapped_earliest_naive
        };

        let target_pickup_local = rest_tz
            .from_local_datetime(&target_pickup_naive)
            .unwrap();

        // Validate against operating hours first
        let day_of_week = target_pickup_local.weekday().num_days_from_sunday() as i64;
        let today_closures: Vec<&SpecialClosure> = special_closures
            .iter()
            .filter(|sc| sc.closure_date == target_pickup_local.date_naive().to_string())
            .collect();

        if today_closures.iter().any(|sc| sc.is_all_day) {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "closed".to_string(),
                reason: Some("special_closure".to_string()),
                reason_detail: Some("Restaurant is closed today for a special event or maintenance.".to_string()),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: None,
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = "The restaurant is closed today for a special event. We are not accepting pickup orders.".to_string();
            return Ok(PlaceOrderOutcome::Closed { output, voice_text });
        }

        let shifts: Vec<&OpeningHours> = opening_hours
            .iter()
            .filter(|oh| oh.day_of_week == day_of_week && !oh.is_closed)
            .collect();

        if shifts.is_empty() {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "closed".to_string(),
                reason: Some("closed_day".to_string()),
                reason_detail: Some("The restaurant is closed on this day of the week.".to_string()),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: None,
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = "The restaurant is closed today. Please check back on our next open day.".to_string();
            return Ok(PlaceOrderOutcome::Closed { output, voice_text });
        }

        let pickup_time_only = target_pickup_local.time();
        let buffer_mins = restaurant.last_order_before_close_minutes;

        let in_service = shifts.iter().any(|shift| {
            if let (Ok(open_t), Ok(close_t)) = (
                NaiveTime::parse_from_str(&shift.open_time, "%H:%M"),
                NaiveTime::parse_from_str(&shift.close_time, "%H:%M"),
            ) {
                let last_order_cutoff = close_t - Duration::minutes(buffer_mins);
                pickup_time_only >= open_t && pickup_time_only <= last_order_cutoff
            } else {
                false
            }
        });

        if !in_service {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "closed".to_string(),
                reason: Some("outside_hours".to_string()),
                reason_detail: Some(format!(
                    "Pickup time {:02}:{:02} is outside open kitchen service hours or past the {}-minute last-order cutoff.",
                    pickup_time_only.hour(), pickup_time_only.minute(), buffer_mins
                )),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: None,
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = format!(
                "The kitchen is not accepting pickup orders at {}. Please select a time during our regular service hours.",
                format_spoken_time(pickup_time_only)
            );
            return Ok(PlaceOrderOutcome::Closed { output, voice_text });
        }

        // Validate target against horizon: pickup orders must be on restaurant's current local day
        if target_pickup_local.date_naive() != now_local.date_naive() {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "invalid_request".to_string(),
                reason: Some("beyond_order_horizon".to_string()),
                reason_detail: Some("Pickup orders can only be scheduled for today.".to_string()),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: Some(earliest_feasible_local.to_rfc3339()),
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = "Pickup orders must be placed for today. Would you like to set your pickup for later today?".to_string();
            return Ok(PlaceOrderOutcome::PickupTooSoon { output, voice_text });
        }

        // Validate pickup time is not before earliest feasible
        if target_pickup_local < earliest_feasible_local {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = PlaceOrderOutput {
                status: "pickup_too_soon".to_string(),
                reason: Some("pickup_too_soon".to_string()),
                reason_detail: Some(format!(
                    "Requested pickup time {:02}:{:02} is before the earliest feasible preparation time {:02}:{:02}.",
                    target_pickup_local.hour(), target_pickup_local.minute(),
                    earliest_feasible_local.hour(), earliest_feasible_local.minute()
                )),
                order: None,
                price_changes: vec![],
                candidates: vec![],
                alternatives: vec![],
                earliest_pickup_local: Some(earliest_feasible_local.to_rfc3339()),
                allergen_notice: ALLERGEN_NOTICE.to_string(),
                idempotent_replay: false,
            };
            let voice_text = format!(
                "The kitchen needs about {} minutes to prepare your order. The earliest pickup time is {}. Would that work for you?",
                total_prep_minutes,
                format_spoken_time(earliest_feasible_local.time())
            );
            return Ok(PlaceOrderOutcome::PickupTooSoon { output, voice_text });
        }

        // 10. Generate order code with collision retry
        let mut order_code = generate_order_code();
        for _ in 0..5 {
            let collision: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM orders WHERE order_code = ?",
            )
            .bind(&order_code)
            .fetch_one(&mut *conn)
            .await?;
            if collision == 0 {
                break;
            }
            order_code = generate_order_code();
        }

        let order_id = uuid::Uuid::new_v4().to_string();
        let pickup_utc_iso = target_pickup_local.with_timezone(&Utc).to_rfc3339();

        let stated_allergens_str = req.allergen_ack.map(|ack| {
            ack.allergens_to_avoid
                .iter()
                .map(|a| a.as_db_str())
                .collect::<Vec<_>>()
                .join(", ")
        });

        // 11. Insert order record
        sqlx::query(
            r#"
            INSERT INTO orders (
                id, order_code, customer_id, booking_id, order_type, status,
                pickup_time, subtotal_cents, tax_cents, tip_cents, total_cents,
                currency, notes, guest_name, guest_phone, guest_email,
                stated_allergens, created_at, updated_at
            )
            VALUES (?, ?, NULL, NULL, 'pickup', 'placed', ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&order_id)
        .bind(&order_code)
        .bind(&pickup_utc_iso)
        .bind(subtotal_cents)
        .bind(tax_cents)
        .bind(total_cents)
        .bind(&restaurant.currency)
        .bind(req.order_notes)
        .bind(guest_name_trimmed)
        .bind(req.phone)
        .bind(req.email)
        .bind(&stated_allergens_str)
        .bind(&now_utc_iso)
        .bind(&now_utc_iso)
        .execute(&mut *conn)
        .await?;

        // 12. Insert order items
        for (line, item) in &resolved_items {
            sqlx::query(
                r#"
                INSERT INTO order_items (
                    order_id, menu_item_id, item_name, quantity,
                    unit_price_cents, currency, notes
                )
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&order_id)
            .bind(item.id)
            .bind(&item.name)
            .bind(line.quantity)
            .bind(item.price_cents)
            .bind(&item.currency)
            .bind(&line.notes)
            .execute(&mut *conn)
            .await?;
        }

        // 13. Insert audit event (never log PII such as guest_name, notes, allergens)
        let audit_payload = serde_json::json!({
            "order_code_prefix": &order_code[..order_code.len().min(6)],
            "total_cents": total_cents,
            "items_count": resolved_items.len(),
            "pickup_time": target_pickup_local.to_rfc3339(),
        });

        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('order', ?, 'placed', 'mcp', ?, ?)
            "#,
        )
        .bind(&order_id)
        .bind(audit_payload.to_string())
        .bind(&now_utc_iso)
        .execute(&mut *conn)
        .await?;

        // 14. Construct success output
        let order_summary = OrderSummary {
            order_code: order_code.clone(),
            guest_name: guest_name_trimmed.to_string(),
            status: "placed".to_string(),
            pickup_time_local: target_pickup_local.to_rfc3339(),
            lines: line_summaries,
            subtotal: format_price_display(subtotal_cents, &restaurant.currency),
            tax: format_price_display(tax_cents, &restaurant.currency),
            total: format_price_display(total_cents, &restaurant.currency),
            payment_status: "unpaid_pay_at_pickup".to_string(),
            stated_allergens: stated_allergens_str,
            created_at: now_utc_iso.clone(),
        };

        let output = PlaceOrderOutput {
            status: "placed".to_string(),
            reason: None,
            reason_detail: None,
            order: Some(order_summary),
            price_changes: vec![],
            candidates: vec![],
            alternatives: vec![],
            earliest_pickup_local: Some(earliest_feasible_local.to_rfc3339()),
            allergen_notice: ALLERGEN_NOTICE.to_string(),
            idempotent_replay: false,
        };

        // 15. Record idempotency key if provided
        if let Some(key) = req.idempotency_key {
            let key_id = uuid::Uuid::new_v4().to_string();
            let expires_at_iso = (now + Duration::hours(IDEMPOTENCY_KEY_EXPIRY_HOURS)).to_rfc3339();
            let response_json = serde_json::to_string(&output).unwrap_or_default();

            sqlx::query(
                r#"
                INSERT INTO idempotency_keys (
                    id, key, tool_name, request_hash, response_json,
                    booking_id, order_id, created_at, expires_at
                )
                VALUES (?, ?, 'place_order', ?, ?, NULL, ?, ?, ?)
                "#,
            )
            .bind(&key_id)
            .bind(key)
            .bind(&request_hash)
            .bind(&response_json)
            .bind(&order_id)
            .bind(&now_utc_iso)
            .bind(&expires_at_iso)
            .execute(&mut *conn)
            .await?;
        }

        sqlx::query("COMMIT").execute(&mut *conn).await?;
        let voice_text = generate_place_order_voice_text(&output);
        Ok(PlaceOrderOutcome::Placed { output, voice_text })
    }
    .await;

    match res {
        Ok(outcome) => Ok(outcome),
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Executes cancellation of an order inside `BEGIN IMMEDIATE`.
pub async fn cancel_order_tx(
    pool: &SqlitePool,
    req: CancelOrderTxRequest<'_>,
    rate_limiter: &OrderCancellationRateLimiter,
    clock: &dyn Clock,
) -> Result<CancelOrderOutcome, sqlx::Error> {
    let now = clock.now();
    let now_utc_iso = now.to_rfc3339();
    let normalized_code = normalize_order_code(req.order_code);

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    // Rate limiter check
    if !rate_limiter.check_and_record(&normalized_code, now) {
        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
        let output = CancelOrderOutput {
            status: "limit_exceeded".to_string(),
            already_cancelled: false,
            order: None,
            policy_note: None,
            reason: Some("rate_limit_exceeded".to_string()),
            reason_detail: Some("Too many failed cancellation attempts. Please wait 15 minutes before trying again.".to_string()),
        };
        let voice_text = "Too many cancellation attempts were made for this order. Please wait fifteen minutes before trying again.".to_string();
        return Ok(CancelOrderOutcome::RateLimited { output, voice_text });
    }

    let existing: Option<Order> = sqlx::query_as(
        r#"
        SELECT id, order_code, customer_id, booking_id, order_type, status, pickup_time,
               subtotal_cents, tax_cents, tip_cents, total_cents, currency,
               notes, guest_name, guest_phone, guest_email, stated_allergens,
               created_at, updated_at, cancelled_at
        FROM orders
        WHERE order_code = ?
        "#,
    )
    .bind(&normalized_code)
    .fetch_optional(&mut *conn)
    .await?;

    let order = match existing {
        Some(o) => o,
        None => {
            sqlx::query("ROLLBACK").execute(&mut *conn).await?;
            let output = generic_not_found_cancel_output();
            let voice_text = generic_not_found_voice_text();
            return Ok(CancelOrderOutcome::NotFound { output, voice_text });
        }
    };

    // Guest name verification (case-insensitive, trimmed)
    if !order
        .guest_name
        .trim()
        .eq_ignore_ascii_case(req.guest_name.trim())
    {
        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
        let output = generic_not_found_cancel_output();
        let voice_text = generic_not_found_voice_text();
        return Ok(CancelOrderOutcome::NotFound { output, voice_text });
    }

    // Check status
    if order.status == OrderStatus::Cancelled {
        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
        let summary = build_order_summary(&order, &mut conn).await?;
        let output = CancelOrderOutput {
            status: "cancelled".to_string(),
            already_cancelled: true,
            order: Some(summary),
            policy_note: None,
            reason: None,
            reason_detail: Some("This order was already cancelled.".to_string()),
        };
        let voice_text = format!(
            "This order under code {} was already cancelled. No further action is needed.",
            format_spoken_order_code(&order.order_code)
        );
        return Ok(CancelOrderOutcome::AlreadyCancelled { output, voice_text });
    }

    if order.status != OrderStatus::Placed {
        sqlx::query("ROLLBACK").execute(&mut *conn).await?;
        let summary = build_order_summary(&order, &mut conn).await?;
        let output = CancelOrderOutput {
            status: "not_cancellable".to_string(),
            already_cancelled: false,
            order: Some(summary),
            policy_note: Some("Orders already in kitchen preparation cannot be cancelled via voice assistant.".to_string()),
            reason: Some("order_already_in_preparation".to_string()),
            reason_detail: Some("The kitchen has already started preparing this order. Please call the restaurant directly at +91 80 4567 8900 for assistance.".to_string()),
        };
        let voice_text = "Your order is already being prepared by the kitchen and cannot be cancelled online. Please call the restaurant directly for assistance.".to_string();
        return Ok(CancelOrderOutcome::NotCancellable { output, voice_text });
    }

    // Update status to Cancelled
    sqlx::query(
        "UPDATE orders SET status = 'cancelled', cancelled_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now_utc_iso)
    .bind(&now_utc_iso)
    .bind(&order.id)
    .execute(&mut *conn)
    .await?;

    // Audit event (sanitized, no PII)
    let audit_payload = serde_json::json!({
        "order_code_prefix": &order.order_code[..order.order_code.len().min(6)],
        "action": "cancelled",
    });
    sqlx::query(
        r#"
        INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
        VALUES ('order', ?, 'cancelled', 'mcp', ?, ?)
        "#,
    )
    .bind(&order.id)
    .bind(audit_payload.to_string())
    .bind(&now_utc_iso)
    .execute(&mut *conn)
    .await?;

    sqlx::query("COMMIT").execute(&mut *conn).await?;

    let mut cancelled_order = order;
    cancelled_order.status = OrderStatus::Cancelled;
    cancelled_order.cancelled_at = Some(now_utc_iso);

    let summary = build_order_summary(&cancelled_order, &mut conn).await?;
    let output = CancelOrderOutput {
        status: "cancelled".to_string(),
        already_cancelled: false,
        order: Some(summary),
        policy_note: Some("Order cancelled successfully prior to preparation.".to_string()),
        reason: None,
        reason_detail: None,
    };
    let voice_text = format!(
        "Your pickup order under code {} has been cancelled. No charge was incurred.",
        format_spoken_order_code(&cancelled_order.order_code)
    );

    Ok(CancelOrderOutcome::Cancelled { output, voice_text })
}

/// Retrieves an order summary with name verification.
pub async fn get_order_tx(
    pool: &SqlitePool,
    order_code_input: &str,
    guest_name_input: &str,
) -> Result<GetOrderOutcome, sqlx::Error> {
    let normalized_code = normalize_order_code(order_code_input);
    let mut conn = pool.acquire().await?;

    let existing: Option<Order> = sqlx::query_as(
        r#"
        SELECT id, order_code, customer_id, booking_id, order_type, status, pickup_time,
               subtotal_cents, tax_cents, tip_cents, total_cents, currency,
               notes, guest_name, guest_phone, guest_email, stated_allergens,
               created_at, updated_at, cancelled_at
        FROM orders
        WHERE order_code = ?
        "#,
    )
    .bind(&normalized_code)
    .fetch_optional(&mut *conn)
    .await?;

    let order = match existing {
        Some(o) => o,
        None => {
            let output = GetOrderOutput {
                status: "not_found".to_string(),
                order: None,
                message: "No matching order found for the provided order code and guest name."
                    .to_string(),
            };
            let voice_text = generic_not_found_voice_text();
            return Ok(GetOrderOutcome::NotFound { output, voice_text });
        }
    };

    if !order
        .guest_name
        .trim()
        .eq_ignore_ascii_case(guest_name_input.trim())
    {
        let output = GetOrderOutput {
            status: "not_found".to_string(),
            order: None,
            message: "No matching order found for the provided order code and guest name."
                .to_string(),
        };
        let voice_text = generic_not_found_voice_text();
        return Ok(GetOrderOutcome::NotFound { output, voice_text });
    }

    let summary = build_order_summary(&order, &mut conn).await?;
    let voice_text = format!(
        "I found your pickup order under code {}. Status is {}. Total is {}.",
        format_spoken_order_code(&order.order_code),
        order.status.as_str(),
        paise_to_spoken_rupees(order.total_cents)
    );
    let output = GetOrderOutput {
        status: "found".to_string(),
        order: Some(summary),
        message: "Order retrieved successfully.".to_string(),
    };

    Ok(GetOrderOutcome::Found { output, voice_text })
}

// --- Helper Functions ---

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Draft => "draft",
            OrderStatus::Placed => "placed",
            OrderStatus::Preparing => "preparing",
            OrderStatus::Ready => "ready",
            OrderStatus::Completed => "completed",
            OrderStatus::Cancelled => "cancelled",
        }
    }
}

pub fn generic_not_found_cancel_output() -> CancelOrderOutput {
    CancelOrderOutput {
        status: "not_found".to_string(),
        already_cancelled: false,
        order: None,
        policy_note: None,
        reason: Some("not_found".to_string()),
        reason_detail: Some(
            "No matching order found for the provided order code and guest name. Please check your order details.".to_string(),
        ),
    }
}

pub fn generic_not_found_voice_text() -> String {
    "I couldn't find an order with that code and name. Please check the code and the name on the order.".to_string()
}

pub async fn build_order_summary(
    order: &Order,
    conn: &mut sqlx::pool::PoolConnection<sqlx::Sqlite>,
) -> Result<OrderSummary, sqlx::Error> {
    let items: Vec<OrderItem> = sqlx::query_as(
        r#"
        SELECT id, order_id, menu_item_id, item_name, quantity, unit_price_cents, currency, notes
        FROM order_items
        WHERE order_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(&order.id)
    .fetch_all(&mut **conn)
    .await?;

    let mut line_summaries = Vec::new();
    for it in items {
        let allergens: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT a.name
            FROM allergens a
            JOIN menu_item_allergens mia ON mia.allergen_id = a.id
            WHERE mia.menu_item_id = ?
            "#,
        )
        .bind(it.menu_item_id)
        .fetch_all(&mut **conn)
        .await?;

        let dietary: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT dt.name
            FROM dietary_tags dt
            JOIN menu_item_dietary_tags midt ON midt.tag_id = dt.id
            WHERE midt.menu_item_id = ?
            "#,
        )
        .bind(it.menu_item_id)
        .fetch_all(&mut **conn)
        .await?;

        let line_total = it.unit_price_cents * it.quantity;
        line_summaries.push(OrderLineSummary {
            item_id: it.menu_item_id,
            name: it.item_name,
            quantity: it.quantity,
            unit_price: format_price_display(it.unit_price_cents, &it.currency),
            line_total: format_price_display(line_total, &it.currency),
            notes: it.notes,
            allergens: allergens.into_iter().map(|(n,)| n).collect(),
            dietary_tags: dietary.into_iter().map(|(n,)| n).collect(),
        });
    }

    Ok(OrderSummary {
        order_code: order.order_code.clone(),
        guest_name: order.guest_name.clone(),
        status: order.status.as_str().to_string(),
        pickup_time_local: order.pickup_time.clone().unwrap_or_default(),
        lines: line_summaries,
        subtotal: format_price_display(order.subtotal_cents, &order.currency),
        tax: format_price_display(order.tax_cents, &order.currency),
        total: format_price_display(order.total_cents, &order.currency),
        payment_status: "unpaid_pay_at_pickup".to_string(),
        stated_allergens: order.stated_allergens.clone(),
        created_at: order.created_at.clone(),
    })
}

fn snap_pickup_time_up(dt: NaiveDateTime, granularity_minutes: i64) -> NaiveDateTime {
    let minute = dt.minute() as i64;
    let rem = minute % granularity_minutes;
    if rem == 0 && dt.second() == 0 {
        dt
    } else {
        let add = granularity_minutes - rem;
        let base = dt.date().and_hms_opt(dt.hour(), dt.minute(), 0).unwrap();
        base + Duration::minutes(add)
    }
}

fn parse_pickup_time_str(
    input: &str,
    today_date: NaiveDate,
) -> Result<NaiveDateTime, OrderValidationError> {
    let clean = input.trim();
    if clean.len() == 5 && clean.contains(':') {
        let time = NaiveTime::parse_from_str(clean, "%H:%M")
            .map_err(|_| OrderValidationError::InvalidPickupTime(clean.to_string()))?;
        Ok(today_date.and_time(time))
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S") {
        Ok(dt)
    } else if let Ok(dt) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M") {
        Ok(dt)
    } else {
        Err(OrderValidationError::InvalidPickupTime(clean.to_string()))
    }
}

// --- Voice Text Generators ---

pub fn generate_place_order_voice_text(out: &PlaceOrderOutput) -> String {
    if let Some(order) = &out.order {
        let total_spoken = paise_to_spoken_rupees(order.total.amount_minor);
        let count = order.lines.iter().map(|l| l.quantity).sum::<i64>();
        let count_str = if count == 1 {
            "1 item".to_string()
        } else {
            format!("{} items", count)
        };

        let pickup_time_spoken =
            if let Ok(dt) = DateTime::parse_from_rfc3339(&order.pickup_time_local) {
                format_spoken_time(dt.time())
            } else {
                "the scheduled time".to_string()
            };

        let code_spoken = format_spoken_order_code(&order.order_code);

        let allergen_part = if let Some(stated) = &order.stated_allergens {
            format!(
                " Items are listed as not containing {}. Please confirm severe allergies with staff at pickup.",
                stated
            )
        } else {
            " Please let us know if you have any allergies.".to_string()
        };

        format!(
            "I've placed your pickup order for {} totaling {}, ready at {}. Your order code is {}. Pay at pickup.{}",
            count_str, total_spoken, pickup_time_spoken, code_spoken, allergen_part
        )
    } else {
        "Your order has been placed.".to_string()
    }
}

pub fn generate_ambiguous_item_voice_text(query: &str, candidates: &[ItemCandidate]) -> String {
    let names: Vec<String> = candidates.iter().map(|c| c.name.clone()).take(3).collect();
    if names.len() >= 2 {
        format!(
            "I found multiple dishes matching {}. Did you mean the {} or the {}?",
            query, names[0], names[1]
        )
    } else {
        format!("Did you mean {}?", names.join(" or "))
    }
}

pub fn generate_allergen_conflict_voice_text(
    conflicts: &[ConflictingLine],
    alternatives: &[String],
) -> String {
    let first = &conflicts[0];
    let alt_text = if !alternatives.is_empty() {
        format!(" Would you like to try the {} instead?", alternatives[0])
    } else {
        " Would you like to choose a different dish?".to_string()
    };

    if first.unverified_allergen_data {
        format!(
            "Allergen information for {} is unverified and cannot be confirmed without restaurant staff.{}",
            first.matched_item_name, alt_text
        )
    } else {
        let allergens = first.allergens_found.join(" and ");
        format!(
            "The {} is listed as containing {}, which you asked to avoid.{}",
            first.matched_item_name, allergens, alt_text
        )
    }
}

// --- Unit Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paise_to_spoken_rupees() {
        assert_eq!(paise_to_spoken_rupees(0), "zero rupees");
        assert_eq!(paise_to_spoken_rupees(100), "one rupee");
        assert_eq!(paise_to_spoken_rupees(200), "two rupees");
        assert_eq!(paise_to_spoken_rupees(32000), "three hundred twenty rupees");
        assert_eq!(paise_to_spoken_rupees(38000), "three hundred eighty rupees");
        assert_eq!(
            paise_to_spoken_rupees(1550),
            "fifteen rupees and fifty paise"
        );
        assert_eq!(paise_to_spoken_rupees(50), "fifty paise");
        assert_eq!(paise_to_spoken_rupees(101), "one rupee and one paisa");
        assert_eq!(paise_to_spoken_rupees(100500), "one thousand five rupees");
    }

    #[test]
    fn test_state_machine_allowed_and_illegal() {
        // Allowed
        assert!(can_transition(OrderStatus::Draft, OrderStatus::Placed));
        assert!(can_transition(OrderStatus::Draft, OrderStatus::Cancelled));
        assert!(can_transition(OrderStatus::Placed, OrderStatus::Preparing));
        assert!(can_transition(OrderStatus::Placed, OrderStatus::Cancelled));
        assert!(can_transition(OrderStatus::Preparing, OrderStatus::Ready));
        assert!(can_transition(OrderStatus::Ready, OrderStatus::Completed));

        // Illegal
        assert!(!can_transition(OrderStatus::Cancelled, OrderStatus::Placed));
        assert!(!can_transition(
            OrderStatus::Completed,
            OrderStatus::Cancelled
        ));
        assert!(!can_transition(OrderStatus::Cancelled, OrderStatus::Draft));
        assert!(!can_transition(OrderStatus::Placed, OrderStatus::Completed));
    }

    #[test]
    fn test_order_code_generation_and_normalization() {
        let code = generate_order_code();
        assert!(code.starts_with("TO-"));
        assert_eq!(code.len(), 8);

        // Safe alphabet
        for c in code[3..].chars() {
            assert!(ORDER_CODE_ALPHABET.contains(&(c as u8)));
            assert_ne!(c, '0');
            assert_ne!(c, 'O');
            assert_ne!(c, '1');
            assert_ne!(c, 'I');
            assert_ne!(c, 'L');
        }

        // Normalization
        assert_eq!(normalize_order_code("to 8k4mz"), "TO-8K4MZ");
        assert_eq!(normalize_order_code("8k4mz"), "TO-8K4MZ");
        assert_eq!(normalize_order_code("TO-8K4MZ"), "TO-8K4MZ");
        assert_eq!(normalize_order_code("  to-8k4mz  "), "TO-8K4MZ");
    }

    #[test]
    fn test_spoken_order_code_formatting() {
        assert_eq!(format_spoken_order_code("TO-8K4MZ"), "T O 8 K 4 M Z");
    }

    #[test]
    fn test_validation_rules() {
        let valid_items = vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 2,
            notes: Some("Extra hot".to_string()),
        }];

        // Valid
        assert!(validate_order_request(
            &valid_items,
            "Rohan Sharma",
            Some("+91 98765 43210"),
            Some("rohan@example.com"),
            None,
            Some("idem-key-1234"),
        )
        .is_ok());

        // Empty items
        assert!(matches!(
            validate_order_request(&[], "Rohan", None, None, None, None),
            Err(OrderValidationError::InvalidLineCount(0))
        ));

        // Quantity 0
        let bad_qty = vec![OrderLineInput {
            item_id: Some(1),
            item_name: None,
            quantity: 0,
            notes: None,
        }];
        assert!(matches!(
            validate_order_request(&bad_qty, "Rohan", None, None, None, None),
            Err(OrderValidationError::InvalidLineQuantity(0))
        ));

        // Missing identifier
        let no_id = vec![OrderLineInput {
            item_id: None,
            item_name: None,
            quantity: 1,
            notes: None,
        }];
        assert!(matches!(
            validate_order_request(&no_id, "Rohan", None, None, None, None),
            Err(OrderValidationError::MissingItemIdentifier)
        ));

        // Invalid guest name
        assert!(matches!(
            validate_order_request(&valid_items, "   ", None, None, None, None),
            Err(OrderValidationError::InvalidGuestName)
        ));

        // Invalid idempotency key
        assert!(matches!(
            validate_order_request(&valid_items, "Rohan", None, None, None, Some("short")),
            Err(OrderValidationError::InvalidIdempotencyKey(_))
        ));
    }

    #[test]
    fn test_rate_limiter_allows_up_to_5() {
        let limiter = OrderCancellationRateLimiter::new();
        let now = Utc::now();

        for _ in 0..5 {
            assert!(limiter.check_and_record("TO-ABCDE", now));
        }
        // 6th attempt within window should fail
        assert!(!limiter.check_and_record("TO-ABCDE", now));
    }
}
