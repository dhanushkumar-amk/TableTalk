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

## AWS Services

- **What I used it for**: 
- **What worked well**: 
- **What needs work**: 
- **Onboarding experience**: 
- **Would I build with it again**: [Yes / No] — [Why]

---

## Kiro / AI Tools

- **What I used it for**: 
- **What worked well**: 
- **What needs work**: 
- **Onboarding experience**: 
- **Would I build with it again**: [Yes / No] — [Why]
