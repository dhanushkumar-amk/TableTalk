# TableTalk

> Voice-driven restaurant intelligence powered by the Model Context Protocol (MCP).

**Status**: in development

TableTalk is a restaurant Model Context Protocol (MCP) server that empowers voice assistants to search menus, check table availability, book reservations, and place orders. A React web frontend simulates the voice assistant experience for real-time demos and evaluation.

---

## Hackathon

- **Competition**: Amazon Developer Hackathon
- **Track**: Alexa+ track
- **Mini Challenges**: AWS Builder challenge & Open Source mini challenge

---

## Prerequisites

- **Rust**: stable toolchain (1.80+ recommended, managed via `rust-toolchain.toml`)
- **Node.js**: v20+ LTS (with `npm` v10+)
- **Git**
- **curl** (for endpoint testing)

---

## Project Structure

```
tabletalk/
├── backend/                  # Cargo workspace member (Axum + Tokio)
│   ├── Cargo.toml            # Backend dependencies
│   ├── migrations/           # Embedded SQL migrations
│   │   └── 0001_initial_schema.sql
│   ├── tests/                # Integration tests (MCP protocol, database integrity)
│   └── src/
│       ├── main.rs           # Startup & server runtime entrypoint (supports 'seed' CLI)
│       ├── lib.rs            # Library exports for integration testing
│       ├── config.rs         # Environment variable configuration
│       ├── server.rs         # TableTalk MCP server handler & capabilities
│       ├── http.rs           # Axum router, health check, CORS, Origin validation
│       ├── db/               # Database persistence layer
│       │   ├── mod.rs        # Connection pool setup, pragmas, migrations
│       │   ├── error.rs      # DbError domain error types (thiserror)
│       │   ├── models.rs     # Typed structs, newtypes, and domain enums
│       │   ├── seed.rs       # Deterministic idempotent fictional data seeder
│       │   └── repo/         # Repositories (restaurant, tables, menu, customers, bookings, orders)
│       └── tools/            # MCP tool implementations
│           ├── mod.rs
│           └── ping.rs       # ping_restaurant hello-world tool
├── frontend/                 # Vite + React + TypeScript web application
│   ├── src/
│   │   ├── components/ui/    # shadcn/ui components (Tailwind CSS)
│   │   ├── App.tsx           # TableTalk client interface
│   │   └── main.tsx          # Application entrypoint
│   ├── package.json
│   └── vite.config.ts
├── docs/
│   ├── FRICTION_LOG.md       # Developer experience and friction tracking
│   └── PRODUCT_FEEDBACK.md   # Feedback for MCP SDK, Streamable HTTP, AWS, and AI tools
├── .github/
│   └── workflows/
│       └── ci.yml            # CI workflow for backend and frontend
├── .gitignore                # Workspace git ignore rules
├── Cargo.toml                # Root workspace configuration
├── rust-toolchain.toml       # Pinned Rust stable toolchain
├── LICENSE                   # MIT License
└── README.md
```

---

## Setup & Run

### 1. Backend (Rust / Axum)

Run tests across the workspace:
```bash
cargo test
```

Run linter and formatting checks:
```bash
cargo fmt --check
cargo clippy -- -D warnings
```

Start the backend server (binds to `0.0.0.0:${PORT:-8080}`):
```bash
cargo run --bin tabletalk-backend
```

Verify the health check endpoint in a separate terminal:
```bash
curl http://localhost:8080/health
# Response: {"status":"ok"}
```

### 2. Configuration & Environment Variables

TableTalk backend accepts configuration via environment variables (see `.env.example`):

| Variable | Description | Default |
|---|---|---|
| `PORT` | Listening port for the Axum HTTP server | `8080` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed origins for DNS rebinding protection and CORS | `http://localhost:5173,http://127.0.0.1:5173,http://localhost:8080,http://127.0.0.1:8080` |
| `RUST_LOG` | Tracing logging filter | `info,tabletalk_backend=debug` |
| `DATABASE_URL` | SQLite database connection string | `sqlite://data/tabletalk.db` |
| `SEED_ON_START` | Seed database on boot if empty | `false` |
| `RESTAURANT_TZ` | Establishment IANA timezone override | `Asia/Kolkata` |
| `RATE_LIMIT_PER_MINUTE` | Max requests per minute per IP on `/mcp` and checkout endpoints | `600` |
| `REQUEST_TIMEOUT_SECS` | Maximum HTTP request timeout in seconds | `30` |

