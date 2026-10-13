# MCP Specification 2025-11-25 Analysis & SDK Evaluation

**Author**: TableTalk Engineering  
**Date**: 2026-10-12  
**Target Spec Version**: `2025-11-25`  
**Transport**: Streamable HTTP  

---

## 1. Primary Sources & References

All research and claims in this document are derived from primary documentation accessed on **2026-10-12**:

| Source | Reference / URL | Date Accessed |
|---|---|---|
| MCP Specification Index | [modelcontextprotocol.io/specification](https://modelcontextprotocol.io/specification/) | 2026-10-12 |
| MCP 2025-11-25 Changelog | [modelcontextprotocol.io/specification/2025-11-25/changelog](https://modelcontextprotocol.io/specification/2025-11-25/changelog) | 2026-10-12 |
| MCP Transports Specification | [modelcontextprotocol.io/specification/2025-11-25/basic/transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports) | 2026-10-12 |
| MCP Lifecycle Specification | [modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle) | 2026-10-12 |
| MCP Server Tools Specification | [modelcontextprotocol.io/specification/2025-11-25/server/tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools) | 2026-10-12 |
| MCP Authorization Specification | [modelcontextprotocol.io/specification/2025-11-25/basic/authorization](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization) | 2026-10-12 |
| Rust MCP SDK (`rmcp`) Crate | [crates.io/crates/rmcp/3.5.1](https://crates.io/crates/rmcp/3.5.1) | 2026-10-12 |
| Rust MCP SDK Source Repository | [github.com/modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk) | 2026-10-12 |
| MCP Inspector CLI | [npmjs.com/package/@modelcontextprotocol/inspector](https://www.npmjs.com/package/@modelcontextprotocol/inspector) (v2.10.1) | 2026-10-12 |

---

## 2. Specification Summary: 2025-11-25 for TableTalk

### A. Lifecycle & Negotiation
- **Initialize Handshake**:
  - The client initiates communication with an HTTP POST containing a JSON-RPC `initialize` request (`method: "initialize"`), declaring `protocolVersion: "2025-11-25"`, `capabilities`, and `clientInfo`.
  - The server responds with `InitializeResult` stating the agreed `protocolVersion`, server `capabilities` (`tools`, `resources`, etc.), and `serverInfo`.
  - The negotiated protocol version MUST be less than or equal to the client's requested version.
  - The client completes initialization by sending `notifications/initialized`. Prior to this notification, the server MUST NOT send general requests (only pings and logging are permitted).
- **Protocol Version Header**:
  - Subsequent client requests must send `MCP-Protocol-Version: 2025-11-25`. Unsupported versions must be rejected with HTTP `400 Bad Request`.

### B. Streamable HTTP Transport
- **Single Endpoint**: All operations occur on a single HTTP endpoint path (e.g., `/mcp`).
- **HTTP POST**:
  - Client sends JSON-RPC requests, notifications, or responses as the POST body.
  - Client MUST supply `Accept: application/json, text/event-stream`.
  - If input is a notification or response, server returns HTTP `202 Accepted` with an empty body.
  - If input is a request, server responds with `Content-Type: application/json` (single response) or `Content-Type: text/event-stream` (SSE stream).
- **HTTP GET & SSE Streaming**:
  - Client may issue GET with `Accept: text/event-stream` to open a streaming channel for server-sent notifications or message resumption.
  - When initiating an SSE stream, the server SHOULD emit a priming event with an event ID and empty data (`id: 0`, `retry: 3000`).
  - Servers may terminate the stream or disconnect at will to prevent idle connection exhaustion; clients poll/reconnect respecting the `retry` duration.
- **Session Management (`Mcp-Session-Id`)**:
  - During initialization, the server MAY issue an `Mcp-Session-Id` header (visible ASCII, cryptographically secure UUID/token).
  - If issued, the client MUST supply `Mcp-Session-Id` on all subsequent requests.
  - Missing session ID on non-init requests yields HTTP `400 Bad Request`.
  - Expired or terminated session yields HTTP `404 Not Found`, instructing client to re-initialize.
  - Clients can explicitly terminate a session via HTTP `DELETE /mcp` with `Mcp-Session-Id`.
- **Security & Origin Validation**:
  - Servers MUST validate the `Origin` header on all incoming connections to prevent DNS rebinding attacks.
  - If `Origin` is present and invalid, servers MUST return HTTP `403 Forbidden`.
  - Local bindings SHOULD default to `127.0.0.1` rather than `0.0.0.0` unless explicitly configured for public hosting.

### C. Tools & Schemas
- **Tool Identifiers**:
  - Names MUST be 1–128 characters, case-sensitive, using only `[A-Za-z0-9_.-]` (SEP-986).
- **Schema Dialect**:
  - Default JSON Schema dialect is **Draft 2020-12** (SEP-1613).
  - Parameterless tools must explicitly declare `{ "type": "object", "additionalProperties": false }`.
- **Structured Content & `outputSchema`**:
  - Tools may provide `outputSchema` in their definition.
  - Execution responses return `structuredContent` (JSON object) alongside text/image `content` blocks.
- **Error Distinction (Critical for LLM Self-Correction)**:
  - **Protocol Errors**: Standard JSON-RPC errors (e.g., `-32602 Unknown tool`, `-32600 Invalid request`) ONLY for unknown tools or malformed envelopes.
  - **Tool Execution Errors**: Returned as a standard result object with `isError: true` (e.g., validation errors like "Date must be in the future", "Table not available for party of 8 at 7:00 PM"). This allows the assistant model to observe the failure reason and self-correct (SEP-1303).

### D. Authorization (Self-Hosted Servers)
- **Requirement**: For self-hosted servers with public access, authorization is **OPTIONAL** if no authentication is mandated.
- If authentication is enforced, servers SHOULD conform to OAuth 2.1 and provide OAuth 2.0 Protected Resource Metadata (RFC 9728) with `Authorization: Bearer <token>` in the request headers (never in query parameters).

### E. Other Protocol Primitives for TableTalk
| Primitive | Relevant to Restaurant MCP? | Evaluation & Recommendation |
|---|---|---|
| **Resources** | **Yes** | Expose static/dynamic read-only data (e.g., `restaurant://menu`, `restaurant://hours`, `restaurant://allergens`) as MCP resources for contextual grounding. |
| **Prompts** | **Yes** | Provide structured prompt templates (e.g., `book_table_flow`, `order_checkout_summary`) to steer voice assistant dialogues into predictable states. |
| **Elicitation** | **Optional / Future** | URL-mode elicitation can hand off sensitive actions (e.g., credit card payment completion for deposits) to external web links. |
| **MCP Apps / UI** | **Yes (for Frontend)** | MCP UI elements (`io.modelcontextprotocol/ui`) allow the React simulator to render rich interactive visual cards (menu items, seating maps, reservation receipts) alongside voice responses. |
| **Tasks** | **No** (Skip for Phase 1-3) | Tasks represent experimental asynchronous durable tracking (SEP-1686); restaurant queries are low-latency synchronous operations. |

---

## 3. Alexa+ Track Hackathon Requirements

1. **Mandatory Runtime Technology**:
   - The server must be a self-hosted MCP server implementing MCP specification version **`2025-11-25` (minimum acceptable)** over **Streamable HTTP**.
   - Must be imported and actively called at runtime in executable code, not merely referenced in documentation.
2. **Architecture**:
   - Alexa+ acts as the MCP client connecting to the self-hosted Streamable HTTP endpoint.
   - Alexa+ handles speech-to-text, orchestration, and speech synthesis, while TableTalk supplies tools, schemas, and structured data.
3. **Hardware / Device Constraint**:
   - No physical Echo device required; developers can validate and demo via web interfaces (our React frontend).
4. **Public Docs Availability**:
   - High-level track guidelines, MCP spec links, and Streamable HTTP documentation are publicly available on the Amazon Developer Hackathon portal and Devpost.
   - *Note / Assumption*: Internal proprietary Alexa+ backend sandbox gateway parameters (if any) are distributed via hackathon participant portals upon team submission registration. The standard MCP Streamable HTTP specification remains the canonical interface contract.

---

## 4. Rust MCP SDK (`rmcp`) Evaluation

- **Crate Version**: `rmcp 3.5.1` (Released: 2026; requires Rust 1.88+).
- **Protocol Versions Supported in Code**:
  - Found in `rmcp::model::ProtocolVersion`:
    - `V_2024_11_05 = "2024-11-05"`
    - `V_2025_03_26 = "2025-03-26"`
    - `V_2025_06_18 = "2025-06-18"`
    - `V_2025_11_25 = "2025-11-25"` (Explicitly defined!)
    - `V_2026_07_28 = "2026-07-28"` (`LATEST`)
    - `LATEST_WITH_INITIALIZE = V_2025_11_25`
  - **Verdict on `2025-11-25`**: Fully supported and explicitly designated as `LATEST_WITH_INITIALIZE`.
- **Streamable HTTP Features**:
  - Enabled via Cargo feature `transport-streamable-http-server`.
  - Exposes `StreamableHttpService<S, LocalSessionManager>` which integrates natively into `axum::Router::nest_service("/mcp", service)`.
  - Supports session generation (`mcp-session-id`), priming SSE events (`id: 0`, `retry: 3000`), and SSE reconnection.
  - Supports `StreamableHttpServerConfig` with `.with_allowed_origins(...)` for Origin validation.
- **Tool Macros & Schemas**:
  - Provides procedural macros: `#[tool]`, `#[tool_handler]`, and `#[tool_router]`.
  - Emits JSON Schema Draft 2020-12 via `schemars`.
  - Supports structured output using `Json<T>`.

### Compatibility Matrix

| Feature / Spec Requirement | 2025-11-25 Requirement | `rmcp` 3.5.1 Support | Evidence & Reference |
|---|---|---|---|
| **Protocol Version 2025-11-25** | Support `2025-11-25` handshake | **Yes** | `rmcp::model::ProtocolVersion::V_2025_11_25`; verified in live spike echo |
| **Streamable HTTP Transport** | Single endpoint, POST + GET | **Yes** | `rmcp::transport::streamable_http_server::StreamableHttpService` |
| **Axum Integration** | Mount onto Tokio/Axum router | **Yes** | `router.nest_service("/mcp", service)` |
| **Session Handling** | `Mcp-Session-Id` header tracking | **Yes** | `LocalSessionManager` issues UUID header, verified in spike response |
| **SSE Priming Event** | SSE priming with `id` and `retry` | **Yes** | Server emits `id: 0`, `retry: 3000` chunk before JSON response |
| **Origin Header Validation** | Reject invalid origins with HTTP 403 | **Yes (Configurable)** | `StreamableHttpServerConfig::with_allowed_origins()` returns 403 Forbidden |
| **JSON Schema 2020-12** | Default dialect for input schemas | **Yes** | Emits `"$schema": "https://json-schema.org/draft/2020-12/schema"` |
| **Tool Macros** | Ergonomic typed tool definitions | **Yes** | `#[tool]`, `#[tool_handler]`, `#[tool_router]` |
| **Structured Output** | `structuredContent` + `outputSchema` | **Yes** | `Json<T>` return type; `model::tool::output_schema` |
| **Tool Execution Errors** | Return `{ "isError": true }` on domain error | **Yes** | `CallToolResult { isError: true, content: [...] }` |

---

## 5. Live Throwaway Spike Results

A throwaway spike was compiled and run in a temporary scratch directory (`/tmp/rmcp-spike`) with an Axum server mounting `rmcp::transport::streamable_http_server::StreamableHttpService` on port `8089`.

### A. Raw `initialize` Request (HTTP POST)
```http
POST /mcp HTTP/1.1
Host: 127.0.0.1:8089
Accept: application/json, text/event-stream
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-11-25",
    "capabilities": {},
    "clientInfo": {
      "name": "tabletalk-test-client",
      "version": "1.0.0"
    }
  }
}
```

### B. Raw `initialize` Response
```http
HTTP/1.1 200 OK
content-type: text/event-stream
cache-control: no-cache
x-accel-buffering: no
mcp-session-id: 7dc4999e-d119-4565-afd1-512cce116fb1
transfer-encoding: chunked
date: Mon, 12 Oct 2026 12:27:13 GMT

data: 
id: 0
retry: 3000

data: {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"rmcp","version":"3.5.1"}}}
```

### C. Live Verification Findings
1. **Protocol Negotiation**: The server explicitly responded with `"protocolVersion": "2025-11-25"`.
2. **Session Generation**: The server generated `mcp-session-id: 7dc4999e-d119-4565-afd1-512cce116fb1`.
3. **Tool Listing (`tools/list`)**: Returned the tool schema with JSON Schema 2020-12 dialect:
   ```json
   {
     "name": "ping_tabletalk",
     "description": "Test tool for TableTalk spike",
     "inputSchema": {
       "$schema": "https://json-schema.org/draft/2020-12/schema",
       "type": "object",
       "properties": { "message": { "type": "string" } },
       "required": ["message"]
     }
   }
   ```
4. **Tool Call (`tools/call`)**: Executed dummy tool and responded with `{ "content": [{"type":"text","text":"Spike received: Verified"}], "isError": false }`.
5. **MCP Inspector CLI Verification**:
   - Ran `npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:8089/mcp --method tools/list`.
   - Verified that the official MCP Inspector successfully connected, negotiated the protocol, parsed the schema, and called the tool without errors.

---

## 6. Architecture Decision & Justification

### Decision: **Option B (Use `rmcp` plus small custom patches or wrappers for gaps)**

#### Justification:
1. **`rmcp` 3.5.1 solves 90% of the hardest protocol requirements**:
   - Correctly negotiates spec `2025-11-25`.
   - Implements full SSE chunking, event priming (`retry: 3000`), and session state tracking.
   - Generates compliant JSON Schema Draft 2020-12 from Rust structs via `schemars`.
2. **Why not Option A (rmcp as-is)?**:
   - `rmcp`'s default configuration does not enforce `Origin` validation by default (`allowed_origins` defaults to empty). We must supply an explicit configuration wrapper to enforce DNS rebinding protection and return HTTP 403 Forbidden.
   - We need custom Axum middleware for domain error normalization so restaurant domain validation (e.g. invalid date formats, party size exceeds capacity) always maps to Tool Execution Errors (`isError: true`) instead of generic JSON-RPC protocol error codes.
3. **Why not Option C (re-implementing from scratch on Axum)?**:
   - Rebuilding JSON-RPC dispatch, SSE chunk streaming, event ID resumption, session stores, and JSON Schema generation from scratch would consume excessive development time and introduce protocol edge-case risks.

#### Architectural Risks & Mitigations:
| Risk | Impact | Mitigation |
|---|---|---|
| **Permissive Default Origin Policy** | Potential DNS rebinding vulnerability if unconfigured | Explicitly initialize `StreamableHttpServerConfig::default().with_allowed_origins([...])` in server setup. |
| **In-Memory Session Eviction** | In-memory session loss on server restart | Use stateless or durable session configuration; client automatically re-initializes on HTTP 404. |
| **Tool Execution Error Format** | Assistant hallucinating protocol failure instead of self-correcting | Ensure all business logic errors return `CallToolResult { isError: true, content: [...] }`. |

---

## 7. Compliance Checklist (Verified in Phase 4)

All checklist items below have been formally proven by automated tests in `backend/tests/` and recorded in `docs/MCP_VERIFICATION.md`:

- [x] **Streamable HTTP Endpoint**: Server exposes `/mcp` accepting HTTP POST and GET (`test_full_mcp_lifecycle_and_tools`, `test_sse_stream_open_and_client_disconnect`).
- [x] **Protocol Version Agreement**: Server accepts and negotiates `2025-11-25` (`test_edge_case_e_unsupported_protocol_version_negotiation`).
- [x] **Protocol Header Validation**: Rejects requests specifying unsupported protocol versions with HTTP 400 (`test_edge_case_f_invalid_protocol_version_header`).
- [x] **Session Handling**: Generates secure `Mcp-Session-Id` and tracks sessions across requests (`test_50_concurrent_sessions_and_latency`).
- [x] **Session Termination & Re-initialization**: Handles `DELETE /mcp` with HTTP 202 and returns HTTP 404 for terminated or unknown sessions (`test_edge_case_g_session_termination_and_unknown_session`).
- [x] **SSE Priming & Disconnect Resilience**: Emits initial event with ID and retry field; handles mid-stream client disconnects without task leaks or server panic (`test_sse_stream_open_and_client_disconnect`).
- [x] **Origin Validation & DNS Rebinding**: Rejects disallowed `Origin` headers with HTTP 403 Forbidden; permits configured and non-browser (absent) origins (`test_edge_case_l_origin_header_validation`).
- [x] **Request Body & Media Type Limits**: Rejects oversized payloads (>2MB) without crash (`test_edge_case_j_oversized_request_body`); rejects malformed JSON (HTTP 415), invalid JSON-RPC (HTTP 415), wrong `Content-Type` (HTTP 415), and missing `Accept` header (HTTP 406).
- [x] **JSON-RPC Conformance**: Rejects pre-initialization calls with HTTP 422 (`test_edge_case_d`); returns `-32601` for unknown methods (`test_edge_case_c`); rejects batch requests (`test_edge_case_i`).
- [x] **Client Notifications & Ping**: Correctly handles notifications with HTTP 202 empty body (`test_edge_case_k`) and handles `ping` / `notifications/cancelled` (`test_edge_case_m`).
- [x] **Request Timeout & Concurrency**: Cuts off hung requests with HTTP 408 (`test_request_timeout_cutoff`); handles 50 concurrent sessions with 0 errors/panics at p50 111ms / p95 116ms (`test_50_concurrent_sessions_and_latency`).
- [x] **Tool Naming**: All nine tool names (`ping_restaurant`, `search_menu`, `check_availability`, `create_booking`, `cancel_booking`, `get_booking`, `place_order`, `get_order`, `cancel_order`) conform to `[A-Za-z0-9_.-]` within 1–128 characters.
- [x] **JSON Schema Dialect**: Tools export schemas in Draft 2020-12 format.
- [x] **Parameterless Tools**: Zero-parameter calls fall back gracefully to defaults.
- [x] **Domain Error Separation**: Tool execution errors return `isError: true` inside tool results, not JSON-RPC protocol error codes.
- [x] **Runtime Invocation**: MCP server is imported and called at runtime in executable code (`tabletalk-backend`).
- [x] **Read-Only Availability & Menu Discovery**: `check_availability` and `search_menu` are registered with `readOnlyHint: true`, `idempotentHint: true`, `destructiveHint: false`, `openWorldHint: false`, and strongly-typed `outputSchema` (verified in `mcp_availability_test.rs`).
- [x] **Transactional Booking Tools & Side-Effect Annotations**:
  - `create_booking`: `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: true` (with idempotency key), `openWorldHint: false`.
  - `cancel_booking`: `readOnlyHint: false`, `destructiveHint: true`, `idempotentHint: true`, `openWorldHint: false`.
  - `get_booking`: `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, `openWorldHint: false`.
  - All side effects run in strict SQLite `BEGIN IMMEDIATE` write transactions with audit trail events and anti-enumeration protections.
- [x] **Transactional Ordering Tools & Pricing Integrity (Phase 9)**:
  - `place_order`: `readOnlyHint: false`, `destructiveHint: false`, `idempotentHint: true` (with key), `openWorldHint: false`.
  - `cancel_order`: `readOnlyHint: false`, `destructiveHint: true`, `idempotentHint: true`, `openWorldHint: false`.
  - `get_order`: `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`, `openWorldHint: false`.
  - Pure integer arithmetic in minor units (paise), server-side price snapshots, round-half-up tax calculation, allergen conflict rejection with unverified data blocks, dynamic kitchen prep estimation with slot grid snapping, and rate-limited anti-enumeration cancellations.

---

## 8. Remaining Known Gaps (Deferred to Later Phases)

The following capabilities are intentionally deferred per the project roadmap:

1. **Payments & Deposits**: Online payments, credit card capture, deposit processing, and payment ledgers are deferred to Phase 12. Orders are strictly `"unpaid_pay_at_pickup"`.
2. **Session Memory & "Your Usual"**: Contextual state across dialogues and customer past favorites ("the usual") are deferred to Phase 10.
3. **Rich Cards / UI Elements**: MCP UI widgets and visual interactive order cards are deferred to Phase 13.
4. **Persistent Session Storage**: Sessions are managed in-memory via `LocalSessionManager`. Server restarts invalidate existing sessions (clients must re-initialize). Distributed session persistence (Redis/DynamoDB) is deferred.
5. **Authentication & Authorization**: The `/mcp` transport does not yet require OAuth 2.0 Bearer tokens or API keys (scheduled for future security phase).
6. **Bedrock / LLM Orchestration**: Direct AWS Bedrock agent wiring is scheduled for later phases.


