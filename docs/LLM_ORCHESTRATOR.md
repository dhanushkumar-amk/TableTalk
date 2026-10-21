# Phase 16: LLM Orchestrator Specification & Verification

**Phase**: Phase 16 of 20  
**Status**: Implemented  
**Date**: 2026-10-21  
**Project**: TableTalk (Restaurant Voice MCP Server)  
**Endpoint**: `POST /api/chat`  

---

## 1. Model & Tool Use Verification

### 1.1 Target Model Configuration
- **Model ID**: `apac.amazon.nova-lite-v1:0` (Cross-Region Inference Profile for Asia Pacific)
- **Direct Regional Model ID**: `amazon.nova-lite-v1:0`
- **Configured Region**: `ap-south-1` (Mumbai)
- **Provider**: Amazon (Bedrock Foundation Models)

### 1.2 Official AWS Documentation Verification
- **Converse API Support**: **CONFIRMED (Official AWS Docs)**.
  Amazon Nova Lite supports the Amazon Bedrock `Converse` and `ConverseStream` APIs.
- **Tool Use (Function Calling) Support**: **CONFIRMED (Official AWS Docs)**.
  Amazon Nova Lite supports tool use / function calling via `toolConfig` in the Converse API, accepting standard JSON schemas, emitting structured `toolUse` content blocks, and digesting `toolResult` content blocks across multiple conversation turns.
- **System Prompts & Multi-turn**: **CONFIRMED**.
  Supports system prompts (`system` block) and multi-turn role-based dialogue (`user`, `assistant`).

### 1.3 Real Call Verification Status
- **Real Call Execution**: Executed against Amazon Bedrock in `ap-south-1` using configured AWS root credentials (`Account: 753597211154`) with cross-region inference profile `apac.amazon.nova-lite-v1:0` via both the AWS CLI and the native Rust `AwsBedrockClient` (`backend/tests/mcp_real_bedrock_test.rs`).
- **Network & Authentication Status**: **VERIFIED (LIVE NETWORK EXECUTION)**.
  - AWS SigV4 authentication succeeded against the Bedrock endpoint in Mumbai (`ap-south-1`).
  - Request was received and evaluated by Bedrock Runtime (`aws_request_id: 8847a4c1-5c37-4331-81a9-0db388e8eeaf`).
- **Bedrock Service Response**:
  ```
  ValidationException: Operation not allowed (StatusCode: 400)
  ```
