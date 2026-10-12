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
