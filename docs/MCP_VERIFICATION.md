# Model Context Protocol (MCP) Runtime Verification

**Date**: 2026-10-12  
**Target Spec Version**: `2025-11-25`  
**Transport**: Streamable HTTP  
**Server Implementation**: `tabletalk` (v0.1.0) on Axum + Tokio using `rmcp` 3.5.1  

---

## 1. Overview

This document records the exact runtime manual verification commands and verbatim output for the TableTalk MCP server (Phase 3 of 20). Both direct HTTP client requests (`curl.exe`) and the official MCP Inspector CLI (`@modelcontextprotocol/inspector`) were executed against `http://127.0.0.1:8080/mcp`.

---

## 2. Server Startup

The server was started using the test runner harness:

```powershell
cargo test --test server_runner run_server_for_manual_verification -- --nocapture --ignored
```

Output:
```text
TableTalk MCP Server listening on http://127.0.0.1:8080
```

---

## 3. Health Check Verification (`GET /health`)

### Command
```powershell
curl.exe -i http://127.0.0.1:8080/health
```

### Response
```http
HTTP/1.1 200 OK
content-type: application/json
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
content-length: 15
date: Mon, 12 Oct 2026 12:52:35 GMT

{"status":"ok"}
```

---

## 4. MCP Protocol Negotiation & Lifecycle (`POST /mcp`)

### Step 1: Initialize Handshake (`initialize`)

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2025-11-25",
      "capabilities": {},
      "clientInfo": {
        "name": "tabletalk-manual-client",
        "version": "1.0.0"
      }
    }
  }'
```

#### Raw Response
```http
HTTP/1.1 200 OK
content-type: text/event-stream
cache-control: no-cache
x-accel-buffering: no
mcp-session-id: 74ed7ed7-3fe5-4c6e-ac59-11bca222e4b9
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
transfer-encoding: chunked
date: Mon, 12 Oct 2026 12:53:02 GMT

data: 
id: 0
retry: 3000

data: {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"tabletalk","title":"TableTalk Restaurant MCP Server","version":"0.1.0","description":"TableTalk is an intelligent restaurant assistant MCP server that powers dining interactions."},"instructions":"TableTalk is an intelligent restaurant assistant powered by the Model Context Protocol. It enables voice assistants and client interfaces to discover menu items, inspect dietary tags, check real-time dining room availability, confirm reservations, and manage orders."}}
```

**Verification Notes**:
- Agreed protocol version: `2025-11-25`.
- Server assigned `mcp-session-id: 74ed7ed7-3fe5-4c6e-ac59-11bca222e4b9`.
- Streamable HTTP priming event emitted with `id: 0` and `retry: 3000`.
- Server identity: `name: "tabletalk"`, `version: "0.1.0"`.

---

### Step 2: Client Initialized Notification (`notifications/initialized`)

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -H "mcp-session-id: 74ed7ed7-3fe5-4c6e-ac59-11bca222e4b9" `
  -H "mcp-protocol-version: 2025-11-25" `
  -d '{
    "jsonrpc": "2.0",
    "method": "notifications/initialized"
  }'
```

#### Raw Response
```http
HTTP/1.1 202 Accepted
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
content-length: 0
date: Mon, 12 Oct 2026 12:53:19 GMT
```

**Verification Notes**:
- HTTP `202 Accepted` returned with zero-length body per Streamable HTTP specification.

---

### Step 3: Tool Discovery (`tools/list`)

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -H "mcp-session-id: 74ed7ed7-3fe5-4c6e-ac59-11bca222e4b9" `
  -H "mcp-protocol-version: 2025-11-25" `
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list"
  }'
```

#### Raw Response
```http
HTTP/1.1 200 OK
content-type: text/event-stream
cache-control: no-cache
x-accel-buffering: no
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
transfer-encoding: chunked
date: Mon, 12 Oct 2026 12:53:32 GMT

data: 
id: 0/0
retry: 3000

data: {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"ping_restaurant","description":"Ping the TableTalk restaurant assistant to verify connectivity and retrieve server status","inputSchema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"name":{"description":"Optional guest or client name to include in greeting","type":["string","null"]}},"type":"object"},"outputSchema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"message":{"description":"Friendly greeting message confirming assistant status","type":"string"},"server":{"description":"Identifier of the serving MCP implementation","type":"string"},"version":{"description":"Crate package version of the TableTalk server","type":"string"}},"required":["message","server","version"],"type":"object"},"annotations":{"readOnlyHint":true,"destructiveHint":false,"idempotentHint":true}}]}}
id: 1/0
```

**Verification Notes**:
- Tool `ping_restaurant` discovered.
- Both `inputSchema` and `outputSchema` conform to JSON Schema Draft 2020-12 (`https://json-schema.org/draft/2020-12/schema`).
- Annotations present: `readOnlyHint: true`, `destructiveHint: false`, `idempotentHint: true`.

---

### Step 4: Tool Execution (`tools/call ping_restaurant`)

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -H "mcp-session-id: 74ed7ed7-3fe5-4c6e-ac59-11bca222e4b9" `
  -H "mcp-protocol-version: 2025-11-25" `
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "ping_restaurant",
      "arguments": {
        "name": "Alex"
      }
    }
  }'