Copy the example configuration:
```bash
cp .env.example .env
```

### 3. Database (SQLite / SQLx)

TableTalk uses SQLite with Write-Ahead Logging (WAL) and foreign keys enabled. Migrations are embedded and apply automatically on server startup.

#### Seeding the Database
Populate the database with deterministic fictional seed data for "The Saffron Nebula Bistro" (14 tables, 35 menu items, 5 demo customers with preferences, past orders, and future bookings):
```bash
cargo run -- seed
```
Seeding is idempotent—running it multiple times leaves all row counts unchanged without duplicate data or foreign key violations.

#### Resetting the Database
To reset the database to a clean state:
```bash
# Delete the SQLite database file and WAL logs
rm -f data/tabletalk.db*

# Re-run migrations and seed
cargo run -- seed
```

---

## MCP Endpoint

- **Endpoint URL**: `http://localhost:8080/mcp`
- **Transport**: Streamable HTTP
- **Supported Protocol Version**: `2025-11-25`
- **Session Management**: Stateful via `mcp-session-id` header
- **DNS Rebinding Protection**: Validates incoming `Origin` headers against `ALLOWED_ORIGINS` (requests without `Origin` are allowed for non-browser clients)
- **Rate Limiting**: Sliding-window limiter on `/mcp` and checkout endpoints (default 600 req/min per IP, configurable via `RATE_LIMIT_PER_MINUTE`). Returns HTTP `429 Too Many Requests` with a `Retry-After` header. `/health` is unthrottled.
- **Request Body Limit**: 2 MB max body size (`413 Payload Too Large`).
- **Request Timeout**: 30-second timeout on requests (`408 Request Timeout`).
- **Panic Resilience**: Unhandled panics are caught by `CatchPanicLayer`, logging the incident with a UUID Request ID and returning a generic 500 without crashing the server.
- **Input Validation**: All 24 tools enforce bounds, ISO dates/times, and sanitized strings. Extra unknown fields are ignored per Postel's Law.

### Testing with `curl`

1. **Initialize handshake**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "Accept: application/json, text/event-stream" \
     -d '{
       "jsonrpc": "2.0",
       "id": 1,
       "method": "initialize",
       "params": {
         "protocolVersion": "2025-11-25",
         "capabilities": {},
         "clientInfo": {"name": "test-client", "version": "1.0.0"}
       }
     }'
   ```
   *Note the `mcp-session-id` header in the HTTP 200 response.*

2. **Send initialized notification**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{"jsonrpc": "2.0", "method": "notifications/initialized"}'
   ```

