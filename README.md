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
│   ├── tests/                # Integration tests for MCP and health endpoints
│   └── src/
│       ├── main.rs           # Startup & server runtime entrypoint
│       ├── lib.rs            # Library exports for integration testing
│       ├── config.rs         # Environment variable configuration
│       ├── server.rs         # TableTalk MCP server handler & capabilities
│       ├── http.rs           # Axum router, health check, CORS, Origin validation
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

Copy the example configuration:
```bash
cp .env.example .env
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

For verbatim commands and recorded responses, see [docs/MCP_VERIFICATION.md](docs/MCP_VERIFICATION.md).

### Testing with MCP Inspector CLI

Connect the official `@modelcontextprotocol/inspector` CLI directly to the running server:

```bash
# List available tools
npx @modelcontextprotocol/inspector --cli --server-url http://localhost:8080/mcp --method tools/list

# Invoke the ping_restaurant tool
npx @modelcontextprotocol/inspector --cli --server-url http://localhost:8080/mcp --method tools/call --tool-name ping_restaurant --tool-arg name="Alice"
```

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
