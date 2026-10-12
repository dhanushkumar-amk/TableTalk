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