3. **Discover tools (`tools/list`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{"jsonrpc": "2.0", "id": 2, "method": "tools/list"}'
   ```

4. **Call the hello-world ping tool (`tools/call`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 3,
       "method": "tools/call",
       "params": {
         "name": "ping_restaurant",
         "arguments": {"name": "Guest"}
       }
     }'
   ```

5. **Search the menu (`search_menu`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 4,
       "method": "tools/call",
       "params": {
         "name": "search_menu",
         "arguments": {"query": "biryani", "limit": 3}
       }
     }'
   ```

6. **Check table availability (`check_availability`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 5,
       "method": "tools/call",
       "params": {
         "name": "check_availability",
         "arguments": {
           "date": "2026-10-23",
           "time": "19:30",
           "party_size": 2,
           "area": "indoor"
         }
       }
     }'
   ```

7. **Create a booking (`create_booking`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 6,
       "method": "tools/call",
       "params": {
         "name": "create_booking",
         "arguments": {
           "date": "2026-10-23",
           "time": "19:30",
           "party_size": 2,
           "guest_name": "Eleanor Vance",
           "idempotency_key": "idem-eleanor-booking-001"
         }
       }
     }'
   ```
   *Returns status `"confirmed"`, confirmation code (e.g. `TT-7PTUF`), and natural voice confirmation.*

8. **Lookup booking summary (`get_booking`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 7,
       "method": "tools/call",
       "params": {
         "name": "get_booking",
         "arguments": {
           "confirmation_code": "TT-7PTUF",
           "guest_name": "Eleanor Vance"
         }
       }
     }'
   ```

9. **Cancel booking (`cancel_booking`)**:
   ```bash
   curl -i -X POST http://localhost:8080/mcp \
     -H "Content-Type: application/json" \
     -H "mcp-session-id: <SESSION_ID>" \
     -H "mcp-protocol-version: 2025-11-25" \
     -d '{
       "jsonrpc": "2.0",
       "id": 8,
       "method": "tools/call",
       "params": {
         "name": "cancel_booking",
         "arguments": {
           "confirmation_code": "TT-7PTUF",
           "guest_name": "Eleanor Vance",
           "reason": "Change of plans"
         }
       }
     }'
   ```

10. **Place a pickup order (`place_order`)**:
    ```bash
    curl -i -X POST http://localhost:8080/mcp \
      -H "Content-Type: application/json" \
      -H "mcp-session-id: <SESSION_ID>" \
      -H "mcp-protocol-version: 2025-11-25" \
      -d '{
        "jsonrpc": "2.0",
        "id": 9,
        "method": "tools/call",
        "params": {
          "name": "place_order",
          "arguments": {
            "items": [
              {"item_id": 1, "quantity": 2, "notes": "Extra crispy leaves"},
              {"item_id": 25, "quantity": 2}
            ],
            "pickup_time": "13:15",
            "guest_name": "Aarav Mehta",
            "idempotency_key": "order-idem-aarav-001"
          }
        }
      }'
    ```
    *Returns status `"placed"`, order code (e.g. `TO-GWT4K`), itemized pricing, round-half-up tax, and natural voice confirmation.*

11. **Lookup order summary (`get_order`)**:
    ```bash
    curl -i -X POST http://localhost:8080/mcp \
      -H "Content-Type: application/json" \
      -H "mcp-session-id: <SESSION_ID>" \
      -H "mcp-protocol-version: 2025-11-25" \
      -d '{
        "jsonrpc": "2.0",
        "id": 10,
        "method": "tools/call",
        "params": {
          "name": "get_order",
          "arguments": {
            "order_code": "TO-GWT4K",
            "guest_name": "Aarav Mehta"
          }
        }
      }'
    ```

12. **Cancel order (`cancel_order`)**:
    ```bash
    curl -i -X POST http://localhost:8080/mcp \
      -H "Content-Type: application/json" \
      -H "mcp-session-id: <SESSION_ID>" \
      -H "mcp-protocol-version: 2025-11-25" \
      -d '{
        "jsonrpc": "2.0",
        "id": 11,
        "method": "tools/call",
        "params": {
          "name": "cancel_order",
          "arguments": {
            "order_code": "TO-GWT4K",
            "guest_name": "Aarav Mehta",
            "reason": "Meeting rescheduled"
          }
        }
      }'
    ```

For verbatim commands and recorded responses, see [docs/MCP_VERIFICATION.md](docs/MCP_VERIFICATION.md) and tool specifications in [docs/MCP_TOOLS.md](docs/MCP_TOOLS.md).

---

## Testing

TableTalk includes a comprehensive test suite covering protocol conformance, edge cases, streaming resilience, concurrency, and automated smoke testing.

### 1. Rust Test Suite (`cargo test`)

Run all unit and integration tests across the workspace:
```bash
cargo test
```

This executes:
- **Unit Tests**: Parameter validation and response formatting for tools (`ping_restaurant`).
- **Protocol Edge-Case Tests** (`tests/mcp_edge_cases_test.rs`): 13 spec conformance tests covering malformed JSON, invalid JSON-RPC, unknown methods, pre-initialization calls, protocol version negotiation, session lifecycle, Content-Type/Accept negotiation, JSON-RPC batching, oversized payloads, client notifications, Origin validation, and cancellations/pings.
- **Concurrency & Streaming Tests** (`tests/mcp_concurrency_and_stream_test.rs`): SSE stream lifecycle with mid-stream client disconnects, 50 concurrent session handshakes + tool calls with latency tracking (p50/p95), and request timeout cutoffs.

### 2. Automated Smoke Test Script (`scripts/smoke-test.sh`)

An end-to-end bash script that boots the server in the background, waits for `/health`, executes the MCP Inspector CLI (`@modelcontextprotocol/inspector --cli`) and protocol validation checks, asserts PASS/FAIL per check, and traps on exit:

```bash
# Run with default port (8088)
./scripts/smoke-test.sh

# Or run with custom port and timeout
PORT=8090 MAX_RETRIES=15 ./scripts/smoke-test.sh
```

The script exits `0` on success and non-zero on any failure.

### 3. MCP Inspector Testing

#### A. Automated CLI Mode
```bash
# Discover tools
npx @modelcontextprotocol/inspector --cli http://localhost:8080/mcp --method tools/list

# Invoke tool with argument
npx @modelcontextprotocol/inspector --cli http://localhost:8080/mcp --method tools/call --tool-name ping_restaurant --tool-arg name="Alice"

# Invoke tool with default argument
npx @modelcontextprotocol/inspector --cli http://localhost:8080/mcp --method tools/call --tool-name ping_restaurant
```

#### B. Interactive Web UI Mode
1. Start the TableTalk backend server:
   ```bash
   cargo run --bin tabletalk-backend
   ```
2. In a separate terminal, launch the MCP Inspector web UI:
   ```bash
   npx @modelcontextprotocol/inspector
   ```
3. Open the browser URL displayed in the terminal (e.g. `http://127.0.0.1:6274`).
4. Select transport **Streamable HTTP**.
5. Enter Server URL: `http://localhost:8080/mcp`.
6. Click **Connect**. Verify that server capabilities and metadata (`tabletalk`, `v0.1.0`) appear.
7. Navigate to the **Tools** tab. Click **List Tools** to observe `ping_restaurant`.
8. Test tool calls:
   - Provide `{"name": "Alice"}` and click **Run Tool**. Verify response contains `Hello, Alice!`.
   - Leave `name` blank and click **Run Tool**. Verify response defaults to `Hello, Guest!`.
   - Provide an invalid schema type (e.g. numeric name) and click **Run Tool**. Verify error response.
9. Click **Disconnect**, then **Connect** again to verify seamless session resumption.

---

### 3. Frontend (React + Vite + TypeScript)

Install dependencies:
```bash
cd frontend
npm install
```

Start the local development server:
```bash
npm run dev
```
Open `http://localhost:5173` in your browser.

Build for production:
```bash
npm run build
```

---

## Memory and Privacy (Phase 10)

TableTalk provides an opt-in profile memory engine allowing voice assistants to remember dining guests across sessions while strictly enforcing data minimization, consent, and allergen safety.

### Privacy Highlights
- **Strictly Opt-In**: Memory is OFF by default. No profile is created unless the guest explicitly consents (`consent_acknowledged: true`).
- **Opaque Token Identity**: Bearer profile tokens (`tt_prof_<43 chars>`) are generated using the OS CSPRNG (256 bits of entropy) and stored only as SHA-256 hashes. Plain tokens are never stored in the database or logged.
- **Safety-Critical Allergen Enforcement**: Stored profile allergies merge into order avoidances and cannot be bypassed via ad-hoc parameters. The server never claims food is "safe" or "guaranteed".
- **Hard Deletion (`forget_me`)**: Permanently hard-deletes the profile, preferences, and security tokens. Historical orders and bookings are unlinked and scrubbed to non-identifying aggregates (`[deleted]` guest name, NULL contacts/notes).
- **Inactivity Purge**: Inactive profiles (> 365 days) are purged via `cargo run -- purge-expired --days 365`.

### Example Memory Flow
1. **Enable Memory (Opt-in)**:
   ```json
   { "tool": "enable_memory", "arguments": { "guest_name": "Ananya Sharma", "consent_acknowledged": true } }
   ```
   *Response*: Returns `profile_token` (e.g. `tt_prof_...`) once for caller storage.
2. **Remember Allergy & Favorites**:
   ```json
   { "tool": "remember_preference", "arguments": { "profile_token": "tt_prof_...", "kind": "allergy", "value": "peanuts", "severity": "severe" } }
   ```
3. **Safe Ordering with Stored Allergens**:
   ```json
   { "tool": "place_order", "arguments": { "profile_token": "tt_prof_...", "items": [{ "item_id": 5, "quantity": 1 }] } }
   ```
   *Response*: Order is blocked (`status: "allergen_conflict"`), flagging Kurkuri Bhindi Tartlets as containing peanuts.
4. **Usual Order Suggestion**:
   ```json
   { "tool": "get_usual_order", "arguments": { "profile_token": "tt_prof_..." } }
   ```
   *Voice*: *"Welcome back, Ananya. Same as last time, the Paneer Tikka Angaare and 2 Truffle Garlic Butter Naan? It's eight hundred eighty-two rupees today."*
5. **Full Deletion**:
   ```json
   { "tool": "forget_me", "arguments": { "profile_token": "tt_prof_...", "confirm": true } }
   ```
   *Response*: Hard-deletes profile and scrubs order records to `[deleted]`.

> [!CAUTION]
> **Synthetic Demo Data Disclaimer**:
> TableTalk uses fictitious names, addresses, and sample credentials for hackathon evaluation only. Real-world deployments require OAuth 2.0 per the official MCP authorization specification, field-level encryption at rest (AES-GCM-256), and formal Data Protection Impact Assessments (DPIAs) for health-related allergen data.

---

## Rich Cards & Developer Gallery (Phase 13)

TableTalk provides an accessible, high-fidelity card system driven by MCP `structuredContent` with a typed TypeScript contract and runtime payload validation:

- **Menu & Dietary**: `MenuCarousel` with keyboard snap-scroll navigation, spice meters, dietary chips, and verbatim allergen notices.
- **Availability & Bookings**: `AvailabilityCard` with snapshot disclaimers and alternative slot chips; `BookingConfirmationCard` with monospace confirmation code and one-click copy.
- **Negotiation & Offers**: `NegotiationOptionsCard` translating machine reason codes into human phrases, real-time hold countdown timers with live-region announcements.
- **Simulated Payments**: `PaymentCard` with explicit `"SIMULATED PAYMENT — no real money"` banner and mock checkout button.
- **Order & Waitlist**: `OrderSummaryCard` displaying lines, taxes, and pay-at-pickup status; `WaitlistCard` tracking approximate queue positions.
- **Privacy & Safety**: Zero sensitive profile tokens in DOM; user notes strictly escaped as inert text; prohibited terms (`safe`, `guaranteed`, `allergy-free`) and green checkmarks are strictly prevented.

### Running the Card Gallery

A dedicated developer harness renders every card component against 33 real-server JSON fixtures:

```bash
cd frontend
npm run dev
```

Open your browser to: **`http://localhost:5173/dev/cards`**

**Developer Controls**:
1. **Theme Toggle**: Switch between Light and Dark mode.
2. **Viewport Width**: Test responsiveness across 360px (mobile), 768px (tablet), and 100% (desktop).
3. **Voice-Only Fallback**: Simulate what non-card clients display (spoken caption only).
4. **Reduced Motion**: Verify that animations and transitions adhere to `prefers-reduced-motion`.
5. **Live Event Monitor**: Observe typed UI events (`onAcceptOption`, `onCancelBooking`, `onOpenCheckout`) dispatched from card interactions without violating the human/AI boundary.

---

## License

This project is licensed under the [MIT License](LICENSE).

