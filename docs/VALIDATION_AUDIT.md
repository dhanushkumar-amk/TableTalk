# Input Validation & Error Handling Audit (Phase 14)

## 1. Executive Summary

In Phase 14 of the TableTalk project, all 24 Model Context Protocol (MCP) tools underwent a comprehensive input validation and error-handling pass. The system was hardened against malformed, malicious, or out-of-bounds inputs without altering successful responses, adding new tools, or weakening any existing test assertions.

All 24 tools now adhere to a unified validation and error architecture defined in `backend/src/validation.rs`.

---

## 2. Core Policies

### 2.1 Unknown Extra Fields Policy
* **Decision**: **Ignore** unknown extra fields across all 24 tools.
* **Rationale**: LLM callers (e.g., Alexa+, Claude, GPT-4) frequently include extraneous keys (such as `thinking`, `context`, or speculative metadata). In accordance with Postel's Law (*"Be liberal in what you accept, and conservative in what you send"*), discarding unexpected keys ensures robust tool invocation while preventing spurious schema failures.
* **Implementation**: Standard Serde deserialization without `deny_unknown_fields`.

### 2.2 String Sanitization & Control Characters
* **Trimming**: All string inputs have leading and trailing whitespace stripped.
* **Control Characters**: Null bytes (`\x00`), ASCII control characters (`\x01`..`\x1F` excluding standard formatting), and ANSI escape sequences (`\x1B[...]`) are stripped out before validation and repository queries.
* **Length Caps**: Every string field has an explicit maximum character bound (e.g., guest names capped at 100 characters, tokens at 128 characters, notes at 1,000 characters).

### 2.3 Strict Date & Time Parsing
* **Dates**: Strict ISO-8601 calendar date `YYYY-MM-DD` validated via `chrono::NaiveDate::parse_from_str`. Ambiguous formats (e.g., `MM/DD/YYYY` or `DD-MM-YYYY`) are rejected with clear error messages naming the required format.
* **Times**: Strict 24-hour `HH:MM` validated via `chrono::NaiveTime::parse_from_str`. 12-hour AM/PM formats or invalid clock times (e.g. `25:99`) are rejected with actionable guidance.

### 2.4 Numeric & Array Bounds
* **Integer/Float bounds**: Min and max ranges enforced prior to SQL queries (e.g. party sizes `1..=20`, search limit `1..=10`, price caps `0.0..=100000.0`).
* **Array sizes**: Bounded between 1 and 20 items (e.g. order line items, preference lists) to prevent memory exhaustion.

### 2.5 Error Model Hierarchy
1. **Tool Execution Errors (`isError: true`)**:
   - Triggered exclusively on invalid arguments, boundary violations, or invalid date/time formats.
   - Returns a structured message naming the invalid field and the valid format or bounds so the LLM can self-correct.
2. **Business Outcomes (`isError: false`)**:
   - Domain outcomes (`closed`, `fully_booked`, `not_found`, `conflict`, `limit_exceeded`) remain normal tool responses containing standard structured envelopes and spoken text.
3. **Internal Errors**:
   - Never leak SQL queries, SQLite error codes, table names, file paths, or backtraces.
   - Logs the real error with a unique UUID Request ID via `tracing::error!`.
   - Returns a sanitized, generic user message referencing the Request ID for auditing.
4. **Protocol Errors**:
   - Malformed JSON-RPC or transport-level violations return standard JSON-RPC error codes (-32700, -32600, -32601, -32602).

---

## 3. Tool Audit Table (All 24 Tools)

