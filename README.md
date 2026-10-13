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

## License

This project is licensed under the [MIT License](LICENSE).
