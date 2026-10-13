use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::{collections::HashMap, sync::Mutex};

pub use crate::db::models::PreferenceKind;
use crate::{
    availability::Clock,
    db::models::Customer,
    menu_search::{PriceDisplay, ALLERGEN_NOTICE},
    ordering::paise_to_spoken_rupees,
};

pub const DEFAULT_RETENTION_DAYS: i64 = 365;
pub const ALLERGY_RECONFIRMATION_DAYS: i64 = 180;
pub const MAX_NOTE_LENGTH: usize = 140;
pub const MAX_NOTES_PER_CUSTOMER: usize = 20;

// --- Token Rate Limiter ---

pub struct ProfileTokenRateLimiter {
    attempts: Mutex<HashMap<String, Vec<DateTime<Utc>>>>,
    max_attempts: usize,
    window: Duration,
}

impl Default for ProfileTokenRateLimiter {
    fn default() -> Self {
        Self::new(5, Duration::minutes(15))
    }
}

impl ProfileTokenRateLimiter {
    pub fn new(max_attempts: usize, window: Duration) -> Self {
        Self {
            attempts: Mutex::new(HashMap::new()),
            max_attempts,
            window,
        }
    }

    /// Records a failed attempt and returns false if rate limit is exceeded.
    pub fn record_failure(&self, key: &str, now: DateTime<Utc>) -> bool {
        let mut map = self.attempts.lock().unwrap();
        let list = map.entry(key.to_string()).or_default();
        let cutoff = now - self.window;
        list.retain(|t| *t > cutoff);
        if list.len() >= self.max_attempts {
            false
        } else {
            list.push(now);
            true
        }
    }

    /// Checks if a key is currently rate-limited.
    pub fn is_rate_limited(&self, key: &str, now: DateTime<Utc>) -> bool {
        let mut map = self.attempts.lock().unwrap();
        if let Some(list) = map.get_mut(key) {
            let cutoff = now - self.window;
            list.retain(|t| *t > cutoff);
            list.len() >= self.max_attempts
        } else {
            false
        }
    }

    /// Resets attempts for a key (e.g. upon successful authentication).
    pub fn reset(&self, key: &str) {
        let mut map = self.attempts.lock().unwrap();
        map.remove(key);
    }
}

// Global rate limiter instance
static GLOBAL_TOKEN_RATE_LIMITER: std::sync::OnceLock<ProfileTokenRateLimiter> =
    std::sync::OnceLock::new();

pub fn get_token_rate_limiter() -> &'static ProfileTokenRateLimiter {
    GLOBAL_TOKEN_RATE_LIMITER.get_or_init(ProfileTokenRateLimiter::default)
}

// --- Crypto & Token Helpers ---

/// Generates a cryptographically secure 256-bit bearer profile token.
/// Format: `tt_prof_` followed by URL-safe base64 string.
pub fn generate_profile_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let b64 = base64_url_encode(&bytes);
    format!("tt_prof_{}", b64)
}

/// Computes SHA-256 hash of a profile token for database storage and constant-time lookup.
pub fn hash_profile_token(token: &str) -> String {
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

/// Simple URL-safe base64 encoder without padding (RFC 4648 §5).
fn base64_url_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::with_capacity((input.len() * 4).div_ceil(3));
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as usize
        } else {
            0
        };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[(triple >> 18) & 0x3F] as char);
        result.push(CHARSET[(triple >> 12) & 0x3F] as char);

        if i + 1 < input.len() {
            result.push(CHARSET[(triple >> 6) & 0x3F] as char);
        }
        if i + 2 < input.len() {
            result.push(CHARSET[triple & 0x3F] as char);
        }

        i += 3;
    }
    result
}

// --- Masking & Text Sanitization ---

pub fn mask_phone(phone: Option<&str>) -> Option<String> {
    phone.map(|p| {
        let p_trimmed = p.trim();
        if p_trimmed.len() <= 4 {
            "****".to_string()
        } else {
            let last4 = &p_trimmed[p_trimmed.len() - 4..];
            let prefix = if p_trimmed.starts_with('+') {
                let end = p_trimmed.find(' ').unwrap_or(3);
                &p_trimmed[..end]
            } else {
                ""
            };
            if !prefix.is_empty() {
                format!("{} ****{}", prefix, last4)
            } else {
                format!("****{}", last4)
            }
        }
    })
}

pub fn mask_email(email: Option<&str>) -> Option<String> {
    email.map(|e| {
        let e_trimmed = e.trim();
        if let Some((user, domain)) = e_trimmed.split_once('@') {
            if user.len() <= 2 {
                format!("*@{}", domain)
            } else {
                let first = &user[0..1];
                let last = &user[user.len() - 1..];
                format!("{}***{}@{}", first, last, domain)
            }
        } else {
            "***@example.com".to_string()
        }
    })
}

/// Sanitizes user-provided note: strips control characters, caps length at 140 chars.
pub fn sanitize_note(note: &str) -> String {
    let clean: String = note
        .chars()
        .filter(|c| !c.is_control() || *c == ' ' || *c == '\t')
        .take(MAX_NOTE_LENGTH)
        .collect();
    clean.trim().to_string()
}

