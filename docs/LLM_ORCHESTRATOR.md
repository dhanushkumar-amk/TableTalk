# Phase 16-17: LLM Orchestrator Specification & Verification

**Phases**: Phase 16-17 Combined  
**Status**: Implemented  
**Date**: 2026-10-22  
**Project**: TableTalk (Restaurant Voice MCP Server)  
**Endpoint**: `POST /api/chat`  
**Default Provider**: `groq` (configurable via `LLM_PROVIDER=groq | bedrock`)  

---

## 1. Provider Switch Architecture (`LlmClient`)

TableTalk implements a unified, provider-agnostic `LlmClient` trait:
```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    fn provider_name(&self) -> &'static str;

    async fn converse(
        &self,
        model_id: &str,
        system_prompt: &str,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
        max_tokens: i32,
    ) -> Result<ConverseOutput, LlmError>;
}
```

The orchestrator service loop is 100% shared between both providers. Tool definitions are dynamically converted to each provider's dialect, tool executions run in-process, results are fed back into multi-turn dialogue, and structured results are extracted for UI card rendering.

---

## 2. Groq Verification & Official Documentation Findings

### 2.1 API Format
- **Protocol**: OpenAI-compatible REST API.
- **Endpoint**: `POST https://api.groq.com/openai/v1/chat/completions`
- **Authentication**: `Authorization: Bearer $GROQ_API_KEY`
- **Tool Use Syntax**: Standard JSON Schema function calling via `tools` array with `type: "function"` and `tool_choice: "auto"`.
- **Model Output**: Emits `choices[0].message.tool_calls` containing unique `id`, `function.name`, and JSON-encoded `function.arguments`.

### 2.2 Selected Models for Tool Calling
1. **Primary Model**: `llama-3.3-70b-versatile`
   - **Context Window**: 131,072 tokens
   - **Max Completion Tokens**: 32,768 tokens
   - **Inference Speed**: ~280 tokens/second (LPU Inference Engine)
   - **Tool Calling Support**: **VERIFIED (Official Groq Docs)**. Native function calling support with JSON schema parameter validation.
2. **Secondary / Lightweight Model**: `llama-3.1-8b-instant`
   - **Context Window**: 131,072 tokens
   - **Inference Speed**: ~750 tokens/second
   - **Tool Calling Support**: **VERIFIED**.

### 2.3 Streaming Support
- **SSE Streaming**: Supported on Groq via `"stream": true` yielding chunks formatted as `data: {...}`.
- **TableTalk Chat API Usage**: `POST /api/chat` operates synchronously (`stream: false`) in order to return the unified JSON response containing `assistant` spoken text, `structured_contents` (for rich frontend card rendering), `provider` identifier, and `fallback_mode` flag.

### 2.4 Rate Limits (Free Plan / Developer Base)
- **Requests Per Minute (RPM)**: 30 RPM
- **Requests Per Day (RPD)**: 1,000 RPD
- **Tokens Per Minute (TPM)**: 12,000 TPM
- **Tokens Per Day (TPD)**: 100,000 TPD
- **Throttling Behavior**: Groq emits HTTP `429 Too Many Requests` with a JSON payload (`Rate limit reached`).
- **Resilience Strategy**:
  1. Automated retry with exponential backoff on HTTP 429 and 5xx (up to 3 attempts, 500ms initial backoff).
  2. If throttled beyond backoff threshold, seamless fallback to the deterministic keyword router with `"fallback_mode": true`.

### 2.5 Verification Status & UNVERIFIED Items
- **Tool Calling Format & Schema**: **VERIFIED** via official Groq documentation and mock integration test suite (`test_groq_client_mock_server_flow_and_429_retry`).
- **UNVERIFIED Items**:
  - The exact plan/tier and rate limit headroom of the user's personal Groq account until exercised with a live API key.
  - Network latency variability from local execution to Groq edge data centers.

---

## 3. Amazon Bedrock Provider Status

- **Configuration**: Managed via `AWS_REGION` and `BEDROCK_MODEL_ID` in `.env`.
- **Status**: **Implemented, not exercised against real AWS in this demo phase** (Bedrock client is fully implemented behind `LlmClient` and validated against fake unit/integration tests).
- **Fallback Verification**: When unconfigured or unauthorized, Bedrock routes immediately to the deterministic fallback router.

---

## 4. Orchestration Loop & Safety Guardrails

### 4.1 Orchestration Loop Steps
1. **Sanitize & Trim**: Trim dialogue history to the last 8 messages. Scrub profile tokens (`tt_prof_...`) and Groq API keys (`gsk_...`) from all messages sent to the model.
2. **Dynamic System Prompt**: Injects current restaurant local time (`Asia/Kolkata`) on every single turn.
3. **Execution Cap**: Maximum 5 iterations per request to prevent runaway loops.
4. **Timeout**: 20-second hard timeout per request.
5. **Tool Allowlist & Safety**:
   - Strict allowlist of permitted restaurant tools (no payment or financial transaction tools).
   - Server-side bearer `profile_token` injection into database tools; token is never passed to or stored in LLM contexts.
   - Destructive tools (`cancel_booking`, `cancel_order`, `forget_me`, `forget_preference`, `leave_waitlist`) require an explicit user confirmation turn before execution.
6. **PII & Key Redaction**: `redact_for_logging` scrubs phone numbers, emails, profile tokens, and API keys before logging.

### 4.2 Deterministic Rule-Based Fallback
If the active provider (Groq or Bedrock) is unavailable, unconfigured, throttled (429), or exceeds token limits:
- The request is routed to `handle_fallback` which supports:
  - Menu search (dietary, vegan, peanut-free, categories)
  - Table availability checking
  - Booking confirmation
  - Pickup ordering
- Returns `"fallback_mode": true` and `"provider": "fallback"` so the UI displays the "Demo mode" badge while the demo functions flawlessly.
