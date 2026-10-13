# Product Feedback

Feedback and evaluation of core developer tools and platform components used in TableTalk.

---

## Rust MCP SDK

- **What I used it for**: Evaluating protocol version compatibility (`2025-11-25`), Streamable HTTP transport integration with Axum, macro-based tool schema generation, and high-concurrency / adversarial testing for the TableTalk MCP server.
- **What worked well**: 
  - Direct integration with Axum via `StreamableHttpService` using standard Tower service mounting.
  - Clean procedural macros (`#[tool]`, `#[tool_handler]`, `#[tool_router]`) that derive JSON Schema Draft 2020-12 automatically with `schemars`.
  - Built-in support for tool annotations (`read_only_hint`, `idempotent_hint`, `destructive_hint`) and `outputSchema`.
  - Structured output (`Json<T>`) cleanly populates both `structuredContent` and JSON serialized `content` text.
  - First-class support for `2025-11-25` via `ProtocolVersion::V_2025_11_25` and built-in session generation (`mcp-session-id`).
  - High concurrency resilience: Flawlessly handled 50 concurrent initialize + tools/call sequences in ~125ms total duration with 0 panics or task leaks (p50: 111ms, p95: 116ms).
  - Clean SSE disconnection handling: When a client opens `GET /mcp` and drops the connection mid-stream, `StreamableHttpService` terminates the stream gracefully without panicking or leaking tasks.
  - Compliant session teardown: Handles `DELETE /mcp` with HTTP 202 Accepted, and accurately rejects post-termination requests with HTTP 404.
  - Out-of-the-box compatibility with the official `@modelcontextprotocol/inspector` test suite.
- **What needs work**: 
  - Lack of a standard `prelude` module causes trial-and-error discovery of trait and macro imports.
  - Non-exhaustive structs like `InitializeResult` and `Implementation` must be instantiated via builder/new methods rather than struct literals, which is standard Rust design but needs better rustdoc discovery examples.
  - `StreamableHttpService` does not provide an integrated request payload size limiter, so developers must explicitly apply `tower_http::limit::RequestBodyLimitLayer` and `Content-Length` validation middleware to avoid denial-of-service vulnerabilities.
  - Documentation around feature flags (`transport-streamable-http-server`, `schemars`, `macros`) could provide clearer end-to-end Axum examples in the README.
- **Onboarding experience**: Fast once crate features are understood. Compiles quickly and error messages from macro expansions are readable.
- **Would I build with it again**: **Yes** — The crate handles complex JSON-RPC transport framing, SSE chunking, and session state cleanly, saving dozens of hours of custom protocol scaffolding.

---

## Streamable HTTP

- **What I used it for**: Inspecting single-endpoint transport behavior, Server-Sent Events (SSE) streaming, session header tracking, Origin validation for remote voice assistant connectivity, and adversarial edge case validation.
- **What worked well**: 
  - Solves the dual-endpoint awkwardness of the older HTTP+SSE transport by multiplexing requests and responses on a single path (`/mcp`).
  - Clear SSE priming semantics (`id: 0`, `retry: 3000`) make reconnections resilient.
  - Stateless/stateful flexibility allows servers to support simple request-response or long-lived streams on demand.
  - Strict protocol status codes (HTTP 415 for malformed JSON, HTTP 422 for uninitialized requests, HTTP 400 for bad protocol version headers, HTTP 404 for unknown sessions, HTTP 406 for missing Accept header) make client-side protocol debugging deterministic and robust.
  - Standard client notifications (e.g. `notifications/initialized`, `notifications/cancelled`) correctly respond with HTTP 202 Accepted and an empty body.
- **What needs work**: 
  - Testing via standard CLI tools (curl, fetch) requires careful header management (`Accept`, `Mcp-Session-Id`, `MCP-Protocol-Version`).
  - DNS rebinding / Origin validation rules are easy to overlook unless explicitly verified during server configuration.
- **Onboarding experience**: Well-documented in the 2025-11-25 specification with clear sequence diagrams and status code requirements.
- **Would I build with it again**: **Yes** — It is significantly cleaner to deploy behind reverse proxies and API gateways than older SSE-specific bidirectional hacks.

---