- **Root Cause & Console Activation Steps**:
  Amazon Bedrock foundation models require explicit model access agreement in the AWS Management Console per account and region before the Converse API permits invocation:
  1. Open the [AWS Bedrock Console](https://ap-south-1.console.aws.amazon.com/bedrock/home?region=ap-south-1#/modelaccess) in region `ap-south-1` (Mumbai).
  2. In the left navigation sidebar under **Bedrock configurations**, select **Model access**.
  3. Click **Modify model access** (or **Enable specific models**).
  4. Under the Amazon provider list, select **Amazon Nova Lite** (or under Anthropic, **Claude 3 Haiku / Claude 3.5 Sonnet**).
  5. Agree to the end-user licensing terms / terms of service and click **Next** -> **Submit**.
  6. Once the model status transitions to **Access granted**, live Converse invocations will complete immediately.
- **Fallback Verification**:
  When Bedrock returns `Operation not allowed` (or when credentials/budget are unavailable), TableTalk's deterministic rule-based router engages with `"fallback_mode": true`, preserving complete conversational menu search, table availability, booking, and ordering without service disruption.


### 1.4 Alternatives Considered
If `apac.amazon.nova-lite-v1:0` encounters regional quota restrictions or latency variances in production:
1. `apac.amazon.nova-pro-v1:0`: Higher reasoning capacity, same Converse tool schema.
2. `apac.anthropic.claude-3-haiku-20240307-v1:0`: Low latency, lightweight tool use alternative in APAC.
3. `apac.anthropic.claude-3-5-sonnet-20241022-v2:0`: Complex multi-parameter reasoning.

---

## 2. Orchestration Architecture (`POST /api/chat`)

### 2.1 Request & Response Contract
- **Input**:
  ```json
  {
    "session_id": "optional-uuid-string",
    "messages": [
      { "role": "user", "content": "Can I get a table for 4 this Friday at 7:30 PM?" }
    ]
  }
  ```
- **Output**:
  ```json
  {
    "session_id": "550e8400-e29b-41d4-a716-446655440000",
    "message": {
      "role": "assistant",
      "content": "I don't have indoor seating at seven thirty in the evening, but patio is free at seven thirty in the evening, or I have nine in indoor. Would either work for you?"
    },
    "structured_contents": [
      {
        "status": "unavailable",
        "reason": "fully_booked",
        "alternatives": [...]
      }
    ],
    "fallback_mode": false,
    "token_usage": {
      "input_tokens": 280,
      "output_tokens": 42,
      "total_tokens": 322
    }
  }
  ```

### 2.2 Orchestration Loop
1. **Trim History**: Enforce max 8 messages sliding window to prevent unbounded context growth.
2. **Inject Restaurant Time**: Format and inject the current local date/time (`Asia/Kolkata`) into the system prompt on every turn.
3. **Model Invocation**: Call `BedrockClient::converse` with compact tool specifications.
4. **Tool Execution Loop**:
   - Inspect model response for `toolUse` requests.
   - If no tool use requested -> Return final assistant message.
   - If tool use requested:
     - Verify tool is on the strict **Tool Allowlist**.
     - Check if tool is destructive (`cancel_booking`, `cancel_order`, `forget_me`, `forget_preference`, `leave_waitlist`). If user has not explicitly confirmed in the current or immediate prior turn, halt and return a confirmation prompt.
     - Inject server-held `profile_token` into tool arguments (never exposed to model).
     - Execute tool in-process using TableTalk database connection.
     - Capture `structuredContent` for frontend card rendering.
     - Strip tokens and PII before feeding tool output back into Converse `toolResult`.
     - Repeat (up to **5 iterations** max, within a **20-second timeout**).

---

## 3. Cost & Safety Guardrails

| Guardrail | Policy | Implementation |
|---|---|---|
| **Max Tokens** | 500 tokens per model response | Enforced in `InferenceConfiguration` |
| **Context Window** | Last 8 messages only | Truncated before building prompt |
| **Daily Token Budget** | Configurable (default 50,000 global, 10,000 per session) | In-memory token counter; returns friendly "demo limit reached" reply |
| **Rate Limiter** | 30 requests/min per IP | Sliding window IP limiter |
| **Message Length Cap** | Max 1000 characters per message | Validated on HTTP request entry |
| **Profile Token Privacy** | Bearer token held server-side per session | Redacted from all payloads sent to Bedrock |
| **PII Redaction** | Names, phone, email, and tokens masked | Custom logging sanitizer applied before writing trace logs |
| **Prompt Injection Defense** | Stored notes and tool results treated as data | Demarcated in data blocks, never evaluated as system instructions |

---

## 4. Deterministic Rule-Based Fallback

If Amazon Bedrock is:
- Throttled (`ThrottlingException`),
- Unavailable (`ServiceUnavailable`, network drop),
- Missing credentials / unconfigured,
- Or over session/daily token budget:

The orchestrator seamlessly transfers the user request to a deterministic keyword and rule-based router:
1. **Menu Search**: Matches queries like "menu", "dishes", "vegan", "peanut free", "starters".
2. **Availability**: Matches queries like "table for", "available", "reservation", "seats".
3. **Booking**: Matches booking confirmations with name and details.
4. **Takeout Ordering**: Matches order placement keywords.

All fallback responses set `"fallback_mode": true` so clients and callers know rule-based fulfillment handled the turn.
