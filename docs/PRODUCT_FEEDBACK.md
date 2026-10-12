# Product Feedback

Feedback and evaluation of core developer tools and platform components used in TableTalk.

---

## Rust MCP SDK

- **What I used it for**: Evaluating protocol version compatibility (`2025-11-25`), Streamable HTTP transport integration with Axum, and macro-based tool schema generation for the TableTalk MCP server.
- **What worked well**: 
  - Direct integration with Axum via `StreamableHttpService` using standard Tower service mounting.
  - Clean procedural macros (`#[tool]`, `#[tool_handler]`, `#[tool_router]`) that derive JSON Schema Draft 2020-12 automatically with `schemars`.
  - First-class support for `2025-11-25` via `ProtocolVersion::V_2025_11_25` and built-in session generation (`mcp-session-id`).
  - Out-of-the-box compatibility with the official `@modelcontextprotocol/inspector` test suite.
- **What needs work**: 
  - Lack of a standard `prelude` module causes trial-and-error discovery of trait and macro imports.
  - Documentation around feature flags (`transport-streamable-http-server`, `schemars`, `macros`) could provide clearer end-to-end Axum examples in the README.
- **Onboarding experience**: Fast once crate features are understood. Compiles quickly and error messages from macro expansions are readable.
- **Would I build with it again**: **Yes** — The crate handles complex JSON-RPC transport framing, SSE chunking, and session state cleanly, saving dozens of hours of custom protocol scaffolding.

---

## Streamable HTTP

- **What I used it for**: Inspecting single-endpoint transport behavior, Server-Sent Events (SSE) streaming, session header tracking, and Origin validation for remote voice assistant connectivity.
- **What worked well**: 
  - Solves the dual-endpoint awkwardness of the older HTTP+SSE transport by multiplexing requests and responses on a single path (`/mcp`).
  - Clear SSE priming semantics (`id: 0`, `retry: 3000`) make reconnections resilient.
  - Stateless/stateful flexibility allows servers to support simple request-response or long-lived streams on demand.
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