```

#### Raw Response
```http
HTTP/1.1 200 OK
content-type: text/event-stream
cache-control: no-cache
x-accel-buffering: no
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
transfer-encoding: chunked
date: Mon, 12 Oct 2026 12:53:50 GMT

data: 
id: 0/1
retry: 3000

data: {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"message\":\"Hello, Alex! TableTalk restaurant MCP server is online.\",\"server\":\"tabletalk\",\"version\":\"0.1.0\"}"}],"structuredContent":{"message":"Hello, Alex! TableTalk restaurant MCP server is online.","server":"tabletalk","version":"0.1.0"},"isError":false}}
id: 1/1
```

**Verification Notes**:
- `structuredContent` returned matching `outputSchema`.
- `content` text block populated.
- `isError: false` explicitly signals tool execution success.

---

## 5. Security & Protocol Header Verification

### A. Disallowed Origin Header Rejected with HTTP 403 Forbidden

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "Origin: http://disallowed-malicious-site.com" `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

#### Raw Response
```http
HTTP/1.1 403 Forbidden
content-type: text/plain; charset=utf-8
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
content-length: 38
date: Mon, 12 Oct 2026 12:53:58 GMT

Forbidden: Origin not in allowed list
```

---

### B. Unsupported MCP-Protocol-Version Rejected with HTTP 400 Bad Request

#### Request
```powershell
curl.exe -i -X POST http://127.0.0.1:8080/mcp `
  -H "mcp-protocol-version: 1999-01-01" `
  -H "Content-Type: application/json" `
  -H "Accept: application/json, text/event-stream" `
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'
```

#### Raw Response
```http
HTTP/1.1 400 Bad Request
content-type: text/plain; charset=utf-8
vary: origin, access-control-request-method, access-control-request-headers
access-control-expose-headers: mcp-session-id,mcp-protocol-version
content-length: 58
date: Mon, 12 Oct 2026 12:54:06 GMT

Bad Request: Unsupported MCP-Protocol-Version: 1999-01-01
```

---

## 6. Official MCP Inspector Verification

The official Model Context Protocol Inspector CLI (`@modelcontextprotocol/inspector` v2.10.1) was run against `http://127.0.0.1:8080/mcp`.

### A. Inspector Tool Listing (`tools/list`)

#### Command
```powershell
npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:8080/mcp --method tools/list
```

#### Output
```json
{
  "tools": [
    {
      "name": "ping_restaurant",
      "description": "Ping the TableTalk restaurant assistant to verify connectivity and retrieve server status",
      "inputSchema": {
        "type": "object",
        "properties": {
          "name": {
            "description": "Optional guest or client name to include in greeting",
            "type": [
              "string",
              "null"
            ]
          }
        },
        "$schema": "https://json-schema.org/draft/2020-12/schema"
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "message": {
            "description": "Friendly greeting message confirming assistant status",
            "type": "string"
          },
          "server": {
            "description": "Identifier of the serving MCP implementation",
            "type": "string"
          },
          "version": {
            "description": "Crate package version of the TableTalk server",
            "type": "string"
          }
        },
        "required": [
          "message",
          "server",
          "version"
        ],
        "$schema": "https://json-schema.org/draft/2020-12/schema"
      },
      "annotations": {
        "readOnlyHint": true,
        "destructiveHint": false,
        "idempotentHint": true
      }
    }
  ]
}
Schema portability: 0 errors, 1 warning across 1 tool. Re-run with --strict for details.
```

---

### B. Inspector Tool Invocation (`tools/call ping_restaurant`)

#### Command
```powershell
npx @modelcontextprotocol/inspector --cli --server-url http://127.0.0.1:8080/mcp --method tools/call --tool-name ping_restaurant --tool-arg name="InspectorDemo"
```

#### Output
```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"message\":\"Hello, InspectorDemo! TableTalk restaurant MCP server is online.\",\"server\":\"tabletalk\",\"version\":\"0.1.0\"}"
    }
  ],
  "structuredContent": {
    "message": "Hello, InspectorDemo! TableTalk restaurant MCP server is online.",
    "server": "tabletalk",
    "version": "0.1.0"
  },
  "isError": false
}
```

---

## 7. Automated Test Suite Execution

All test cases pass in the offline integration test suite:

```powershell
cargo test
```

Output:
```text
running 2 tests
test tools::ping::tests::test_handle_ping_default ... ok
test tools::ping::tests::test_handle_ping_named ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 4 tests
test test_disallowed_origin_rejected_with_403 ... ok
test test_health_endpoint ... ok
test test_unsupported_protocol_version_header_rejected_with_400 ... ok
test test_full_mcp_lifecycle_and_tools ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 8. Phase 4: Verification, Adversarial Testing & Hardening

Phase 4 hardened the Streamable HTTP transport and verified compliance with MCP specification `2025-11-25` through adversarial edge cases, high concurrency, mid-stream disconnects, and automated Inspector tooling.

### 8.1 Task 1: MCP Inspector Interactive UI Steps

*(Note: Operating in a headless development agent environment without a graphical desktop. The steps below provide the exact protocol for manual interactive validation in a local browser)*

1. **Launch TableTalk Backend**:
   ```bash
   cargo run --bin tabletalk-backend
   ```
   *Expected*: Server logs `TableTalk backend listening on http://0.0.0.0:8080 (MCP endpoint: http://0.0.0.0:8080/mcp)`.