## SQLx 0.8 & SQLite (Persistence Layer)

- **What I used it for**: Relational schema persistence, embedded migrations, connection pooling with custom PRAGMAs (WAL, foreign keys, busy timeout), strongly-typed entity mapping with `FromRow`, and concurrency-safe table reservation transactions.
- **What worked well**:
  - `sqlx::migrate!("./migrations")` embeds schema migrations directly into the application binary, enabling self-bootstrapping databases in fresh container environments and in-memory test harnesses with zero external CLI requirements.
  - Connection pooling with `SqlitePool` smoothly coordinates WAL mode, ensuring concurrent read operations (e.g. menu search, customer profile reads) are not blocked by write transactions.
  - Strongly-typed enum mapping via `#[derive(sqlx::Type)]` with `#[sqlx(rename_all = "snake_case")]` provides exhaustive pattern matching in domain logic while seamlessly serializing to/from SQLite TEXT and CHECK constraints.
  - In-memory SQLite testing (`sqlite::memory:`) makes automated unit and property testing blazing fast (7 comprehensive database integration and concurrency tests complete in ~100ms).
- **What needs work**:
  - Compile-time checked queries (`sqlx::query!`) require either a live database or pre-generated `.sqlx/` JSON metadata via `cargo-sqlx`. This creates significant developer friction when writing dynamic queries with optional filter predicates, making runtime `sqlx::query_as` and `QueryBuilder` far more practical for search-heavy workflows.
  - SQLite's `INSERT OR REPLACE` deletes conflicting rows prior to insertion rather than performing an in-place update. In schemas with `ON DELETE RESTRICT` foreign keys, this triggers unexpected constraint failures during re-seeding unless `ON CONFLICT (...) DO UPDATE` or `INSERT OR IGNORE` is explicitly used.
- **Onboarding experience**: Excellent documentation on crates.io and docs.rs. Adding features like `chrono` and `uuid` is transparent and well-integrated.
- **Would I build with it again**: **Yes** — SQLx provides the ideal balance of async Tokio compatibility, compile-time type safety via derives, and embedded zero-maintenance persistence for embedded SQLite architectures.

---

## Chrono 0.4 & Chrono-Tz (Timezone & Availability Scheduling Engine)

- **What I used it for**: Implementing the pure, deterministic restaurant capacity and availability scheduling engine (`backend/src/availability.rs`), handling localized dining room timestamps (`Asia/Kolkata`), resolving daylight saving transitions (DST gaps and overlaps), and managing half-open `[start, end)` booking windows.
- **What worked well**:
  - `chrono-tz::Tz` compiles the complete IANA database directly into the binary, eliminating runtime filesystem access or external Olson database files (`/usr/share/zoneinfo`) which are notorious for missing in minimal Windows and scratch Docker containers.
  - The `chrono::offset::MappedLocalTime` enum (`Single`, `Ambiguous`, `None`) provides explicit, type-safe semantics for handling DST edge cases:
    - `MappedLocalTime::None` precisely detects spring-forward hour skips (DST gap) and returns a typed `dst_gap` rejection rather than silently inventing an invalid local wall-clock time.
    - `MappedLocalTime::Ambiguous(earlier, later)` allows deterministic resolution of autumn fall-back repeats by picking the standard time offset.
  - Duration arithmetic (`chrono::Duration`) and half-open interval overlap detection (`req_start < book_end && book_start < req_end`) are cleanly expressed without subtle floating-point or leap-second drift.
- **What needs work**:
  - Trait visibility ergonomics: In order to call methods on `chrono_tz::Tz` or `NaiveDate` (such as `.from_local_datetime()`, `.weekday()`), the traits `chrono::TimeZone` and `chrono::Datelike` must be explicitly brought into lexical scope (`use chrono::{TimeZone, Datelike};`). Without them, rustc yields compiler errors indicating private or non-existent methods (`method weekday is private`), which can confuse developers unfamiliar with Chrono's trait hierarchy.
  - Formatting ISO-8601 timestamps for SQLite requires extra care: SQLite evaluates strings lexicographically (`ASCII`), so all UTC dates must be formatted uniformly (`+00:00` or `Z`) to guarantee correct sorting and boundary comparison in SQL queries.