// --- Data Types for Memory MCP Tools ---

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InitialPreferenceInput {
    pub kind: String, // "allergy" | "dietary" | "seat_pref" | "favorite_item" | "note"
    pub value: String,
    pub severity: Option<String>, // "mild" | "severe" | "unknown"
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StoredPreferenceView {
    pub id: i64,
    pub kind: String,
    pub value: String,
    pub severity: Option<String>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ProfileCounts {
    pub total_orders: i64,
    pub total_bookings: i64,
    pub preference_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DataExportRecord {
    pub display_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub consent_version: String,
    pub consent_granted_at: String,
    pub preferences: Vec<StoredPreferenceView>,
    pub order_ids: Vec<String>,
    pub booking_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EnableMemoryOutput {
    pub status: String,
    pub profile_token: Option<String>,
    pub guest_name: String,
    pub consent_version: String,
    pub preferences_saved: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GetProfileOutput {
    pub status: String,
    pub guest_name: String,
    pub phone_masked: Option<String>,
    pub email_masked: Option<String>,
    pub consent_version: String,
    pub consent_granted_at: String,
    pub preferences: Vec<StoredPreferenceView>,
    pub counts: ProfileCounts,
    pub needs_reconfirmation: Vec<String>,
    pub data_export: Option<DataExportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RememberPreferenceOutput {
    pub status: String,
    pub preference: Option<StoredPreferenceView>,
    pub deduplicated: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ForgetPreferenceOutput {
    pub status: String,
    pub kind: String,
    pub value: String,
    pub removed: bool,
    pub policy_note: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RotateProfileTokenOutput {
    pub status: String,
    pub new_profile_token: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ScrubbedBusinessRecordsSummary {
    pub scrubbed_orders: i64,
    pub scrubbed_bookings: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ForgetMeOutput {
    pub status: String,
    pub removed_components: Vec<String>,
    pub scrubbed_business_records: ScrubbedBusinessRecordsSummary,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SuggestedOrderLine {
    pub item_id: i64,
    pub item_name: String,
    pub quantity: i64,
    pub unit_price: PriceDisplay,
    pub line_total: PriceDisplay,
    pub user_provided_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PriceDifference {
    pub item_id: i64,
    pub item_name: String,
    pub previous_price: PriceDisplay,
    pub current_price: PriceDisplay,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ConflictingAllergyItem {
    pub item_id: i64,
    pub item_name: String,
    pub conflicting_allergens: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GetUsualOrderOutput {
    pub status: String, // "suggested" | "no_history" | "all_conflicted" | "not_found"
    pub guest_name: Option<String>,
    pub suggested_lines: Vec<SuggestedOrderLine>,
    pub subtotal: Option<PriceDisplay>,
    pub tax: Option<PriceDisplay>,
    pub total: Option<PriceDisplay>,
    pub price_changes: Vec<PriceDifference>,
    pub unavailable_items: Vec<String>,
    pub conflicting_items: Vec<ConflictingAllergyItem>,
    pub needs_allergy_reconfirmation: Vec<String>,
    pub allergen_notice: String,
}

// Generic profile not found output
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct GenericProfileNotFoundOutput {
    pub status: String,
    pub message: String,
}

impl GenericProfileNotFoundOutput {
    pub fn new() -> Self {
        Self {
            status: "not_found".to_string(),
            message: "Profile not found or invalid profile token.".to_string(),
        }
    }
}

impl Default for GenericProfileNotFoundOutput {
    fn default() -> Self {
        Self::new()
    }
}

// --- Core Database Engine Functions ---

/// Resolves a profile token to a customer ID, enforcing rate limiting and checking revocation.
pub async fn resolve_profile_token(
    pool: &SqlitePool,
    token: &str,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<Option<Customer>, sqlx::Error> {
    let now = clock.now();
    let token_clean = token.trim();
    if token_clean.is_empty() || !token_clean.starts_with("tt_prof_") {
        return Ok(None);
    }

    let token_hash = hash_profile_token(token_clean);

    // Rate limiter check on the hash prefix
    let rate_key = if token_hash.len() >= 12 {
        &token_hash[..12]
    } else {
        &token_hash
    };
    if rate_limiter.is_rate_limited(rate_key, now) {
        return Ok(None);
    }

    // Query active token
    let row: Option<(i64, String, Option<String>)> = sqlx::query_as(
        r#"
        SELECT customer_id, token_hash, revoked_at
        FROM profile_tokens
        WHERE token_hash = ? AND revoked_at IS NULL
        "#,
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    if let Some((cust_id, stored_hash, _)) = row {
        // Constant-time check on token hash
        if !constant_time_eq_slices(stored_hash.as_bytes(), token_hash.as_bytes()) {
            rate_limiter.record_failure(rate_key, now);
            return Ok(None);
        }

        // Token is valid! Reset rate limiter
        rate_limiter.reset(rate_key);

        // Update last_used_at
        let now_iso = now.to_rfc3339();
        let _ = sqlx::query("UPDATE profile_tokens SET last_used_at = ? WHERE token_hash = ?")
            .bind(&now_iso)
            .bind(&token_hash)
            .execute(pool)
            .await;

        // Fetch customer record
        let customer: Option<Customer> = sqlx::query_as(
            r#"
            SELECT id, display_name, phone, email, created_at
            FROM customers
            WHERE id = ?
            "#,
        )
        .bind(cust_id)
        .fetch_optional(pool)
        .await?;

        Ok(customer)
    } else {
        rate_limiter.record_failure(rate_key, now);
        Ok(None)
    }
}

/// Enables memory for a guest, creating the profile, consent, and initial token.
pub async fn enable_memory_tx(
    pool: &SqlitePool,
    guest_name: &str,
    phone: Option<&str>,
    email: Option<&str>,
    initial_preferences: &[InitialPreferenceInput],
    clock: &dyn Clock,
) -> Result<EnableMemoryOutput, sqlx::Error> {
    let now = clock.now();
    let now_iso = now.to_rfc3339();

    // 0. Clean and validate initial preferences before opening transaction
    let mut cleaned_prefs = Vec::new();
    for pref in initial_preferences {
        if let Ok(clean) = validate_and_clean_preference(pool, &pref.kind, &pref.value).await {
            cleaned_prefs.push((pref, clean));
        }
    }

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<EnableMemoryOutput, sqlx::Error> = async {
        // 1. Insert customer
        let cust_res = sqlx::query(
            r#"
            INSERT INTO customers (display_name, phone, email, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(guest_name.trim())
        .bind(phone.map(|s| s.trim()))
        .bind(email.map(|s| s.trim()))
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        let customer_id = cust_res.last_insert_rowid();

        // 2. Insert consent record
        sqlx::query(
            r#"
            INSERT INTO memory_consents (customer_id, consent_version, granted_at, revoked_at)
            VALUES (?, 'v1.0', ?, NULL)
            "#,
        )
        .bind(customer_id)
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        // 3. Generate token & insert hash
        let plain_token = generate_profile_token();
        let token_hash = hash_profile_token(&plain_token);

        sqlx::query(
            r#"
            INSERT INTO profile_tokens (customer_id, token_hash, created_at, last_used_at, revoked_at)
            VALUES (?, ?, ?, ?, NULL)
            "#,
        )
        .bind(customer_id)
        .bind(&token_hash)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        // 4. Save initial preferences
        let mut saved_count = 0;
        for (pref, (kind, clean_val)) in cleaned_prefs {
            let sev = if kind == PreferenceKind::Allergy {
                pref.severity.clone().or(Some("unknown".to_string()))
            } else {
                None
            };
            let conf_at = if kind == PreferenceKind::Allergy {
                Some(now_iso.clone())
            } else {
                None
            };

            sqlx::query(
                r#"
                INSERT INTO customer_preferences (customer_id, kind, value, source, created_at, confirmed_at, severity)
                VALUES (?, ?, ?, 'explicit', ?, ?, ?)
                "#,
            )
            .bind(customer_id)
            .bind(kind)
            .bind(&clean_val)
            .bind(&now_iso)
            .bind(conf_at)
            .bind(sev)
            .execute(&mut *conn)
            .await?;

            saved_count += 1;
        }

        // 5. Audit event (zero PII in payload)
        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('customer', ?, 'enable_memory', 'mcp', NULL, ?)
            "#,
        )
        .bind(customer_id.to_string())
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        sqlx::query("COMMIT").execute(&mut *conn).await?;

        Ok(EnableMemoryOutput {
            status: "enabled".to_string(),
            profile_token: Some(plain_token),
            guest_name: guest_name.trim().to_string(),
            consent_version: "v1.0".to_string(),
            preferences_saved: saved_count,
            message: "Memory profile successfully enabled. Keep your profile token secure; it will not be shown again.".to_string(),
        })
    }.await;

    match res {
        Ok(out) => Ok(out),
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Rotates an existing profile token atomically.
pub async fn rotate_profile_token_tx(
    pool: &SqlitePool,
    old_token: &str,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<RotateProfileTokenOutput, sqlx::Error> {
    let now = clock.now();
    let now_iso = now.to_rfc3339();

    let customer = match resolve_profile_token(pool, old_token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(RotateProfileTokenOutput {
                status: "not_found".to_string(),
                new_profile_token: None,
                message: "Profile not found or invalid profile token.".to_string(),
            });
        }
    };

    let old_hash = hash_profile_token(old_token);
    let new_plain = generate_profile_token();
    let new_hash = hash_profile_token(&new_plain);

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<RotateProfileTokenOutput, sqlx::Error> = async {
        // Revoke old token
        sqlx::query(
            "UPDATE profile_tokens SET revoked_at = ? WHERE token_hash = ? AND revoked_at IS NULL",
        )
        .bind(&now_iso)
        .bind(&old_hash)
        .execute(&mut *conn)
        .await?;

        // Insert new token
        sqlx::query(
            r#"
            INSERT INTO profile_tokens (customer_id, token_hash, created_at, last_used_at, revoked_at)
            VALUES (?, ?, ?, ?, NULL)
            "#,
        )
        .bind(customer.id)
        .bind(&new_hash)
        .bind(&now_iso)
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        // Audit event
        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('customer', ?, 'rotate_token', 'mcp', NULL, ?)
            "#,
        )
        .bind(customer.id.to_string())
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        sqlx::query("COMMIT").execute(&mut *conn).await?;

        Ok(RotateProfileTokenOutput {
            status: "rotated".to_string(),
            new_profile_token: Some(new_plain),
            message: "Profile token successfully rotated. The previous token has been permanently revoked.".to_string(),
        })
    }.await;

    match res {
        Ok(out) => Ok(out),
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Hard deletion of guest profile and preferences (`forget_me`).
/// Unlinks and scrubs historical orders and bookings in the same transaction.
pub async fn forget_me_tx(
    pool: &SqlitePool,
    token: &str,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<ForgetMeOutput, sqlx::Error> {
    let now = clock.now();
    let now_iso = now.to_rfc3339();

    let customer = match resolve_profile_token(pool, token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(ForgetMeOutput {
                status: "not_found".to_string(),
                removed_components: vec![],
                scrubbed_business_records: ScrubbedBusinessRecordsSummary {
                    scrubbed_orders: 0,
                    scrubbed_bookings: 0,
                },
                message: "Profile not found or invalid profile token.".to_string(),
            });
        }
    };

    let cust_id = customer.id;

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    let res: Result<ForgetMeOutput, sqlx::Error> = async {
        // 0. Delete cached idempotency records containing customer responses
        sqlx::query(
            r#"
            DELETE FROM idempotency_keys
            WHERE order_id IN (SELECT id FROM orders WHERE customer_id = ?)
               OR booking_id IN (SELECT id FROM bookings WHERE customer_id = ?)
            "#,
        )
        .bind(cust_id)
        .bind(cust_id)
        .execute(&mut *conn)
        .await?;

        // 1. Scrub notes in order items of this customer's past orders first
        sqlx::query(
            r#"
            UPDATE order_items
            SET notes = NULL
            WHERE order_id IN (SELECT id FROM orders WHERE customer_id = ?)
            "#,
        )
        .bind(cust_id)
        .execute(&mut *conn)
        .await?;

        // 2. Scrub orders
        let orders_affected = sqlx::query(
            r#"
            UPDATE orders
            SET customer_id = NULL,
                guest_name = '[deleted]',
                guest_phone = NULL,
                guest_email = NULL,
                notes = NULL,
                stated_allergens = NULL
            WHERE customer_id = ?
            "#,
        )
        .bind(cust_id)
        .execute(&mut *conn)
        .await?
        .rows_affected() as i64;

        // 2. Scrub bookings
        let bookings_affected = sqlx::query(
            r#"
            UPDATE bookings
            SET customer_id = NULL,
                guest_name = '[deleted]',
                guest_phone = NULL,
                guest_email = NULL,
                special_requests = NULL
            WHERE customer_id = ?
            "#,
        )
        .bind(cust_id)
        .execute(&mut *conn)
        .await?
        .rows_affected() as i64;

        // 2b. Delete waitlist entries, holds, and negotiations linked to customer or guest name
        sqlx::query("DELETE FROM waitlist_entries WHERE customer_id = ? OR guest_name = ?")
            .bind(cust_id)
            .bind(&customer.display_name)
            .execute(&mut *conn)
            .await?;

        sqlx::query("DELETE FROM slot_holds WHERE guest_name = ?")
            .bind(&customer.display_name)
            .execute(&mut *conn)
            .await?;

        sqlx::query("DELETE FROM negotiations WHERE guest_name = ?")
            .bind(&customer.display_name)
            .execute(&mut *conn)
            .await?;

        // 3. Audit event (opaque ID only, no PII)
        sqlx::query(
            r#"
            INSERT INTO audit_events (entity, entity_id, action, actor, payload_json, created_at)
            VALUES ('customer', ?, 'customer_forget_me', 'mcp', NULL, ?)
            "#,
        )
        .bind(cust_id.to_string())
        .bind(&now_iso)
        .execute(&mut *conn)
        .await?;

        // 4. Hard delete customer (cascades to customer_preferences, profile_tokens, memory_consents)
        sqlx::query("DELETE FROM customers WHERE id = ?")
            .bind(cust_id)
            .execute(&mut *conn)
            .await?;

        sqlx::query("COMMIT").execute(&mut *conn).await?;

        Ok(ForgetMeOutput {
            status: "forgotten".to_string(),
            removed_components: vec![
                "profile".to_string(),
                "preferences".to_string(),
                "profile_tokens".to_string(),
                "consent_records".to_string(),
                "waitlist_entries".to_string(),
                "slot_holds".to_string(),
                "negotiations".to_string(),
            ],
            scrubbed_business_records: ScrubbedBusinessRecordsSummary {
                scrubbed_orders: orders_affected,
                scrubbed_bookings: bookings_affected,
            },
            message: "Your profile, preferences, and security tokens have been permanently deleted. Historical order and booking records have been scrubbed of all personal identifiers.".to_string(),
        })
    }.await;

    match res {
        Ok(out) => Ok(out),
        Err(e) => {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
            Err(e)
        }
    }
}

/// Retrieves the profile details, masked contact, preferences, and data export.
pub async fn get_profile_tx(
    pool: &SqlitePool,
    token: &str,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<GetProfileOutput, sqlx::Error> {
    let now = clock.now();
    let customer = match resolve_profile_token(pool, token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(GetProfileOutput {
                status: "not_found".to_string(),
                guest_name: String::new(),
                phone_masked: None,
                email_masked: None,
                consent_version: String::new(),
                consent_granted_at: String::new(),
                preferences: vec![],
                counts: ProfileCounts {
                    total_orders: 0,
                    total_bookings: 0,
                    preference_count: 0,
                },
                needs_reconfirmation: vec![],
                data_export: None,
            });
        }
    };

    let cust_id = customer.id;

    // Fetch consent
    let consent: Option<(String, String)> = sqlx::query_as(
        "SELECT consent_version, granted_at FROM memory_consents WHERE customer_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(cust_id)
    .fetch_optional(pool)
    .await?;

    let (consent_version, consent_granted_at) =
        consent.unwrap_or_else(|| ("v1.0".to_string(), now.to_rfc3339()));

    // Fetch preferences
    type RawPrefRow = (
        i64,
        PreferenceKind,
        String,
        Option<String>,
        Option<String>,
        String,
    );
    let raw_prefs: Vec<RawPrefRow> = sqlx::query_as(
        r#"
            SELECT id, kind, value, severity, confirmed_at, created_at
            FROM customer_preferences
            WHERE customer_id = ?
            ORDER BY id ASC
            "#,
    )
    .bind(cust_id)
    .fetch_all(pool)
    .await?;

    let mut preferences = Vec::new();
    let mut needs_reconfirmation = Vec::new();
    let reconfirmation_cutoff = now - Duration::days(ALLERGY_RECONFIRMATION_DAYS);

    for (id, kind, value, severity, confirmed_at, created_at) in raw_prefs {
        let kind_str = match kind {
            PreferenceKind::Allergy => "allergy",
            PreferenceKind::Dietary => "dietary",
            PreferenceKind::SeatPref => "seat_pref",
            PreferenceKind::FavoriteItem => "favorite_item",
            PreferenceKind::Note => "note",
        }
        .to_string();

        if kind == PreferenceKind::Allergy {
            let is_stale = match &confirmed_at {
                Some(ts) => match DateTime::parse_from_rfc3339(ts) {
                    Ok(dt) => dt.with_timezone(&Utc) < reconfirmation_cutoff,
                    Err(_) => true,
                },
                None => true,
            };
            if is_stale && !needs_reconfirmation.contains(&value) {
                needs_reconfirmation.push(value.clone());
            }
        }

        preferences.push(StoredPreferenceView {
            id,
            kind: kind_str,
            value,
            severity,
            confirmed_at,
            created_at,
        });
    }

    // Counts
    let total_orders: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE customer_id = ?")
        .bind(cust_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let total_bookings: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM bookings WHERE customer_id = ?")
            .bind(cust_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let counts = ProfileCounts {
        total_orders,
        total_bookings,
        preference_count: preferences.len() as i64,
    };

    // Data export
    let order_ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM orders WHERE customer_id = ? ORDER BY created_at DESC")
            .bind(cust_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

    let booking_ids: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM bookings WHERE customer_id = ? ORDER BY created_at DESC",
    )
    .bind(cust_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let data_export = Some(DataExportRecord {
        display_name: customer.display_name.clone(),
        phone: customer.phone.clone(),
        email: customer.email.clone(),
        consent_version: consent_version.clone(),
        consent_granted_at: consent_granted_at.clone(),
        preferences: preferences.clone(),
        order_ids,
        booking_ids,
    });

    Ok(GetProfileOutput {
        status: "found".to_string(),
        guest_name: customer.display_name,
        phone_masked: mask_phone(customer.phone.as_deref()),
        email_masked: mask_email(customer.email.as_deref()),
        consent_version,
        consent_granted_at,
        preferences,
        counts,
        needs_reconfirmation,
        data_export,
    })
}

/// Stores or deduplicates a guest dining preference.
pub async fn remember_preference_tx(
    pool: &SqlitePool,
    token: &str,
    kind_str: &str,
    value: &str,
    severity: Option<&str>,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<Result<RememberPreferenceOutput, String>, sqlx::Error> {
    let now = clock.now();
    let now_iso = now.to_rfc3339();

    let customer = match resolve_profile_token(pool, token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(Ok(RememberPreferenceOutput {
                status: "not_found".to_string(),
                preference: None,
                deduplicated: false,
                message: "Profile not found or invalid profile token.".to_string(),
            }));
        }
    };

    let cust_id = customer.id;

    let (kind, clean_val) = match validate_and_clean_preference(pool, kind_str, value).await {
        Ok(res) => res,
        Err(err) => return Ok(Err(err)),
    };

    // If Note: check count limit
    if kind == PreferenceKind::Note {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM customer_preferences WHERE customer_id = ? AND kind = 'note'",
        )
        .bind(cust_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if count >= MAX_NOTES_PER_CUSTOMER as i64 {
            return Ok(Err(format!(
                "Maximum limit of {} notes per profile reached.",
                MAX_NOTES_PER_CUSTOMER
            )));
        }
    }

    let sev = if kind == PreferenceKind::Allergy {
        match severity.map(|s| s.to_lowercase()) {
            Some(s) if s == "mild" || s == "severe" => Some(s),
            _ => Some("unknown".to_string()),
        }
    } else {
        None
    };

    let mut conn = pool.acquire().await?;
    sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

    // Check if preference already exists (deduplication)
    let existing: Option<(i64, Option<String>, Option<String>, String)> = sqlx::query_as(
        r#"
        SELECT id, severity, confirmed_at, created_at
        FROM customer_preferences
        WHERE customer_id = ? AND kind = ? AND LOWER(value) = LOWER(?)
        LIMIT 1
        "#,
    )
    .bind(cust_id)
    .bind(kind)
    .bind(&clean_val)
    .fetch_optional(&mut *conn)
    .await?;

    let out = if let Some((id, _, _, created_at)) = existing {
        // Update confirmed_at and severity for allergy
        if kind == PreferenceKind::Allergy {
            sqlx::query(
                "UPDATE customer_preferences SET confirmed_at = ?, severity = ? WHERE id = ?",
            )
            .bind(&now_iso)
            .bind(&sev)
            .bind(id)
            .execute(&mut *conn)
            .await?;
        }

        sqlx::query("COMMIT").execute(&mut *conn).await?;

        RememberPreferenceOutput {
            status: "remembered".to_string(),
            preference: Some(StoredPreferenceView {
                id,
                kind: kind_str.to_lowercase(),
                value: clean_val,
                severity: sev,
                confirmed_at: if kind == PreferenceKind::Allergy {
                    Some(now_iso)
                } else {
                    None
                },
                created_at,
            }),
            deduplicated: true,
            message: "Preference already stored; updated confirmation timestamp and severity."
                .to_string(),
        }
    } else {
        // Insert new preference
        let conf_at = if kind == PreferenceKind::Allergy {
            Some(now_iso.clone())
        } else {
            None
        };

        let res = sqlx::query(
            r#"
            INSERT INTO customer_preferences (customer_id, kind, value, source, created_at, confirmed_at, severity)
            VALUES (?, ?, ?, 'voice', ?, ?, ?)
            "#,
        )
        .bind(cust_id)
        .bind(kind)
        .bind(&clean_val)
        .bind(&now_iso)
        .bind(&conf_at)
        .bind(&sev)
        .execute(&mut *conn)
        .await?;

        let new_id = res.last_insert_rowid();

        sqlx::query("COMMIT").execute(&mut *conn).await?;

        RememberPreferenceOutput {
            status: "remembered".to_string(),
            preference: Some(StoredPreferenceView {
                id: new_id,
                kind: kind_str.to_lowercase(),
                value: clean_val,
                severity: sev,
                confirmed_at: conf_at,
                created_at: now_iso,
            }),
            deduplicated: false,
            message: "Preference successfully saved to your profile.".to_string(),
        }
    };

    Ok(Ok(out))
}

/// Removes a saved dining preference from the guest profile.
pub async fn forget_preference_tx(
    pool: &SqlitePool,
    token: &str,
    kind_str: &str,
    value: &str,
    confirm_remove_allergy: bool,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<Result<ForgetPreferenceOutput, String>, sqlx::Error> {
    let customer = match resolve_profile_token(pool, token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(Ok(ForgetPreferenceOutput {
                status: "not_found".to_string(),
                kind: kind_str.to_string(),
                value: value.to_string(),
                removed: false,
                policy_note: None,
                message: "Profile not found or invalid profile token.".to_string(),
            }));
        }
    };

    let cust_id = customer.id;
    let clean_val = value.trim();

    let kind = match kind_str.to_lowercase().as_str() {
        "allergy" => PreferenceKind::Allergy,
        "dietary" => PreferenceKind::Dietary,
        "seat_pref" => PreferenceKind::SeatPref,
        "favorite_item" => PreferenceKind::FavoriteItem,
        "note" => PreferenceKind::Note,
        other => return Ok(Err(format!("Invalid preference kind '{}'.", other))),
    };

    if kind == PreferenceKind::Allergy && !confirm_remove_allergy {
        return Ok(Ok(ForgetPreferenceOutput {
            status: "confirmation_required".to_string(),
            kind: "allergy".to_string(),
            value: clean_val.to_string(),
            removed: false,
            policy_note: Some("Removing an allergy preference requires explicit guest confirmation (confirm_remove_allergy: true). Removal applies to the digital profile only; please remind staff of active allergies.".to_string()),
            message: "Removing an allergy from your profile requires explicit confirmation. Please confirm you wish to remove this allergy.".to_string(),
        }));
    }

    let rows = sqlx::query(
        "DELETE FROM customer_preferences WHERE customer_id = ? AND kind = ? AND LOWER(value) = LOWER(?)",
    )
    .bind(cust_id)
    .bind(kind)
    .bind(clean_val)
    .execute(pool)
    .await?
    .rows_affected();

    if rows > 0 {
        let policy_note = if kind == PreferenceKind::Allergy {
            Some("Allergy preference removed from profile only. The kitchen cannot guarantee an allergy-free environment; severe allergies must always be confirmed directly with staff at each visit.".to_string())
        } else {
            None
        };
        Ok(Ok(ForgetPreferenceOutput {
            status: "forgotten".to_string(),
            kind: kind_str.to_string(),
            value: clean_val.to_string(),
            removed: true,
            policy_note,
            message: format!(
                "Successfully removed '{}' from your profile preferences.",
                clean_val
            ),
        }))
    } else {
        Ok(Ok(ForgetPreferenceOutput {
            status: "not_found".to_string(),
            kind: kind_str.to_string(),
            value: clean_val.to_string(),
            removed: false,
            policy_note: None,
            message: format!("Preference '{}' not found in your profile.", clean_val),
        }))
    }
}

/// Computes the guest's usual order based on completed/placed order history,
/// re-validating against live menu prices, stock availability, and profile allergies.
pub async fn get_usual_order_tx(
    pool: &SqlitePool,
    token: &str,
    rate_limiter: &ProfileTokenRateLimiter,
    clock: &dyn Clock,
) -> Result<GetUsualOrderOutput, sqlx::Error> {
    let now = clock.now();
    let customer = match resolve_profile_token(pool, token, rate_limiter, clock).await? {
        Some(c) => c,
        None => {
            return Ok(GetUsualOrderOutput {
                status: "not_found".to_string(),
                guest_name: None,
                suggested_lines: vec![],
                subtotal: None,
                tax: None,
                total: None,
                price_changes: vec![],
                unavailable_items: vec![],
                conflicting_items: vec![],
                needs_allergy_reconfirmation: vec![],
                allergen_notice: ALLERGEN_NOTICE.to_string(),
            });
        }
    };

    let cust_id = customer.id;

    // 1. Fetch most recent order
    let past_order: Option<(String, String)> = sqlx::query_as(
        r#"
        SELECT id, created_at
        FROM orders
        WHERE customer_id = ? AND status IN ('completed', 'placed')
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(cust_id)
    .fetch_optional(pool)
    .await?;

    let order_id = match past_order {
        Some((oid, _)) => oid,
        None => {
            return Ok(GetUsualOrderOutput {
                status: "no_history".to_string(),
                guest_name: Some(customer.display_name),
                suggested_lines: vec![],
                subtotal: None,
                tax: None,
                total: None,
                price_changes: vec![],
                unavailable_items: vec![],
                conflicting_items: vec![],
                needs_allergy_reconfirmation: vec![],
                allergen_notice: ALLERGEN_NOTICE.to_string(),
            });
        }
    };

    // 2. Fetch past order lines
    let past_lines: Vec<(i64, String, i64, i64, Option<String>)> = sqlx::query_as(
        r#"
        SELECT menu_item_id, item_name, quantity, unit_price_cents, notes
        FROM order_items
        WHERE order_id = ?
        ORDER BY id ASC
        "#,
    )
    .bind(&order_id)
    .fetch_all(pool)
    .await?;

    // 3. Fetch profile's stored allergies
    let profile_allergies: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT value, confirmed_at FROM customer_preferences WHERE customer_id = ? AND kind = 'allergy'",
    )
    .bind(cust_id)
    .fetch_all(pool)
    .await?;

    let mut needs_allergy_reconfirmation = Vec::new();
    let reconf_cutoff = now - Duration::days(ALLERGY_RECONFIRMATION_DAYS);
    let mut stored_allergy_names = Vec::new();

    for (allergy_name, confirmed_at) in profile_allergies {
        let is_stale = match &confirmed_at {
            Some(ts) => match DateTime::parse_from_rfc3339(ts) {
                Ok(dt) => dt.with_timezone(&Utc) < reconf_cutoff,
                Err(_) => true,
            },
            None => true,
        };
        if is_stale && !needs_allergy_reconfirmation.contains(&allergy_name) {
            needs_allergy_reconfirmation.push(allergy_name.clone());
        }
        stored_allergy_names.push(allergy_name.to_lowercase());
    }

    // 4. Validate each past line against live menu
    let mut suggested_lines = Vec::new();
    let mut price_changes = Vec::new();
    let mut unavailable_items = Vec::new();
    let mut conflicting_items = Vec::new();
    let mut subtotal_cents: i64 = 0;

    for (item_id, item_name, quantity, past_price_cents, notes) in past_lines {
        // Query live menu item
        let live_item: Option<(String, i64, bool)> =
            sqlx::query_as("SELECT name, price_cents, is_available FROM menu_items WHERE id = ?")
                .bind(item_id)
                .fetch_optional(pool)
                .await?;

        match live_item {
            None => {
                unavailable_items.push(item_name);
            }
            Some((current_name, current_price_cents, is_available)) => {
                if !is_available {
                    unavailable_items.push(current_name);
                    continue;
                }

                // Check item allergens
                let item_allergens: Vec<String> = sqlx::query_scalar(
                    r#"
                    SELECT a.name
                    FROM menu_item_allergens mia
                    JOIN allergens a ON a.id = mia.allergen_id
                    WHERE mia.menu_item_id = ?
                    "#,
                )
                .bind(item_id)
                .fetch_all(pool)
                .await?;

                let conflicts: Vec<String> = item_allergens
                    .iter()
                    .filter(|a| stored_allergy_names.contains(&a.to_lowercase()))
                    .cloned()
                    .collect();

                if !conflicts.is_empty() {
                    conflicting_items.push(ConflictingAllergyItem {
                        item_id,
                        item_name: current_name,
                        conflicting_allergens: conflicts,
                    });
                    continue;
                }

                // Check price difference
                if current_price_cents != past_price_cents {
                    price_changes.push(PriceDifference {
                        item_id,
                        item_name: current_name.clone(),
                        previous_price: PriceDisplay {
                            amount_minor: past_price_cents,
                            currency: "INR".to_string(),
                            display: format!("₹{}", past_price_cents / 100),
                        },
                        current_price: PriceDisplay {
                            amount_minor: current_price_cents,
                            currency: "INR".to_string(),
                            display: format!("₹{}", current_price_cents / 100),
                        },
                    });
                }

                let line_total_cents = quantity.saturating_mul(current_price_cents);
                subtotal_cents = subtotal_cents.saturating_add(line_total_cents);

                suggested_lines.push(SuggestedOrderLine {
                    item_id,
                    item_name: current_name,
                    quantity,
                    unit_price: PriceDisplay {
                        amount_minor: current_price_cents,
                        currency: "INR".to_string(),
                        display: format!("₹{}", current_price_cents / 100),
                    },
                    line_total: PriceDisplay {
                        amount_minor: line_total_cents,
                        currency: "INR".to_string(),
                        display: format!("₹{}", line_total_cents / 100),
                    },
                    user_provided_note: notes,
                });
            }
        }
    }

    if suggested_lines.is_empty() {
        return Ok(GetUsualOrderOutput {
            status: "all_conflicted".to_string(),
            guest_name: Some(customer.display_name),
            suggested_lines: vec![],
            subtotal: None,
            tax: None,
            total: None,
            price_changes,
            unavailable_items,
            conflicting_items,
            needs_allergy_reconfirmation,
            allergen_notice: ALLERGEN_NOTICE.to_string(),
        });
    }

    // Tax calculation (500 bps = 5% GST round-half-up)
    let tax_cents = (subtotal_cents * 500 + 5000) / 10000;
    let total_cents = subtotal_cents + tax_cents;

    let subtotal_disp = PriceDisplay {
        amount_minor: subtotal_cents,
        currency: "INR".to_string(),
        display: format!("₹{}", subtotal_cents / 100),
    };
    let tax_disp = PriceDisplay {
        amount_minor: tax_cents,
        currency: "INR".to_string(),
        display: format!("₹{}", tax_cents / 100),
    };
    let total_disp = PriceDisplay {
        amount_minor: total_cents,
        currency: "INR".to_string(),
        display: format!("₹{}", total_cents / 100),
    };

    Ok(GetUsualOrderOutput {
        status: "suggested".to_string(),
        guest_name: Some(customer.display_name),
        suggested_lines,
        subtotal: Some(subtotal_disp),
        tax: Some(tax_disp),
        total: Some(total_disp),
        price_changes,
        unavailable_items,
        conflicting_items,
        needs_allergy_reconfirmation,
        allergen_notice: ALLERGEN_NOTICE.to_string(),
    })
}

/// Purges expired profiles inactive for `retention_days`.
pub async fn purge_expired_profiles(
    pool: &SqlitePool,
    retention_days: i64,
    clock: &dyn Clock,
) -> Result<usize, sqlx::Error> {
    let now = clock.now();
    let cutoff = (now - Duration::days(retention_days)).to_rfc3339();

    // Find customers whose newest token activity is before cutoff
    // and who have no active (placed or preparing) orders
    let eligible_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        SELECT c.id
        FROM customers c
        WHERE EXISTS (
            SELECT 1 FROM profile_tokens pt
            WHERE pt.customer_id = c.id
        )
        AND NOT EXISTS (
            SELECT 1 FROM profile_tokens pt
            WHERE pt.customer_id = c.id
              AND (pt.last_used_at > ? OR (pt.last_used_at IS NULL AND pt.created_at > ?))
        )
        AND NOT EXISTS (
            SELECT 1 FROM orders o
            WHERE o.customer_id = c.id
              AND o.status IN ('placed', 'preparing')
        )
        AND NOT EXISTS (
            SELECT 1 FROM bookings b
            WHERE b.customer_id = c.id
              AND b.status IN ('pending', 'confirmed')
              AND b.end_at > ?
        )
        "#,
    )
    .bind(&cutoff)
    .bind(&cutoff)
    .bind(now.to_rfc3339())
    .fetch_all(pool)
    .await?;

    let mut purged = 0;
    for id in eligible_ids {
        let mut conn = pool.acquire().await?;
        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;

        let res: Result<(), sqlx::Error> = async {
            // Delete cached idempotency records
            sqlx::query(
                r#"
                DELETE FROM idempotency_keys
                WHERE order_id IN (SELECT id FROM orders WHERE customer_id = ?)
                   OR booking_id IN (SELECT id FROM bookings WHERE customer_id = ?)
                "#,
            )
            .bind(id)
            .bind(id)
            .execute(&mut *conn)
            .await?;

            // Scrub order items notes first
            sqlx::query(
                "UPDATE order_items SET notes = NULL WHERE order_id IN (SELECT id FROM orders WHERE customer_id = ?)"
            )
            .bind(id)
            .execute(&mut *conn)
            .await?;

            // Scrub orders
            sqlx::query(
                "UPDATE orders SET customer_id = NULL, guest_name = '[deleted]', guest_phone = NULL, guest_email = NULL, notes = NULL, stated_allergens = NULL WHERE customer_id = ?"
            )
            .bind(id)
            .execute(&mut *conn)
            .await?;

            // Scrub bookings
            sqlx::query(
                "UPDATE bookings SET customer_id = NULL, guest_name = '[deleted]', guest_phone = NULL, guest_email = NULL, special_requests = NULL WHERE customer_id = ?"
            )
            .bind(id)
            .execute(&mut *conn)
            .await?;

            // Delete customer
            sqlx::query("DELETE FROM customers WHERE id = ?")
                .bind(id)
                .execute(&mut *conn)
                .await?;

            sqlx::query("COMMIT").execute(&mut *conn).await?;
            Ok(())
        }.await;

        if res.is_ok() {
            purged += 1;
        } else {
            let _ = sqlx::query("ROLLBACK").execute(&mut *conn).await;
        }
    }

    Ok(purged)
}

/// Validates preference kind and value.
pub async fn validate_and_clean_preference(
    pool: &SqlitePool,
    kind_str: &str,
    value: &str,
) -> Result<(PreferenceKind, String), String> {
    let clean_val = value.trim();
    if clean_val.is_empty() {
        return Err("Preference value cannot be empty.".to_string());
    }

    match kind_str.to_lowercase().as_str() {
        "allergy" => {
            // Must match allergens table
            let row: Option<String> = sqlx::query_scalar(
                "SELECT name FROM allergens WHERE LOWER(name) = LOWER(?)",
            )
            .bind(clean_val)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

            if let Some(canonical) = row {
                Ok((PreferenceKind::Allergy, canonical))
            } else {
                Err(format!(
                    "Unknown allergen '{}'. Must match official allergen register.",
                    clean_val
                ))
            }
        }
        "dietary" => {
            // Must match dietary_tags table
            let row: Option<String> = sqlx::query_scalar(
                "SELECT name FROM dietary_tags WHERE LOWER(name) = LOWER(?)",
            )
            .bind(clean_val)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

            if let Some(canonical) = row {
                Ok((PreferenceKind::Dietary, canonical))
            } else {
                Err(format!(
                    "Unknown dietary tag '{}'. Must match official dietary register.",
                    clean_val
                ))
            }
        }
        "seat_pref" => {
            // Must start with or contain area (indoor, patio, bar)
            let lower = clean_val.to_lowercase();
            if lower.contains("indoor") || lower.contains("patio") || lower.contains("bar") || lower.contains("booth") {
                Ok((PreferenceKind::SeatPref, clean_val.to_string()))
            } else {
                Err(format!(
                    "Seat preference must reference a valid dining area (indoor, patio, bar) or seating type. Received '{}'.",
                    clean_val
                ))
            }
        }
        "favorite_item" => {
            // Must resolve to a real menu item ID or exact name
            if let Ok(id) = clean_val.parse::<i64>() {
                let name: Option<String> = sqlx::query_scalar("SELECT name FROM menu_items WHERE id = ?")
                    .bind(id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                if let Some(n) = name {
                    return Ok((PreferenceKind::FavoriteItem, n));
                }
            }

            let name: Option<String> = sqlx::query_scalar(
                "SELECT name FROM menu_items WHERE LOWER(name) = LOWER(?)",
            )
            .bind(clean_val)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

            if let Some(canonical) = name {
                Ok((PreferenceKind::FavoriteItem, canonical))
            } else {
                Err(format!(
                    "Favorite item '{}' not found on the restaurant menu.",
                    clean_val
                ))
            }
        }
        "note" => {
            let sanitized = sanitize_note(clean_val);
            if sanitized.is_empty() {
                Err("Note contains only invalid or control characters.".to_string())
            } else {
                Ok((PreferenceKind::Note, sanitized))
            }
        }
        other => Err(format!(
            "Invalid preference kind '{}'. Must be one of: allergy, dietary, seat_pref, favorite_item, note.",
            other
        )),
    }
}

// --- Spoken Text Generators for Phase 10 Tools ---

pub fn generate_enable_memory_voice_text(guest_name: &str) -> String {
    let first_name = guest_name.split_whitespace().next().unwrap_or(guest_name);
    format!(
        "I've saved your profile, {}. We will remember your dining preferences and past orders for future visits. You can ask me to forget your profile at any time.",
        first_name
    )
}

pub fn generate_forget_me_voice_text() -> String {
    "I have completely removed your profile and preferences. Your historical orders and bookings have been anonymized for business records.".to_string()
}

pub fn generate_forget_preference_voice_text(kind: PreferenceKind, value: &str) -> String {
    if kind == PreferenceKind::Allergy {
        format!(
            "I've removed {} from your saved profile preferences. Please remember to let our staff know directly if you have any active food allergies.",
            value
        )
    } else {
        format!("I've removed {} from your saved preferences.", value)
    }
}

pub fn generate_usual_order_voice_text(out: &GetUsualOrderOutput, first_name: &str) -> String {
    if out.status == "no_history" {
        return format!(
            "Welcome back, {}. You don't have any past orders with us yet. Would you like to check out today's specials or search the menu?",
            first_name
        );
    }

    if out.suggested_lines.is_empty() {
        return format!(
            "Welcome back, {}. None of your usual items are currently available today. Would you like to explore our latest menu?",
            first_name
        );
    }

    let mut parts = Vec::new();
    for l in out.suggested_lines.iter().take(3) {
        if l.quantity == 1 {
            parts.push(format!("the {}", l.item_name));
        } else {
            parts.push(format!("{} {}", l.quantity, l.item_name));
        }
    }
    if out.suggested_lines.len() > 3 {
        parts.push(format!("and {} more items", out.suggested_lines.len() - 3));
    }

    let items_spoken = parts.join(", ");
    let total_spoken = out
        .total
        .as_ref()
        .map(|t| paise_to_spoken_rupees(t.amount_minor))
        .unwrap_or_else(|| "listed prices".to_string());

    let mut speech = format!(
        "Welcome back, {}. Same as last time, {}? It's {} today.",
        first_name, items_spoken, total_spoken
    );

    if !out.price_changes.is_empty() {
        speech.push_str(" Note that menu prices have updated slightly since your last visit.");
    }

    if !out.conflicting_items.is_empty() {
        let conf_names: Vec<String> = out
            .conflicting_items
            .iter()
            .map(|c| {
                format!(
                    "the {} is listed as containing {}",
                    c.item_name,
                    c.conflicting_allergens.join(" and ")
                )
            })
            .collect();
        speech.push_str(&format!(" Please note: {}.", conf_names.join(", ")));
    }

    if !out.needs_allergy_reconfirmation.is_empty() {
        speech.push_str(" It has been a while since your allergy details were updated; please confirm they are still current.");
    }

    speech
}