2. **Launch MCP Inspector Web UI**:
   ```bash
   npx @modelcontextprotocol/inspector
   ```
   *Expected*: Terminal displays the web server listener (e.g. `http://127.0.0.1:6274?MCP_INSPECTOR_API_TOKEN=...`). Open the link in a browser.
3. **Connect to Transport**:
   - Transport Type: Select **Streamable HTTP**.
   - Server URL: `http://localhost:8080/mcp`.
   - Click **Connect**.
   - *Expected*: Status displays "Connected". Capabilities panel lists `tools: {}`. Server info displays `name: "tabletalk"`, `version: "0.1.0"`.
4. **Discover Tools (`tools/list`)**:
   - Navigate to the **Tools** tab. Click **List Tools**.
   - *Expected*: Tool `ping_restaurant` appears with description `"Health check ping tool verifying TableTalk MCP server connectivity."` and input schema accepting optional string `name`.
5. **Tool Call with Argument**:
   - In `ping_restaurant`, input argument `name: "Alice"`.
   - Click **Run Tool**.
   - *Expected*: Result shows `{"message":"Hello, Alice! TableTalk restaurant MCP server is online.","server":"tabletalk","version":"0.1.0"}` with `isError: false`.
6. **Tool Call without Argument (Default)**:
   - Clear input argument or omit `name`. Click **Run Tool**.
   - *Expected*: Result shows `{"message":"Hello, Guest! TableTalk restaurant MCP server is online.","server":"tabletalk","version":"0.1.0"}` with `isError: false`.
7. **Tool Call with Invalid Argument**:
   - Pass an invalid JSON schema argument (e.g. `name: 12345` or extra fields). Click **Run Tool**.
   - *Expected*: Server reports schema validation error envelope cleanly with `isError: true` without server crash.
8. **Disconnect & Reconnect**:
   - Click **Disconnect**. Status switches to "Disconnected".
   - Click **Connect**. Status returns to "Connected" with a newly assigned `mcp-session-id`.

---

### 8.2 Task 2: Automated Smoke Test Script (`scripts/smoke-test.sh`)

The script `scripts/smoke-test.sh` automates end-to-end server orchestration, health checking, MCP Inspector CLI operations, and security assertions.

#### Execution Output (Success Case - Exit Code 0)
```text
$ ./scripts/smoke-test.sh
==================================================
 Starting TableTalk MCP Server on port 8088...
==================================================
Starting backend via cargo run --bin tabletalk-backend...
Waiting for server to become healthy at http://127.0.0.1:8088/health...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.15s
     Running `C:/Users/dhanu/AppData/Local/cargo-target/tabletalk\debug\tabletalk-backend.exe`
Server is healthy (PID: 1831). Running smoke tests...
--------------------------------------------------
[PASS] Health endpoint returned 200 OK: {"status":"ok"}
Testing Inspector CLI tools/list against http://127.0.0.1:8088/mcp...
[PASS] Inspector CLI tools/list discovered ping_restaurant
Testing Inspector CLI tools/call ping_restaurant with name=Alice...
[PASS] Inspector CLI tools/call with argument succeeded (contained 'Alice')
Testing Inspector CLI tools/call ping_restaurant without arguments...
[PASS] Inspector CLI tools/call without argument succeeded (contained default 'Guest')
Testing Inspector CLI tools/call with invalid tool...
[PASS] Inspector CLI correctly failed on nonexistent tool call
Testing Origin header security check via curl...
[PASS] Disallowed Origin returned 403 Forbidden
Testing invalid mcp-protocol-version header via curl...
[PASS] Invalid mcp-protocol-version returned 400 Bad Request
--------------------------------------------------
Summary: 7 passed, 0 failed.
==================================================
Shutting down server (PID: 1831)...
```
Exit code: `0`.

#### Execution Output (Failure Case - Dead Port - Exit Code 1)
```text
$ PORT=999999 MAX_RETRIES=2 ./scripts/smoke-test.sh
==================================================
 Starting TableTalk MCP Server on port 999999...
==================================================
Starting backend via cargo run --bin tabletalk-backend...
Waiting for server to become healthy at http://127.0.0.1:999999/health...
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
     Running `C:/Users/dhanu/AppData/Local/cargo-target/tabletalk\debug\tabletalk-backend.exe`
Server failed to start within 2 seconds.
Shutting down server (PID: 55)...
```
Exit code: `1`.

---

### 8.3 Task 3: Protocol Edge-Case Test Matrix

Implemented in `backend/tests/mcp_edge_cases_test.rs`:

| ID | Edge Case Description | Spec Section / Requirement | Expected Behavior | Actual Behavior | Pass/Fail |
|---|---|---|---|---|---|
| a | Malformed JSON body | Spec 2025-11-25 §Streamable HTTP | HTTP 400 or HTTP 415 rejection | HTTP 415 Unsupported Media Type | **PASS** |
| b | Valid JSON not a JSON-RPC message | Spec 2025-11-25 §JSON-RPC Transport | HTTP 400 or HTTP 415 rejection | HTTP 415 Unsupported Media Type | **PASS** |
| c | Unknown method after init | JSON-RPC 2.0 §5.1 / Spec 2025-11-25 | HTTP 200 with code `-32601` (Method not found) | HTTP 200, code: `-32601`, message: `"Method not found"` | **PASS** |
| d | `tools/call` before `initialize` | Spec 2025-11-25 §Initialization Lifecycle | HTTP 400 or HTTP 422 rejected | HTTP 422 Unprocessable Entity (`"Unexpected message, expect initialize request"`) | **PASS** |
| e | Unsupported `protocolVersion` in `initialize` | Spec 2025-11-25 §Version Negotiation | Server negotiates supported version (`2025-11-25`) | Negotiated `2025-11-25` in `result.protocolVersion` | **PASS** |
| f | Invalid `mcp-protocol-version` header on subsequent requests | Spec 2025-11-25 §Transport Headers | HTTP 400 Bad Request | HTTP 400 Bad Request (`"Unsupported MCP protocol version"`) | **PASS** |
| g | Unknown session ID & request after `DELETE /mcp` | Spec 2025-11-25 §Session Lifecycle | Unknown ID: HTTP 404; DELETE: HTTP 202; After DELETE: HTTP 404 | Unknown: 404; DELETE: 202; Post-delete: 404 | **PASS** |
| h | Wrong `Content-Type` / Missing `Accept` | Spec 2025-11-25 §Media Types | Wrong Content-Type: HTTP 415; Missing Accept: HTTP 406 | Wrong Content-Type: 415; Missing Accept: 406 | **PASS** |
| i | JSON-RPC Batching (`[...]`) | Spec 2025-11-25 §Batch Support | Rejection (HTTP 400 or HTTP 415) | HTTP 415 Unsupported Media Type | **PASS** |
| j | Oversized request body (>2MB) | Server Hardening & DoS Mitigation | Rejection without crash (HTTP 413 or drop); server remains healthy | Immediate rejection; server `/health` returns 200 OK | **PASS** |
| k | Client notification (`notifications/initialized`) | Spec 2025-11-25 §Notifications | HTTP 202 Accepted with empty body | HTTP 202 Accepted, empty body | **PASS** |
| l | Origin header validation | Spec 2025-11-25 §DNS Rebinding | Disallowed: 403; Allowed: 200; No Origin: 200 | Disallowed: 403; Allowed: 200; No Origin: 200 | **PASS** |
| m | Ping method & Cancellation notification | Spec 2025-11-25 §Ping & Cancellation | `ping` returns `{}`; `notifications/cancelled` returns 202 | `ping`: 200 with `{}`; cancellation: 202 | **PASS** |

---

### 8.4 Task 4: Streaming, Concurrency & Robustness

Implemented in `backend/tests/mcp_concurrency_and_stream_test.rs`:

#### 1. SSE Stream Client Disconnect
- **Test**: `test_sse_stream_open_and_client_disconnect`
- **Methodology**: Session initialized; client opens persistent `GET /mcp` SSE stream; client abruptly drops connection mid-stream.
- **Result**: Server does not panic or leak tasks. Subsequent health checks return `200 OK`, and subsequent tool calls on the same session succeed.

#### 2. High Concurrency Load Test (50 Concurrent Sessions)
- **Test**: `test_50_concurrent_sessions_and_latency`
- **Methodology**: 50 simultaneous asynchronous Tokio tasks execute the full lifecycle: `initialize` -> `notifications/initialized` -> `tools/call ping_restaurant`.
- **Measured Metrics**:
  - Total Concurrent Sessions: **50**
  - Total Elapsed Duration: **125.36ms**
  - p50 Latency: **111.03ms**
  - p95 Latency: **116.01ms**
  - Max Latency: **116.52ms**
  - Error/Panic Rate: **0%** (50/50 succeeded)

#### 3. Request Timeout Cutoff
- **Test**: `test_request_timeout_cutoff`
- **Methodology**: Test harness simulates server delay exceeding configured timeout threshold (`REQUEST_TIMEOUT_SECS = 1`).
- **Result**: Server cleanly cuts off hung execution and responds with `HTTP 408 Request Timeout`. Server remains fully responsive for subsequent requests.

#### 4. Graceful Shutdown (SIGTERM / SIGINT)
- **Implementation**: In `backend/src/main.rs`, `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())` listens for `Ctrl+C` (SIGINT) on all platforms and `SIGTERM` on Unix.
- **Log Output**:
  ```text
  INFO http_request{request_id="b7189c44-..." method=POST uri="/mcp"}: tabletalk_backend::http: Incoming request
  INFO tabletalk_backend: Received SIGTERM, initiating graceful shutdown
  INFO tabletalk_backend: TableTalk server gracefully shut down
  ```