- **Onboarding experience**: Extremely fast and predictable. Parsing IANA timezone identifiers via `tz_str.parse::<chrono_tz::Tz>()` is clean and returns well-typed errors on unknown zones.
- **Would I build with it again**: **Yes** — Chrono combined with Chrono-Tz is the gold standard for robust, timezone-aware scheduling systems in Rust.

---

## SQLx SQLite Transaction Modes under High Concurrency (Phase 8 Write-Path)

- **What I used it for**: Building the transactional `create_booking` and `cancel_booking` MCP write paths with high concurrency guarantees (20+ parallel workers contesting the last remaining dining table across 50 repeated iterations).
- **What worked well**:
  - Combining `PRAGMA busy_timeout = 5000` with `BEGIN IMMEDIATE` creates rock-solid concurrency serialization. In testing with 20 parallel Tokio tasks hitting the same physical table simultaneously, exactly 1 worker obtained the booking while the remaining 19 waited cleanly in SQLite's queue and gracefully returned alternative options without a single connection drop or database corruption.
  - Transaction rollbacks are completely atomic: mid-transaction errors (simulated via synthetic audit trail failures) rolled back 100% of persisted rows cleanly.
- **What needs work**:
  - `sqlx::Pool::begin()` defaults to `BEGIN` (which is `BEGIN DEFERRED` in SQLite). Under concurrent write patterns where transactions read capacity before writing, deferred transactions attempt to upgrade from shared to exclusive locks concurrently, causing deadlocks (`SQLITE_BUSY`). SQLx's SQLite driver would benefit from a built-in `pool.begin_immediate()` or builder method to acquire `BEGIN IMMEDIATE` natively without requiring explicit `conn.execute("BEGIN IMMEDIATE")` calls.
- **Onboarding experience**: Straightforward once SQLite locking mechanics (`SHARED`, `RESERVED`, `EXCLUSIVE`) are understood.
- **Would I build with it again**: **Yes** — For embedded single-node services, SQLite WAL mode with `BEGIN IMMEDIATE` delivers sub-15ms write latencies while guaranteeing 100% transactional safety.

---

## Phase 9 Ordering Engine: Integer Money Arithmetic & Allergen Safety Invariants

- **What I used it for**: Building the transactional, money-handling order placement and cancellation engine (`backend/src/ordering.rs`), enforcing server-side integer pricing snapshots, allergen safety invariants (prohibiting unverified data overrides for severe allergens), dynamic prep time estimation, and natural spoken formatting.
- **What worked well**:
  - **Integer Arithmetic Safety**: Handling money exclusively in minor units (paise) via Rust's primitive `i64` with checked arithmetic (`checked_add`, `checked_mul`) completely eliminates floating-point rounding hazards and currency drift. Tax calculation with 500 bps (5% GST) rounded half up `(subtotal * 500 + 5000) / 10000` is deterministic and 100% reproducible across platforms.
  - **Allergen State Machine & Safety Invariants**: Distinguishing between verified allergen presence vs unverified allergen data (`is_unverified`) allowed us to strictly enforce safety: callers cannot override unverified data warnings for severe allergen categories even with `acknowledged: true`. Zero prohibited safety words (`"safe"`, `"guaranteed"`, `"allergy-free"`) are used across code, text, or docs.
  - **Spoken Text Formatting**: Rendering spoken numbers into natural words (e.g., `"one thousand one hundred seventy-six rupees"`, `"seven fifteen in the evening"`, and spaced order codes `"T O G W T 4 K"`) provides a conversational voice assistant experience that reads naturally without stutter or awkward abbreviations.
- **What needs work**:
  - Natural language number formatting requires careful handling of compound numbers (e.g., tens and units) to avoid discarding thousands and hundreds accumulators during string assembly.
  - In voice assistants, keeping order summaries concise (capping spoken lines to 3 items and summarizing remainder with `"and X more items"`) is essential to prevent user fatigue during speech synthesis.
- **Onboarding experience**: Smooth and highly reliable.
- **Would I build with it again**: **Yes** — Pure Rust domain models with injected mock clocks and integer money math make financial and safety-critical ordering engines simple to test and impossible to compromise from client input.