| Tool Name | Input Fields | Required / Optional | Constraints & Types | Baseline Findings | Resolution & Fix |
|:---|:---|:---|:---|:---|:---|
| `ping_restaurant` | `name` | Optional | String, max 100 chars | Free string accepted raw control characters | Sanitized with `sanitize_optional_string`, trimmed, max 100 chars |
| `search_menu` | `query`, `category`, `dietary`, `exclude_allergens`, `max_price`, `available_only`, `limit`, `sort_by`, `profile_token`, `use_profile` | All Optional | Query/category max 100-200 chars; max_price 0..100k; limit 1..10; arrays max 20 | Ad-hoc limits; raw control characters in query | Unified `validation.rs` range checks, string sanitization, and array bounds |
| `get_menu_item` | `item_id`, `name` | At least 1 required | `item_id`: u64 > 0; `name`: string max 100 chars | Raw unvalidated string lookup | String sanitized, `item_id` bounds validated, generic internal errors |
| `check_availability` | `date`, `time`, `party_size`, `duration_minutes`, `alternatives_limit` | `date`, `time`, `party_size` required | Date `YYYY-MM-DD`, Time `HH:MM`, party_size 1..20, duration 15..360, alternatives 0..10 | Lenient parsing could panic on malformed time | Strict `parse_iso_date` & `parse_time_24h`; party size bounded to 1..20 |
| `create_booking` | `date`, `time`, `party_size`, `guest_name`, `guest_phone`, `guest_email`, `area`, `special_requests`, `idempotency_key` | `date`, `time`, `party_size`, `guest_name` required | Party size 1..20; name max 100; phone max 30; email max 100; area enum (`indoor`, `patio`, `bar`); special_requests max 500; key max 128 | Extra fields unhandled; DB errors leaked SQLite error codes | Strict validation on all fields; safe generic DB error handling |
| `get_booking` | `confirmation_code`, `guest_name` | `confirmation_code` required, `guest_name` optional | Code max 32 chars (normalized uppercase); name max 100 chars | Unchecked raw string confirmation code | Normalized via `normalize_code`, sanitized guest name, internal error masking |
| `cancel_booking` | `confirmation_code`, `guest_name`, `reason` | `confirmation_code` required; others optional | Code max 32 chars; name max 100; reason max 500 | Raw reason strings unescaped | Code normalized, strings sanitized, database errors masked |
| `place_order` | `items`, `guest_name`, `guest_phone`, `guest_email`, `order_type`, `table_id`, `notes`, `idempotency_key`, `stated_allergens`, `allergen_override_acknowledged`, `profile_token`, `use_profile` | `items` required (1..20 items) | Item quantity 1..50; name max 100; phone max 30; email max 100; notes max 500; key max 128; allergens array max 20 | Array unbounded; items quantity unbounded | `validate_array_bounds` (1..20), quantity bounded (1..50), sanitized strings |
| `get_order` | `order_code`, `guest_name` | `order_code` required | Code max 32 chars (normalized uppercase); guest name max 100 chars | Unvalidated order code string | Normalized code, sanitized guest name, internal error masking |
| `cancel_order` | `order_code`, `guest_name`, `reason` | `order_code` required | Code max 32 chars; guest name max 100; reason max 500 | Unsanitized reason; potential SQL error leak | Unified string sanitization and safe DB error handling |
| `enable_memory` | `guest_name`, `guest_phone`, `guest_email`, `consent_acknowledged`, `initial_preferences` | `guest_name` and `consent_acknowledged` required | Name max 100; phone max 30; email max 100; preferences max 20 | Array of initial preferences unbounded | Array bounded to max 20, strings sanitized, strict consent boolean check |
| `get_profile` | `profile_token` | Required | Token string max 128 chars | Raw unvalidated token string | Sanitized token, constant-time rate limiting preserved, safe DB errors |
| `remember_preference` | `profile_token`, `kind`, `value`, `severity` | `profile_token`, `kind`, `value` required | Token max 128; kind enum; value max 200; severity enum | Raw control characters in preference values | Control characters stripped, length checked, canonical register validated |
| `forget_preference` | `profile_token`, `kind`, `value` | All required | Token max 128; kind enum; value max 200 | Raw strings without bounds | Sanitized with length caps, safe internal error masking |
| `get_usual_order` | `profile_token` | Required | Token string max 128 chars | Raw token string | Sanitized token, safe internal DB error masking |
| `rotate_profile_token` | `profile_token` | Required | Token string max 128 chars | Raw token string | Sanitized token, safe internal DB error masking |
| `forget_me` | `profile_token`, `confirm` | `profile_token` required | Token max 128; confirm boolean | Boolean defaults unhandled | Strict validation of token and explicit confirmation |
| `negotiate_booking` | `party_size`, `target_date`, `target_time`, `area`, `guest_name`, `tolerance_minutes`, `max_rounds`, `conversation_history` | `party_size`, `target_date`, `target_time` required | Party size 1..20; date `YYYY-MM-DD`; time `HH:MM`; area enum; name max 100; tolerance 0..240; max_rounds 1..10 | Lenient date/time parsing; raw names | Strict ISO date/time validation, sanitized name, bounds on tolerance and rounds |
| `accept_offer` | `offer_token`, `guest_name`, `guest_phone`, `guest_email`, `special_requests` | `offer_token` required | Token max 128; name max 100; phone max 30; email max 100; special_requests max 500 | Unchecked RFC3339 datetime parsing with `.unwrap()` | Replaced `.unwrap()` with safe fallbacks; sanitized all contact strings |
| `release_offer` | `offer_token`, `guest_name` | `offer_token` required | Token max 128; name max 100 | Raw unvalidated token string | Sanitized token and guest name, safe internal DB errors |
| `join_waitlist` | `guest_name`, `party_size`, `date`, `target_time`, `area`, `guest_phone`, `guest_email`, `notes` | `guest_name`, `party_size`, `date`, `target_time` required | Name max 100; party size 1..20; date `YYYY-MM-DD`; time `HH:MM`; area enum; notes max 500 | Party size unbounded; date parsing unvalidated | Strict ISO date/time validation, bounded party size, sanitized strings |
| `get_waitlist_status` | `waitlist_code`, `guest_name` | `waitlist_code` required | Code max 32 chars; guest name max 100 chars | Unchecked `.unwrap()` on RFC3339 datetime parsing | Replaced `.unwrap()` with safe fallbacks; normalized waitlist code |
| `leave_waitlist` | `waitlist_code`, `guest_name` | `waitlist_code` required | Code max 32 chars; guest name max 100 chars | Unvalidated code string | Normalized code, sanitized guest name, safe DB error masking |
| `get_deposit_status` | `confirmation_code`, `guest_name` | `confirmation_code` required | Code max 32 chars; guest name max 100 chars | Raw confirmation code string | Normalized code, sanitized guest name, safe DB error masking |
| `refresh_payment_session` | `confirmation_code`, `guest_name` | `confirmation_code` required | Code max 32 chars; guest name max 100 chars | Direct `.unwrap()` on booking query result (`booking.unwrap()`) | Replaced `booking.unwrap()` with pattern match returning `not_found` business status |