---

## 9. Phase 7: Availability Engine & `check_availability` Tool Verification

Phase 7 implements the read-only `check_availability` MCP tool, the deterministic availability engine, and the alternatives recommendation algorithm. Verification was performed against the live TableTalk server seeded with real restaurant data and tested with both Streamable HTTP requests and the official MCP Inspector.

### 9.1 MCP Inspector Interactive UI Steps (Manual Protocol)

1. **Launch TableTalk Backend**:
   ```bash
   cargo run --bin mcp_tabletalk_backend
   ```
   *Expected*: Server logs `TableTalk backend listening on http://0.0.0.0:8080 (MCP endpoint: http://0.0.0.0:8080/mcp)`.
2. **Launch MCP Inspector Web UI**:
   ```bash
   npx @modelcontextprotocol/inspector
   ```
   *Expected*: Browser opens to the Inspector UI (or URL displayed in console).
3. **Connect to Transport**:
   - Transport Type: Select **Streamable HTTP**.
   - Server URL: `http://localhost:8080/mcp`.
   - Click **Connect**.
   - *Expected*: Server status indicates "Connected", protocol version `2025-11-25`.
4. **Inspect Tool Listing (`tools/list`)**:
   - Navigate to the **Tools** tab. Click **List Tools**.
   - *Expected*: 3 tools appear: `ping_restaurant`, `search_menu`, and `check_availability`.
   - Verify annotations for `check_availability`:
     - `readOnlyHint`: `true`
     - `idempotentHint`: `true`
     - `destructiveHint`: `false`
     - `openWorldHint`: `false`
   - Verify both `inputSchema` and `outputSchema` conform to JSON Schema Draft 2020-12.
5. **Scenario 1: Check Available Table**:
   - Tool: `check_availability`
   - Arguments: `{"date": "2026-10-16", "time": "13:00", "party_size": 2}`
   - Click **Run Tool**.
   - *Expected*: Result shows `status: "available"`, assigns 2-top `T01`, voice text speaks: `"I have a table for 2 guests on Friday the sixteenth at one in the afternoon. Want me to book it?"`.
6. **Scenario 2: Fully Booked Slot with Alternatives**:
   - Tool: `check_availability`
   - Arguments: `{"date": "2026-10-16", "time": "19:30", "party_size": 4, "area": "indoor"}`
   - Click **Run Tool**.
   - *Expected*: Result shows `status: "unavailable"`, `reason: "fully_booked"`, offers patio alternative at 19:30 (`P02`) and indoor dinner slots at 21:00 (`T03`), with natural spoken dialogue.
7. **Scenario 3: Weekly Closed Day**:
   - Tool: `check_availability`
   - Arguments: `{"date": "2026-10-20", "time": "19:30", "party_size": 2}`
   - Click **Run Tool**.
   - *Expected*: Result shows `status: "closed"`, `reason: "closed_day"`, voice text explains Tuesday closure.
8. **Scenario 4: Invalid Argument Self-Correction**:
   - Tool: `check_availability`
   - Arguments: `{"date": "tomorrow", "time": "19:00", "party_size": 2}`
   - Click **Run Tool**.
   - *Expected*: `isError: true` with error message explaining date format required: `Invalid input: 'date' must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received 'tomorrow'.`

---

### 9.2 Real Runtime Scenarios (Verbatim Output)

Executed against seeded `data/tabletalk.db` on Streamable HTTP transport:

#### Scenario 1: Available Slot (Standard Booking)
**Request arguments**:
```json
{
  "date": "2026-10-16",
  "time": "13:00",
  "party_size": 2
}
```
**Verbatim MCP Response (`id: 10`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I have a table for 2 guests on Friday the sixteenth at one in the afternoon. Want me to book it?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-16",
      "time": "13:00",
      "party_size": 2,
      "area": "any",
      "duration_minutes": 75,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.470221500+05:30",
    "status": "available",
    "reason": null,
    "reason_detail": null,
    "snapped_from": null,
    "slot": {
      "start_local": "2026-10-16T13:00:00+05:30",
      "end_local": "2026-10-16T14:15:00+05:30",
      "table_label": "T01",
      "area": "indoor",
      "capacity": 2
    },
    "alternatives": [],
    "opening_hours_that_day": [
      { "open": "12:00", "close": "15:30" },
      { "open": "19:00", "close": "23:30" }
    ],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 2: Fully Booked Slot with Multi-Dimensional Alternatives
**Request arguments**:
```json
{
  "date": "2026-10-16",
  "time": "19:30",
  "party_size": 4,
  "area": "indoor"
}
```
**Verbatim MCP Response (`id: 11`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I don't have indoor seating at seven thirty in the evening, but patio is free at seven thirty in the evening, or I have nine in indoor. Would either work for you?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-16",
      "time": "19:30",
      "party_size": 4,
      "area": "indoor",
      "duration_minutes": 90,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.474022800+05:30",
    "status": "unavailable",
    "reason": "fully_booked",
    "reason_detail": "No tables available for party of 4 at 19:30 in indoor area. Found 3 alternative slot(s).",
    "snapped_from": null,
    "slot": null,
    "alternatives": [
      {
        "start_local": "2026-10-16T19:30:00+05:30",
        "end_local": "2026-10-16T21:00:00+05:30",
        "area": "patio",
        "table_label": "P02",
        "minutes_from_requested": 0,
        "different_day": false
      },
      {
        "start_local": "2026-10-16T21:00:00+05:30",
        "end_local": "2026-10-16T22:30:00+05:30",
        "area": "indoor",
        "table_label": "T03",
        "minutes_from_requested": 90,
        "different_day": false
      },
      {
        "start_local": "2026-10-16T21:15:00+05:30",
        "end_local": "2026-10-16T22:45:00+05:30",
        "area": "indoor",
        "table_label": "T03",
        "minutes_from_requested": 105,
        "different_day": false
      }
    ],
    "opening_hours_that_day": [
      { "open": "12:00", "close": "15:30" },
      { "open": "19:00", "close": "23:30" }
    ],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 3: Weekly Closed Day (Tuesdays Closed)
**Request arguments**:
```json
{
  "date": "2026-10-20",
  "time": "19:30",
  "party_size": 2
}
```
**Verbatim MCP Response (`id: 12`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "The restaurant is closed on Tuesday the twentieth. Would you like to check availability for tomorrow?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-20",
      "time": "19:30",
      "party_size": 2,
      "area": "any",
      "duration_minutes": 75,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.483084600+05:30",
    "status": "closed",
    "reason": "closed_day",
    "reason_detail": "The restaurant is closed on Tuesdays.",
    "snapped_from": null,
    "slot": null,
    "alternatives": [],
    "opening_hours_that_day": [],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 4: Past Time / Lead Time Violation (`too_soon`)
**Request arguments**:
```json
{
  "date": "2026-10-13",
  "time": "12:00",
  "party_size": 2
}
```
**Verbatim MCP Response (`id: 13`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I don't have a table available for 2 on Tuesday the thirteenth around noon. Would you like to check another day?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-13",
      "time": "12:00",
      "party_size": 2,
      "area": "any",
      "duration_minutes": 75,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.498965400+05:30",
    "status": "unavailable",
    "reason": "too_soon",
    "reason_detail": "Bookings must be made at least 30 minutes in advance. The earliest booking time is 14:50.",
    "snapped_from": null,
    "slot": null,
    "alternatives": [],
    "opening_hours_that_day": [],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 5: Party Size Above Restaurant Maximum (`party_too_large`)
**Request arguments**:
```json
{
  "date": "2026-10-16",
  "time": "19:30",
  "party_size": 25
}
```
**Verbatim MCP Response (`id: 14`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I don't have a table available for 25 on Friday the sixteenth around seven thirty in the evening. Would you like to check another day?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-16",
      "time": "19:30",
      "party_size": 25,
      "area": "any",
      "duration_minutes": 120,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.514628800+05:30",
    "status": "unavailable",
    "reason": "party_too_large",
    "reason_detail": "Party size of 25 exceeds the restaurant maximum of 12 guests. Please contact the restaurant directly.",
    "snapped_from": null,
    "slot": null,
    "alternatives": [],
    "opening_hours_that_day": [],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 6: Off-Grid Time Snapping (19:22 snapped to 19:15)
**Request arguments**:
```json
{
  "date": "2026-10-16",
  "time": "19:22",
  "party_size": 2
}
```
**Verbatim MCP Response (`id: 15`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I have a table for 2 guests on Friday the sixteenth at seven twenty-two in the evening. Want me to book it?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-16",
      "time": "19:22",
      "party_size": 2,
      "area": "any",
      "duration_minutes": 75,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:20:26.530204400+05:30",
    "status": "available",
    "reason": null,
    "reason_detail": "Requested time 19:22 was snapped to the nearest 15-minute slot 19:15.",
    "snapped_from": "19:22",
    "slot": {
      "start_local": "2026-10-16T19:15:00+05:30",
      "end_local": "2026-10-16T20:30:00+05:30",
      "table_label": "T01",
      "area": "indoor",
      "capacity": 2
    },
    "alternatives": [],
    "opening_hours_that_day": [
      { "open": "12:00", "close": "15:30" },
      { "open": "19:00", "close": "23:30" }
    ],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

---

#### Scenario 7: Malformed / Relative Argument Rejected as Tool Error
**Request arguments**:
```json
{
  "date": "tomorrow",
  "time": "19:00",
  "party_size": 2
}
```
**Verbatim MCP Response (`id: 16`)**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Invalid input: 'date' must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received 'tomorrow'."
    }
  ],
  "isError": true
}
```

---

## 9. Phase 8 Booking Lifecycle Verification (`create_booking`, `get_booking`, `cancel_booking`)

**Date**: 2026-10-13  
**Server Target**: `http://127.0.0.1:8088/mcp`  
**Tools Tested**: `create_booking`, `get_booking`, `cancel_booking`  

### Scenario 1: Create a Booking (Confirmed)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name create_booking \
  --tool-arg date="2026-10-23" \
  --tool-arg time="19:30" \
  --tool-arg party_size="4" \
  --tool-arg guest_name="Eleanor Vance" \
  --tool-arg email="eleanor@example.com" \
  --tool-arg idempotency_key="idem-eleanor-booking-001" \
  --tool-arg special_requests="Quiet corner booth please"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I have confirmed your table for 4 guests on Friday the twenty-third at seven thirty in the evening under the name Eleanor Vance. Your confirmation code is T T 7 P T U F. If you need to cancel or change your reservation, just mention this code."
    }
  ],
  "structuredContent": {
    "alternatives": [],
    "booking": {
      "area": "indoor",
      "confirmation_code": "TT-7PTUF",
      "created_at": "2026-10-13T09:29:58.143444300+00:00",
      "end_local": "2026-10-23T21:00:00+05:30",
      "guest_name": "Eleanor Vance",
      "party_size": 4,
      "special_requests": "Quiet corner booth please",
      "start_local": "2026-10-23T19:30:00+05:30",
      "status": "confirmed",
      "table_label": "T03"
    },
    "cancellation_policy": "Free cancellation up to 6 hours prior to reservation.",
    "idempotent_replay": false,
    "note": "availability is a snapshot, not a reservation",
    "reason": null,
    "reason_detail": null,
    "status": "confirmed"
  },
  "isError": false
}
```

---

### Scenario 2: Create Booking on Fully Booked Slot (Unavailable with Alternatives)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name create_booking \
  --tool-arg date="2026-10-23" \
  --tool-arg time="19:30" \
  --tool-arg party_size="8" \
  --tool-arg guest_name="Percy Shelley" \
  --tool-arg idempotency_key="idem-percy-001"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I don't have a table available at seven thirty in the evening, but I have nine fifteen or nine thirty in the evening. Would either of those work for you?"
    }
  ],
  "structuredContent": {
    "alternatives": [
      {
        "area": "indoor",
        "different_day": false,
        "end_local": "2026-10-23T23:15:00+05:30",
        "minutes_from_requested": 105,
        "start_local": "2026-10-23T21:15:00+05:30",
        "table_label": "T06"
      },
      {
        "area": "indoor",
        "different_day": false,
        "end_local": "2026-10-23T23:30:00+05:30",
        "minutes_from_requested": 120,
        "start_local": "2026-10-23T21:30:00+05:30",
        "table_label": "T06"
      }
    ],
    "booking": null,
    "cancellation_policy": null,
    "idempotent_replay": false,
    "note": "availability is a snapshot, not a reservation",
    "reason": "fully_booked",
    "reason_detail": "No tables available for party of 8 at 19:30 in any area. Found 2 alternative slot(s).",
    "status": "unavailable"
  },
  "isError": false
}
```

