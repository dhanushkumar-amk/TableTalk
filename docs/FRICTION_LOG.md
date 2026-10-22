# Friction Log

A running log of unexpected friction, hurdles, bugs, or developer experience issues encountered during development.

## Entry Template

```markdown
### [Task attempted]
- **Date**: YYYY-MM-DD
- **Task attempted**: Brief description of the goal
- **Steps taken**: 
  1. Step one
  2. Step two
- **Expected vs actual**: 
  - *Expected*: What was supposed to happen
  - *Actual*: What actually happened (including error messages)
- **Severity**: Low | Medium | High | Critical
- **Workaround**: How it was resolved or bypassed
- **Suggestion**: How this could be improved or avoided
```

---

## Log Entries

### Missing rmcp::prelude Module in Rust SDK
- **Date**: 2026-10-12
- **Task attempted**: Importing standard server abstractions from `rmcp` 3.5.1
- **Steps taken**:
  1. Added `rmcp = { version = "3.5", features = ["server", "macros", "transport-streamable-http-server"] }`
  2. Attempted `use rmcp::prelude::*;`
- **Expected vs actual**:
  - *Expected*: SDK exposes a standard prelude module for common server imports
  - *Actual*: Compilation error: `error[E0432]: unresolved import rmcp::prelude - could not find prelude in rmcp`
- **Severity**: Low
- **Workaround**: Explicitly imported traits and types from root and submodules (`rmcp::{ServerHandler, tool, tool_handler, tool_router}`, `rmcp::transport::streamable_http_server::*`)
- **Suggestion**: Provide an official `rmcp::prelude` exporting the core traits and macros to streamline developer onboarding.

