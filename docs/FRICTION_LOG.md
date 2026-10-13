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
- **Suggestion**: For SQLite-backed reservation systems, explicitly design transaction-level locking primitives with concurrency tests rather than relying on schema-only guards.