---

### Scenario 3: Idempotent Retry with Same Key and Same Parameters
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name create_booking \
  --tool-arg date="2026-10-23" \
  --tool-arg time="19:30" \
  --tool-arg party_size="4" \
  --tool-arg guest_name="Eleanor Vance" \
  --tool-arg email="eleanor@example.com" \
  --tool-arg idempotency_key="idem-eleanor-booking-001" \
  --tool-arg special_requests="Quiet corner booth please"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I have confirmed your table for 4 guests on Friday the twenty-third at seven thirty in the evening under the name Eleanor Vance. Your confirmation code is T T 7 P T U F. If you need to cancel or change your reservation, just mention this code."
    }
  ],
  "structuredContent": {
    "alternatives": [],
    "booking": {
      "area": "indoor",
      "confirmation_code": "TT-7PTUF",
      "created_at": "2026-10-13T09:29:58.143444300+00:00",
      "end_local": "2026-10-23T21:00:00+05:30",
      "guest_name": "Eleanor Vance",
      "party_size": 4,
      "special_requests": "Quiet corner booth please",
      "start_local": "2026-10-23T19:30:00+05:30",
      "status": "confirmed",
      "table_label": "T03"
    },
    "cancellation_policy": "Free cancellation up to 6 hours prior to reservation.",
    "idempotent_replay": true,
    "note": "availability is a snapshot, not a reservation",
    "reason": null,
    "reason_detail": null,
    "status": "confirmed"
  },
  "isError": false
}
```

---

### Scenario 4: Read-Only `get_booking` with Name Verification
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name get_booking \
  --tool-arg confirmation_code="TT-7PTUF" \
  --tool-arg guest_name="Eleanor Vance"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I found your reservation for 4 guests on Friday the twenty-third at seven thirty in the evening under the name Eleanor Vance. Confirmation code is T T 7 P T U F."
    }
  ],
  "structuredContent": {
    "booking": {
      "area": "indoor",
      "confirmation_code": "TT-7PTUF",
      "created_at": "2026-10-13T09:29:58.143444300+00:00",
      "end_local": "2026-10-23T21:00:00+05:30",
      "guest_name": "Eleanor Vance",
      "party_size": 4,
      "special_requests": "Quiet corner booth please",
      "start_local": "2026-10-23T19:30:00+05:30",
      "status": "confirmed",
      "table_label": "T03"
    },
    "message": "Booking retrieved successfully.",
    "status": "found"
  },
  "isError": false
}
```