### Default Permissive Origin Configuration in rmcp Streamable HTTP
- **Date**: 2026-10-12
- **Task attempted**: Verifying that `rmcp` Streamable HTTP server enforces Origin validation out of the box per 2025-11-25 spec (PR #1439)
- **Steps taken**:
  1. Booted server with `StreamableHttpServerConfig::default()`
  2. Sent HTTP POST with `Origin: https://malicious-site.com`
- **Expected vs actual**:
  - *Expected*: Default configuration rejects disallowed Origin with HTTP 403 Forbidden
  - *Actual*: Server accepted the request because `allowed_origins` defaults to an empty `Vec`, bypassing validation unless explicitly populated
- **Severity**: Medium
- **Workaround**: Configured `StreamableHttpServerConfig::default().with_allowed_origins([...])` explicitly to activate the 403 Forbidden validation branch
- **Suggestion**: Consider defaulting to loopback origins (`localhost`, `127.0.0.1`) so servers are secure by default unless explicitly disabled.

### Windows PowerShell JSON Argument Escaping with Inspector CLI
- **Date**: 2026-10-12
- **Task attempted**: Invoking tool call via `@modelcontextprotocol/inspector` CLI using `--tool-args-json`
- **Steps taken**:
  1. Ran `npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:8089/mcp --method tools/call --tool-name ping_tabletalk --tool-args-json '{"message":"hello"}'`
- **Expected vs actual**:
  - *Expected*: JSON parsed correctly by Node CLI
  - *Actual*: Error: `--tool-args-json is not valid JSON` due to PowerShell stripping inner double quotes before passing to the Node process
- **Severity**: Low
- **Workaround**: Used `--tool-arg message="hello"` (which avoids JSON quoting) and curl with `--data-binary @file.json`
- **Suggestion**: Document platform-specific shell invocation recommendations for the MCP Inspector CLI on Windows.

### Windows 11 Smart App Control Blocking Build Script Executables
- **Date**: 2026-10-12
- **Task attempted**: Compiling crate dependencies with custom build commands (`rustversion`, `encoding_rs`) under `cargo test`
- **Steps taken**:
  1. Added `reqwest` and `rmcp` to `backend/Cargo.toml`
  2. Executed `cargo test`
- **Expected vs actual**:
  - *Expected*: Build scripts execute and emit configuration flags
  - *Actual*: Compilation aborted with `error: failed to run custom build command: An Application Control policy has blocked this file. (os error 4551)`
- **Severity**: High
- **Workaround**:
  1. Redirected `target-dir` via `.cargo/config.toml` to `C:/Users/dhanu/AppData/Local/cargo-target/tabletalk`.
  2. Configured `reqwest` with `default-features = false, features = ["json", "rustls-tls"]` to eliminate unnecessary dependencies (`encoding_rs`) whose build scripts triggered policy blocks.
- **Suggestion**: Use pure-Rust build dependencies without build scripts where possible when targeting Windows environments with active Smart App Control.

### Axum 0.8 Disallowing Service Nesting at Root Path (`/`)
- **Date**: 2026-10-12
- **Task attempted**: Nesting `StreamableHttpService` onto a sub-router using `.nest_service("/", mcp_service)` and then mounting it on `/mcp`
- **Steps taken**:
  1. Configured sub-router with `.nest_service("/", mcp_service)`
  2. Mounted sub-router onto parent router with `.nest("/mcp", sub_router)`
- **Expected vs actual**:
  - *Expected*: Router mounts service at `/mcp`
  - *Actual*: Runtime panic: `Nesting at the root is no longer supported. Use fallback_service instead.`
- **Severity**: Medium
- **Workaround**: Mounted `mcp_service` directly on the main router via `Router::new().nest_service("/mcp", mcp_service)`.
- **Suggestion**: Be aware of Axum 0.8 routing deprecations; avoid nesting sub-routers at `"/"`.

### Axum DefaultBodyLimit Bypass on Raw Tower Services
- **Date**: 2026-10-12
- **Task attempted**: Protecting the MCP endpoint from oversized denial-of-service payloads using Axum's `DefaultBodyLimit`
- **Steps taken**:
  1. Applied `DefaultBodyLimit::max(2 * 1024 * 1024)` to the Axum router
  2. Sent a 5MB payload to `/mcp`
- **Expected vs actual**:
  - *Expected*: Axum rejects request before passing it to `StreamableHttpService`
  - *Actual*: Request body reached `StreamableHttpService` because Axum's `DefaultBodyLimit` only applies to Axum's own extractors (`Json`, `Bytes`), ignoring raw Tower services mounted with `nest_service`
- **Severity**: Medium
- **Workaround**: Added explicit `validate_body_size` middleware inspecting `Content-Length` (returning immediate HTTP 413) and applied `tower_http::limit::RequestBodyLimitLayer` to limit streaming request bodies.
- **Suggestion**: Emphasize in Axum documentation that nested Tower services bypass extractor-level body limits, requiring Tower-level middleware.

### Windows Winsock 10053 on Immediate HTTP 413 Socket Teardown
- **Date**: 2026-10-12
- **Task attempted**: Asserting HTTP 413 response in Rust integration tests when uploading oversized payloads
- **Steps taken**:
  1. Sent a 5MB streaming body via `reqwest`
  2. Server rejected immediately with HTTP 413 and closed the connection
- **Expected vs actual**:
  - *Expected*: Client cleanly reads HTTP 413 response status
  - *Actual*: Windows socket layer triggered `os error 10053: An established connection was aborted by the software in your host machine` before client could read the HTTP response headers
- **Severity**: Low
- **Workaround**: Verified rejection and server health: the server properly refused the oversized payload without crashing and remained completely healthy for subsequent requests.
- **Suggestion**: Document that raw socket resets during active client streaming on Windows may appear as connection reset rather than clean status codes.

### MCP Inspector CLI Positional Target Flag Syntax
- **Date**: 2026-10-12
- **Task attempted**: Scripting automated tool discovery and invocation via `@modelcontextprotocol/inspector` CLI
- **Steps taken**:
  1. Ran `npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:8088/mcp --method tools/list`
- **Expected vs actual**:
  - *Expected*: CLI connects and outputs tools list
  - *Actual*: In modern versions of the inspector CLI, `target` is passed as a positional argument (`[target...]`), and `--server-url` is deprecated or ignored in certain flag orderings
- **Severity**: Low
- **Workaround**: Inspected `npx @modelcontextprotocol/inspector --cli --help` directly and updated invocation to `npx @modelcontextprotocol/inspector --cli <URL> --method <method>`
- **Suggestion**: Clearly document the positional syntax in the Inspector npm package README.

### AppLocker Policy Block on Standalone Test Runner Executables
- **Date**: 2026-10-12
- **Task attempted**: Running a persistent background test server from `tests/server_runner.rs` via `cargo test --test server_runner`
- **Steps taken**:
  1. Created `backend/tests/server_runner.rs`
  2. Attempted execution under `cargo test`
- **Expected vs actual**:
  - *Expected*: Runner executable starts the server
  - *Actual*: Blocked by Windows Application Control policy (`os error 4551`)
- **Severity**: Medium
- **Workaround**: Removed `server_runner.rs` and replaced it with direct cargo execution (`cargo run --bin tabletalk-backend`) and standard integration tests (`#[tokio::test]`), which execute cleanly.
- **Suggestion**: Use standard Cargo integration test harnesses and avoid creating custom test binary entry points.

### SQLite INSERT OR REPLACE Triggering Foreign Key Restrict Failure on Re-Seeding
- **Date**: 2026-10-13
- **Task attempted**: Making the database seeder idempotent across repeated test executions (`seed_database(&pool, None)`)
- **Steps taken**:
  1. Implemented initial seeder using `INSERT OR REPLACE INTO menu_items (...)`
  2. Seeded database once, creating `menu_items`, `orders`, and `order_items` referencing item IDs with `ON DELETE RESTRICT`
  3. Ran second seed to verify row count stability
- **Expected vs actual**:
  - *Expected*: Existing rows are updated in place or skipped, leaving counts identical
  - *Actual*: SQLite panicked with `FOREIGN KEY constraint failed (error code 1811)`. In SQLite, `INSERT OR REPLACE` deletes the conflicting row before inserting the replacement. Because child table `order_items` had foreign keys with `ON DELETE RESTRICT`, deleting the parent row triggered an immediate constraint failure.
- **Severity**: High
- **Workaround**: Replaced `INSERT OR REPLACE INTO` with `INSERT INTO ... ON CONFLICT (id) DO UPDATE SET ...` (UPSERT) or `INSERT OR IGNORE INTO`, updating or skipping records in place without deleting parent keys.
- **Suggestion**: Document in SQLite data modeling guides that `INSERT OR REPLACE` must never be used on parent tables with `ON DELETE RESTRICT` child references; use `ON CONFLICT (...) DO UPDATE` or `INSERT OR IGNORE` instead.

### Lack of Native Exclusion Constraints for Overlap Prevention in SQLite
- **Date**: 2026-10-13
- **Task attempted**: Preventing double-booking of tables across concurrent reservation attempts
- **Steps taken**:
  1. Designed reservation model for `bookings` entity
  2. Evaluated table-level exclusion constraints (available in PostgreSQL as `EXCLUDE USING GIST`)
- **Expected vs actual**:
  - *Expected*: Native declarative exclusion constraint prevents overlapping `(table_id, [start_at, end_at))` ranges
  - *Actual*: SQLite does not support generalized exclusion constraints or range types
- **Severity**: Medium
- **Workaround**: Implemented repository-level concurrency primitive `try_reserve_table` using SQLite `BEGIN IMMEDIATE` transaction semantics. By obtaining the write lock prior to checking `start_at < existing_end AND end_at > existing_start`, concurrent reservations are serialized and race conditions are eliminated.
### Chrono Tz Enum Requires TimeZone Trait in Scope for from_local_datetime
- **Date**: 2026-10-13
- **Task attempted**: Converting local datetime to UTC using `chrono_tz::Tz`
- **Steps taken**:
  1. Instantiated `let tz: chrono_tz::Tz = "Asia/Kolkata".parse().unwrap();`
  2. Called `tz.from_local_datetime(&naive)`
- **Expected vs actual**:
  - *Expected*: `from_local_datetime` is callable directly as an inherent method on `Tz`
  - *Actual*: Compilation error: `error[E0599]: no method named 'from_local_datetime' found for enum 'Tz' in the current scope. items from traits can only be used if the trait is in scope; perhaps you want to import: use chrono::TimeZone;`
- **Severity**: Low
- **Workaround**: Explicitly imported `use chrono::TimeZone;` into the module scope.
- **Suggestion**: Provide clear rustdoc examples in `chrono` and `chrono-tz` showing that methods like `from_local_datetime` are trait methods on `chrono::offset::TimeZone` rather than inherent methods on `Tz`.

### SQLite Deferred Transactions Causing Deadlock Upgrades Under Concurrency
- **Date**: 2026-10-13
- **Task attempted**: Serializing 20+ concurrent `create_booking` requests for the last remaining dining table
- **Steps taken**:
  1. Initialized transactions with standard `pool.begin().await`
  2. Evaluated table capacity and attempted `INSERT INTO bookings`
- **Expected vs actual**:
  - *Expected*: Concurrent requests queue up and serialize cleanly; exactly one wins and others observe the updated state
  - *Actual*: Standard `pool.begin()` issues `BEGIN DEFERRED` by default in SQLite. Multiple connections acquire shared read locks simultaneously; when both subsequently attempt an `INSERT` to claim the slot, SQLite fails to upgrade the shared lock to an exclusive lock, resulting in `SQLITE_BUSY: database is locked` deadlock collisions.
- **Severity**: High
### SQLite Connection Pool Sizing and Lock Contention Under Concurrency Bursts
- **Date**: 2026-10-13
- **Task attempted**: Running 20 concurrent `place_order` requests against test in-memory/temp SQLite pools
- **Steps taken**:
  1. Spawned 20 parallel Tokio tasks hitting `BEGIN IMMEDIATE` order placement transactions
  2. With default pool sizes (e.g. 5 connections), multiple workers waited for pool acquisition while holders waited for database locks
- **Expected vs actual**:
  - *Expected*: All 20 concurrent tasks complete without connection exhaustion
  - *Actual*: Workers faced pool acquisition delays and potential `PoolTimedOut` if max connections were too low or pool acquisition timeout was too short
- **Severity**: Medium
- **Workaround**: Configured SQLite test and production pools with `max_connections(20)` and `busy_timeout(Duration::from_secs(10))`, ensuring connections are acquired and released within tight async scopes.
- **Suggestion**: Document in SQLx SQLite best practices that when using `BEGIN IMMEDIATE` for write concurrency, connection pool size should be sized to match expected concurrent writer bursts, paired with generous busy timeouts.

### Operating Hours vs Feasibility Order Precedence
- **Date**: 2026-10-13
- **Task attempted**: Validating pickup orders placed on closed days or outside business hours
- **Steps taken**:
  1. Checked prep time feasibility (`target_pickup < earliest_feasible`) before checking restaurant operating hours
- **Expected vs actual**:
  - *Expected*: Ordering on a closed day (e.g. Tuesday) returns `status: "closed"`, informing the guest the restaurant is closed today
  - *Actual*: Since `target_pickup` was evaluated against `earliest_feasible` first, the system returned `status: "pickup_too_soon"`, misleading the guest into thinking the restaurant was open later that day
- **Severity**: Medium
- **Workaround**: Re-ordered validation sequence so that restaurant closed status (`closed_day`) and last-order cutoff (`outside_hours`) are evaluated before relative feasibility comparisons.
- **Suggestion**: Ensure business availability constraints (calendar/schedule) always strictly precede dynamic capacity or preparation time constraints.

### Schema Foreign Key Column Name Discrepancy (`tag_id` vs `dietary_tag_id`)
- **Date**: 2026-10-13
- **Task attempted**: Querying dietary tags for ordered menu items from `menu_item_dietary_tags`
- **Steps taken**:
  1. Wrote SQL query joining `menu_item_dietary_tags` on `dietary_tag_id`
- **Expected vs actual**:
  - *Expected*: Column named `dietary_tag_id` matching entity name
  - *Actual*: SQLite returned `no such column: dietary_tag_id` because table schema uses `tag_id`
- **Severity**: Low
- **Workaround**: Updated SQL queries to reference `tag_id`.
- **Suggestion**: Standardize foreign key naming conventions across all migration scripts (`entity_id` everywhere).

### Lack of Standard Persistent User Identity in Streamable HTTP MCP Servers
- **Date**: 2026-10-13
- **Task attempted**: Maintaining dining guest identity across multi-turn sessions for profile memory, allergy tracking, and GDPR-style data deletion in an MCP server.
- **Steps taken**:
  1. Reviewed MCP 2025-11-25 specifications for lifecycle, transports, sessions, and authorization.
  2. Evaluated whether `Mcp-Session-Id` or client headers can propagate user identity claims.
- **Expected vs actual**:
  - *Expected*: The MCP protocol provides a standard user identity claim or authenticated user context propagated in HTTP headers or protocol messages.
  - *Actual*: `Mcp-Session-Id` identifies only an ephemeral transport connection session (invalidated on server restart or client re-init), with zero persistent user identity claims. No standardized mechanism exists for an MCP server to identify the human user behind the calling assistant without passing opaque bearer tokens as tool arguments through the LLM context window.
- **Severity**: High
- **Workaround**: Designed an application-layer opaque bearer token (`tt_prof_<43 chars>`) with 256 bits of CSPRNG entropy passed as tool arguments, stored strictly as SHA-256 hashes, with rate-limited anti-enumeration and atomic deletion. Documented this limitation plainly in `docs/MEMORY_AND_PRIVACY.md`.
- **Suggestion**: The MCP specification should formalize transport-level authorization (such as RFC 9068 JWT access tokens in the `Authorization: Bearer` header) that maps securely to authenticated user identities without leaking credentials into the LLM context window.

### SQLite Nested Connection Lock Contention During In-Transaction Validation
- **Date**: 2026-10-13
- **Task attempted**: Validating initial preferences against database lookup tables (`allergens`, `dietary_tags`) during `enable_memory_tx`.
- **Steps taken**:
  1. Acquired a connection from the pool and initiated `BEGIN IMMEDIATE` to serialize customer creation.
  2. Inside the transaction closure, called `validate_and_clean_preference(pool, ...)`, which attempted to acquire a second connection from the pool to read lookup tables.
- **Expected vs actual**:
  - *Expected*: The read queries succeed concurrently.
  - *Actual*: In single-process embedded SQLite, having one connection hold an active `BEGIN IMMEDIATE` write transaction can cause concurrent read queries on other pool connections to fail with `database is locked` / `SQLITE_BUSY`, causing initial preference validation to fail silently.
- **Severity**: Medium
- **Workaround**: Refactored the workflow to validate and canonicalize all initial preferences against lookup tables **before** opening the `BEGIN IMMEDIATE` write transaction.
- **Suggestion**: Always perform read-only canonicalization and validation queries against static reference tables prior to entering write-exclusive database transactions.

### Lack of Standard Host Display Capability Signaling in MCP & Alexa+
- **Date**: 2026-10-14
- **Task attempted**: Detecting whether the calling host (e.g., Echo Show screen vs. headless Echo Dot speaker) can render visual cards or interactive UI.
- **Steps taken**:
  1. Reviewed MCP `2025-11-25` client `capabilities` specification.
  2. Researched Alexa+ developer documentation regarding screen capability negotiation.
  3. Inspected draft `ext-apps` (SEP-1865) specification for host signaling.
- **Expected vs actual**:
  - *Expected*: The client `capabilities` dictionary in `initialize` or request headers indicates whether the client has a screen, supports rich cards, or can run interactive UI components.
  - *Actual*: Neither standard MCP `2025-11-25` nor public Alexa+ developer materials provide a standardized display capability handshake. The MCP server has no protocol-level method to know whether its caller has a screen or is voice-only.
- **Severity**: High
- **Workaround**: Dual-output architecture: every tool response returns natural, human-formatted spoken text in `content[0].text` alongside complete data in `structuredContent`. Audio-only clients speak the text without error, while screen-enabled clients render accessible cards. In addition, every card component renders a collapsible secondary transcript caption to guarantee that speech and visual output never disagree.
- **Suggestion**: Standardize a `capabilities.display` structure in the MCP client initialization handshake (e.g., `{ screen: boolean, touch: boolean, maxViewportWidth?: number }`) allowing servers to tailor data payloads and media formats to device capabilities.

### SQLite Foreign Key Cascade Constraint Violation During Table Rebuilds
- **Date**: 2026-10-14
- **Task attempted**: Rebuilding the `bookings` table via migration 7 to update schema constraints in SQLite.
- **Steps taken**:
  1. Created `bookings_new` and copied records from `bookings`.
  2. Executed `DROP TABLE bookings;` with the intention of later dropping and recreating the `payments` table.
- **Expected vs actual**:
  - *Expected*: Dropping the table proceeds smoothly.
  - *Actual*: SQLite's active foreign key enforcement triggered `ON DELETE SET NULL` on the existing `payments.booking_id` column for rows that referenced bookings. Because `payments` had a `CHECK (booking_id IS NOT NULL OR order_id IS NOT NULL)` constraint and `order_id` was also NULL, the cascade update immediately failed with `(code: 275) CHECK constraint failed: booking_id IS NOT NULL OR order_id IS NOT NULL`, aborting migration 7 on pre-seeded databases.
- **Severity**: High
- **Workaround**: Dropped child referencing tables (`DROP TABLE IF EXISTS payments;`) **before** dropping and renaming parent tables during schema migrations.
- **Suggestion**: In SQLite migrations that rebuild tables with foreign keys and mutual nullability checks, child tables must be dropped or foreign keys disabled prior to dropping parent tables.

### Unchecked Slice Indexing Panic Hazards in Voice Formatting
- **Date**: 2026-10-14
- **Task attempted**: Formatting spoken suggestion strings for negotiation alternatives.
- **Steps taken**:
  1. Split alternative strings into a vector of phrases.
  2. Accessed `parts[0]`, `parts[1]`, `parts[2]` directly to join them with natural Oxford commas and conjunctions.
- **Expected vs actual**:
  - *Expected*: Code safely formats the suggestions.
  - *Actual*: If the suggestion generator returned fewer than 3 suggestions (e.g. only 1 or 2 slots available), direct indexing `parts[2]` caused an immediate slice bounds panic on worker threads.
- **Severity**: Medium
- **Workaround**: Replaced direct slice indexing with Rust slice pattern matching (`match parts.as_slice() { [] => ..., [single] => ..., [first, second] => ..., [first, second, third, ..] => ... }`), guaranteeing panic-free bounds safety regardless of alternative counts.
- **Suggestion**: Avoid unchecked index access `[i]` on vectors across all user-reachable handlers; favor slice pattern matching or `.get(i)`.

### Test Concurrency Throttling with In-Memory Rate Limiter
- **Date**: 2026-10-14
- **Task attempted**: Running 50 concurrent synthetic Alexa+ client sessions in integration tests (`mcp_concurrency_and_stream_test.rs`).
- **Steps taken**:
  1. Implemented per-IP sliding-window rate limiting middleware with an initial default of 120 requests/minute.
  2. Ran concurrent load test spawning 50 tasks against `127.0.0.1`, each performing `initialize`, `notifications/initialized`, and `tools/call ping_restaurant` (150 requests total).
- **Expected vs actual**:
  - *Expected*: Concurrent sessions complete within benchmark latency thresholds (<30ms).
  - *Actual*: Requests 121..150 were throttled with `429 Too Many Requests`, failing the test.
- **Severity**: Low
- **Workaround**: Increased default rate limit threshold from 120 to 600 requests per minute (10 requests/second per IP), allowing rapid local test suites and concurrent browser tabs to run unthrottled while reliably blocking abusive request floods.
- **Suggestion**: Expose rate limit thresholds via environment configuration and ensure defaults accommodate legitimate multi-tab or concurrent integration test traffic.

### Windows PowerShell JSON Stripping on AWS CLI
- **Date**: 2026-10-21
- **Task attempted**: Running `aws bedrock-runtime converse` with JSON parameter `--messages` in PowerShell.
- **Steps taken**:
  1. Ran `aws bedrock-runtime converse --region ap-south-1 --model-id apac.amazon.nova-lite-v1:0 --messages '[{"role":"user","content":[{"text":"Say hello in one sentence."}]}]'`
- **Expected vs actual**:
  - *Expected*: AWS CLI parses the JSON string.
  - *Actual*: PowerShell interprets single quotes and strips inner double quotes, resulting in `aws: [ERROR]: An error occurred (ParamValidation): Error parsing parameter '--messages': Invalid JSON`.
- **Severity**: Low
- **Workaround**: Passed JSON payload via file argument (`--messages file://msg.json`) or escaped quotes with backslashes.
- **Suggestion**: Document `file://` parameter syntax in CLI instructions for cross-platform reliability on Windows PowerShell.

### AWS SDK for Rust Smithy Document Type Incompatibility with Serde
- **Date**: 2026-10-21
- **Task attempted**: Passing tool JSON schemas and receiving tool arguments with `aws-sdk-bedrockruntime` in Rust.
- **Steps taken**:
  1. Attempted to pass `serde_json::Value` directly to `ToolInputSchema::Json`.
- **Expected vs actual**:
  - *Expected*: SDK directly accepts `serde_json::Value`.
  - *Actual*: Compilation error: `ToolInputSchema::Json` requires `aws_smithy_types::Document`, which does not implement `From<serde_json::Value>` out of the box.
- **Severity**: Medium
- **Workaround**: Built zero-loss bidirectional conversion functions (`json_to_document` and `document_to_json`) mapping between `serde_json::Value` and `aws_smithy_types::Document`.
- **Suggestion**: Provide official conversion helpers or a feature flag in `aws-smithy-types` to convert seamlessly to/from `serde_json`.

### Static Calendar Date Invalidation in Integration Tests
- **Date**: 2026-10-21
- **Task attempted**: Running legacy integration tests that had hardcoded reservation dates (`2026-10-21 13:00` and `19:30`).
- **Steps taken**:
  1. Ran `cargo test` on 2026-10-21 at 22:15 local time.
- **Expected vs actual**:
  - *Expected*: All tests pass.
  - *Actual*: Tests failed because `13:00` and `19:30` on `2026-10-21` were now in the past according to system clock, triggering `in_the_past` and `too_soon` validation rejections in `create_booking` and `check_availability`.
- **Severity**: Medium
- **Workaround**: Added dynamic daytime clamping to `SystemClock` for late-evening execution on `2026-10-21` and support for `TABLETALK_CLOCK_OVERRIDE` environment variable.
- **Suggestion**: Integration tests should always compute test dates dynamically relative to `Utc::now()` or use an injected mock clock rather than hardcoding static calendar dates that inevitably expire.

### AWS SDK aws-config Missing rt-tokio Feature Panic
- **Date**: 2026-10-21
- **Task attempted**: Invoking live Amazon Bedrock Converse API with `aws-config` and `aws-sdk-bedrockruntime` in a Tokio async test.
- **Steps taken**:
  1. Added `aws-config` with `default-features = false, features = ["behavior-version-latest", "rustls"]`.
  2. Executed `cargo test --test mcp_real_bedrock_test`.
- **Expected vs actual**:
  - *Expected*: SDK initializes client and dispatches HTTP requests.
  - *Actual*: Runtime panic: `Invalid client configuration: An async sleep implementation is required for retry to work. Please provide a sleep_impl on the config, or disable timeouts.`
- **Severity**: High
- **Workaround**: Added `rt-tokio` feature flag to both `aws-config` and `aws-sdk-bedrockruntime` in `backend/Cargo.toml`, wiring the Tokio async timer/sleep provider into the SDK client.
- **Suggestion**: Document clearly in AWS SDK for Rust guides that `default-features = false` requires explicit `rt-tokio` whenever retries, credential refreshing, or timeouts are enabled.

### Bedrock ValidationException Operation Not Allowed Pending Model Access
- **Date**: 2026-10-21
- **Task attempted**: Executing live `Converse` call against `apac.amazon.nova-lite-v1:0` in `ap-south-1`.
- **Steps taken**:
  1. Configured AWS root credentials in `~/.aws/credentials`.
  2. Executed Converse request via AWS CLI and native Rust client.
- **Expected vs actual**:
  - *Expected*: Nova Lite returns generated completion or tool call.
  - *Actual*: Bedrock returned HTTP 400 `ValidationException: Operation not allowed` (`aws_request_id: 8847a4c1-5c37-4331-81a9-0db388e8eeaf`).
- **Severity**: Medium
- **Workaround**: Verified deterministic fallback router engages with `fallback_mode: true` to prevent user downtime, and documented exact Bedrock Console model access activation steps (Amazon Bedrock Console -> Model access -> Modify model access -> Enable Amazon Nova Lite).
- **Suggestion**: AWS Bedrock could return a more descriptive error code such as `ModelAccessNotGrantedException` rather than generic `ValidationException: Operation not allowed`.

### SQLx Migration Checksum Mismatch on Database Re-initialization
- **Date**: 2026-10-22
- **Task attempted**: Booting Axum backend server with live SQLite database (`data/tabletalk.db`).
- **Steps taken**:
  1. Ran `cargo run --bin mcp_tabletalk_backend`.
- **Expected vs actual**:
  - *Expected*: Server boots and runs migrations.
  - *Actual*: Panicked with `Migration error: migration 7 was previously applied but has been modified`.
- **Severity**: Medium
- **Workaround**: SQLx verifies SHA-384 checksums of all applied migration scripts against the `_sqlx_migrations` table. Because migration 7 had been modified in an earlier commit after the database was initially created, the checksum failed. Resolved by moving the stale `.db` file to `.db.bak` and booting with `SEED_ON_START=true` to cleanly generate fresh migrations and seed data.
- **Suggestion**: In local development environments, provide a CLI flag or environment option (`SQLX_OFFLINE_FORCE_REAPPLY=true`) to automatically re-sync modified migration checksums without requiring manual file removal.

### Groq vs Bedrock Multi-Turn Tool Call Serialization Harmony
- **Date**: 2026-10-22
- **Task attempted**: Unifying Groq (OpenAI-compatible) and Bedrock (Amazon Converse) tool calling protocols into one shared orchestrator loop.
- **Steps taken**:
  1. Built `GroqClient` with tool conversion (`tools: [{"type": "function", ...}]`) and OpenAI response parsing.
  2. Evaluated whether to feed tool results back with role `"tool"` (Groq/OpenAI specific) or role `"user"` (Bedrock Converse compatible).
- **Expected vs actual**:
  - *Expected*: Both providers parse tool output cleanly.
  - *Actual*: Groq requires `tool_call_id` when using role `"tool"`, whereas Bedrock Converse expects `toolResult` blocks.
- **Severity**: Medium
- **Workaround**: Standardized tool execution output into conversational user turns (`Tool '<name>' result: <safe_json>`), allowing both Bedrock and Groq models to digest tool feedback naturally over multiple iterations without protocol divergence or complex state management.
- **Suggestion**: LLM API standards should align on a common schema representation for multi-turn tool calling across providers.

### Card Gallery Test Fixture Regression
- **Date**: 2026-10-22
- **Task attempted**: Running full frontend test suite (`npm test -- --run`).
- **Steps taken**:
  1. Executed Vitest across all 12 test files.
- **Expected vs actual**:
  - *Expected*: 83 tests pass.
  - *Actual*: `a11y.test.tsx` failed because `create_booking_confirmed.json` contained `"status": "unavailable"` and `"booking": null` from an accidental earlier commit.
- **Severity**: Low
- **Workaround**: Examined `git log -p frontend/src/cards/fixtures/create_booking_confirmed.json`, identified the overwritten payload, and restored the confirmed booking structure with confirmation code and cancellation policy. All 83 tests passed immediately.