---

## 4. Panic Safety & Robustness Audit

A codebase-wide grep was performed across all non-test Rust source files in `backend/src/` for `unwrap()`, `expect()`, and unchecked indexing reachable by client input:

1. **`backend/src/tools/refresh_payment_session.rs:127`**:
   * *Issue*: `let booking = booking.unwrap();` assumed a booking existed after a conditional check. If the state changed concurrently, this would panic the worker thread.
   * *Fix*: Replaced with safe pattern matching returning a structured `not_found` tool result if absent.
2. **`backend/src/tools/get_waitlist_status.rs:162-174`**:
   * *Issue*: `DateTime::parse_from_rfc3339(&entry.created_at).unwrap()` panicked on unexpected timestamp formatting.
   * *Fix*: Replaced with `.map(...).unwrap_or_else(...)` defaulting safely to elapsed time 0.
3. **`backend/src/tools/accept_offer.rs:391-396`**:
   * *Issue*: `DateTime::parse_from_rfc3339(&hold.created_at).unwrap()` panicked on malformed database records.
   * *Fix*: Replaced with `.map(...).unwrap_or_else(...)` with fallback to system clock.
4. **`backend/src/negotiation.rs:300`**:
   * *Issue*: Unchecked slice indexing `parts[0], parts[1], parts[2]` when generating spoken suggestion phrases.
   * *Fix*: Replaced with slice pattern matching (`[]`, `[first]`, `[first, second]`, `[first, second, third, ..]`).
5. **HTTP Panic Guard Layer**:
   * *Layer Added*: `tower_http::catch_panic::CatchPanicLayer::custom(handle_panic)` added to the top-level Axum router in `backend/src/http.rs`.
   * *Behavior*: If any handler or tool panics unexpectedly, the layer catches the unwind, logs the error with a unique UUID request ID via `tracing::error!`, and returns a clean HTTP 500 (`Internal Server Error: An unexpected error occurred. Please try again.`) without crashing the process or disconnecting other clients.

---

## 5. Rate Limiting and Size Limits

1. **Rate Limiting Middleware (`backend/src/rate_limit.rs`)**:
   * Sliding-window per-IP rate limiter protecting `/mcp` and `/checkout/:session_id` routes.
   * Configurable via `RATE_LIMIT_PER_MINUTE` environment variable (default: **600 requests per minute** per client IP).
   * Exceeding the threshold returns HTTP `429 Too Many Requests` with a valid `Retry-After: <seconds>` header.
   * Unthrottled Endpoints: The `/health` health-check endpoint bypasses rate limiting to prevent orchestrator false-positives.
2. **Payload Size Limit**:
   * `RequestBodyLimitLayer::new(2 * 1024 * 1024)` (2 MB) enforced via Tower HTTP middleware.
   * `validate_body_size` middleware validates the `Content-Length` header upfront, returning HTTP `413 Payload Too Large` before buffering.
3. **Request Timeout**:
   * `TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30))` terminates hanging requests after 30 seconds.