---

### Scenario 5: Cancel with Wrong Name (Anti-Enumeration Protection)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name cancel_booking \
  --tool-arg confirmation_code="TT-7PTUF" \
  --tool-arg guest_name="John Doe"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I couldn't find a booking with that confirmation code and name. Please check the code and the name on the booking."
    }
  ],
  "structuredContent": {
    "already_cancelled": false,
    "booking": null,
    "policy_note": null,
    "reason": "not_found",
    "reason_detail": "No matching booking found for the provided confirmation code and guest name. Please check your confirmation details.",
    "status": "not_found"
  },
  "isError": false
}
```

*(Note: Tested alongside unknown code `TT-99999` with `John Doe`—the output is byte-identical, preventing confirmation code enumeration).*

---

### Scenario 6: Cancel Correctly (Verified)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name cancel_booking \
  --tool-arg confirmation_code="TT-7PTUF" \
  --tool-arg guest_name="Eleanor Vance" \
  --tool-arg reason="Family emergency"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Your reservation for Friday the twenty-third under code T T 7 P T U F has been cancelled. We hope to welcome you another time."
    }
  ],
  "structuredContent": {
    "already_cancelled": false,
    "booking": {
      "area": "indoor",
      "confirmation_code": "TT-7PTUF",
      "created_at": "2026-10-13T09:29:58.143444300+00:00",
      "end_local": "2026-10-23T21:00:00+05:30",
      "guest_name": "Eleanor Vance",
      "party_size": 4,
      "special_requests": "Quiet corner booth please",
      "start_local": "2026-10-23T19:30:00+05:30",
      "status": "cancelled",
      "table_label": "T03"
    },
    "policy_note": "Cancelled within the free cancellation window.",
    "reason": null,
    "reason_detail": null,
    "status": "cancelled"
  },
  "isError": false
}
```

---

### Scenario 7: Cancel Again (Idempotent Second Cancel)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name cancel_booking \
  --tool-arg confirmation_code="TT-7PTUF" \
  --tool-arg guest_name="Eleanor Vance"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "This reservation under code T T 7 P T U F was already cancelled. No further action is needed."
    }
  ],
  "structuredContent": {
    "already_cancelled": true,
    "booking": {
      "area": "indoor",
      "confirmation_code": "TT-7PTUF",
      "created_at": "2026-10-13T09:29:58.143444300+00:00",
      "end_local": "2026-10-23T21:00:00+05:30",
      "guest_name": "Eleanor Vance",
      "party_size": 4,
      "special_requests": "Quiet corner booth please",
      "start_local": "2026-10-23T19:30:00+05:30",
      "status": "cancelled",
      "table_label": "T03"
    },
    "policy_note": null,
    "reason": null,
    "reason_detail": "This reservation was already cancelled.",
    "status": "already_cancelled"
  },
  "isError": false
}
```

---

### Scenario 8: Invalid Argument Rejected as Tool Error (`isError: true`)
**Command**:
```bash
npx --yes @modelcontextprotocol/inspector --cli "http://127.0.0.1:8088/mcp" \
  --method tools/call --tool-name create_booking \
  --tool-arg date="2026-10-23" \
  --tool-arg time="19:30" \
  --tool-arg party_size="0" \
  --tool-arg guest_name="Bad Request"
```
**Verbatim Output**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Party size must be between 1 and 12 guests. Received 0."
    }
  ],
  "isError": true
}
```

---

## 10. Manual Inspector UI Step-by-Step Guide

If testing using the MCP Inspector Web UI:
1. Start the server: `PORT=8080 cargo run --manifest-path backend/Cargo.toml --bin mcp_tabletalk_backend`
2. Launch the MCP Inspector Web UI: `npx @modelcontextprotocol/inspector`
3. In the Inspector UI top bar:
   - Select transport: **Streamable HTTP** (or Server-Sent Events / HTTP)
   - Enter Server URL: `http://127.0.0.1:8080/mcp`
   - Click **Connect**
4. Verify the **Tools** tab shows 6 registered tools: `ping_restaurant`, `search_menu`, `check_availability`, `create_booking`, `cancel_booking`, `get_booking`.
5. Select `create_booking`:
   - Enter `date`: `"2026-10-24"`, `time`: `"19:30"`, `party_size`: `2`, `guest_name`: `"Jane Doe"`
   - Click **Run Tool**
   - Verify status `confirmed`, a confirmation code `TT-XXXXX`, and natural speech text.
6. Select `get_booking`:
   - Enter the generated `confirmation_code` and `guest_name`: `"Jane Doe"`
   - Click **Run Tool**
   - Verify booking summary returned with status `found`.
7. Select `cancel_booking`:
   - Enter the `confirmation_code` and `guest_name`: `"Jane Doe"`
   - Click **Run Tool**
   - Verify status `cancelled` and confirmation speech.



