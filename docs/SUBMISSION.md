# Devpost Submission Kit: TableTalk

> Ready-to-paste submission kit for the Amazon Developer Hackathon (Alexa+ Track).

---

## 1. Project Name & Tagline

- **Project Name**: TableTalk
- **Tagline**: Voice-first restaurant intelligence powered by the Model Context Protocol (MCP) — featuring smart table negotiation, short-lived holds, and allergen-safe dining.

---

## 2. Text Description (Devpost Project Story)

### The Problem
Booking a restaurant over voice should feel like talking to an attentive maitre d'. Yet today's voice assistants routinely hit dead ends. Ask for a table for four on Friday at 7:30 PM, and if that exact slot is taken, the assistant simply replies: *"No tables available."* Diners are left guessing alternative times, repeating their party details, or giving up and opening an app. Worse, dietary questions often receive vague answers or false promises of safety, and personal preferences are forgotten the moment the session ends.

### What It Does
TableTalk transforms voice dining into a fluid, conversational experience. Instead of binary yes/no answers, TableTalk actively negotiates: when your preferred 7:30 PM slot is full, it immediately evaluates nearby openings, ranks alternatives using deterministic scoring, and places a 5-minute atomic hold so your table isn't snatched while you decide. It provides transparent allergen disclosures, calculates pickup orders with integer paise arithmetic, and remembers frequent orders — strictly with diner consent, backed by a permanent "Forget Me" hard deletion guarantee.

### How It Works
TableTalk is architected as a high-performance Model Context Protocol (MCP) server written in Rust (Axum, Tokio, rmcp) communicating over Streamable HTTP (spec version `2025-11-25`). It exposes 24 specialized MCP tools organized across five key domains:
1. **Core & Availability**: Real-time dining room capacity checking (`check_availability`) and health telemetry.
2. **Negotiation & Waitlist**: Smart table negotiation (`negotiate_booking`), short-lived holds (`accept_offer`), and FIFO dining waitlists (`join_waitlist`).
3. **Allergen-Safe Menu Search**: Transparent dietary filtering (`search_menu`) that actively flags cross-contact risks and bans misleading claims like "100% safe" or "allergy-free".
4. **Pickup Ordering**: Checked integer money calculations (`place_order`) preventing currency drift and estimating live kitchen prep times.
5. **Memory & Privacy**: Opt-in preference memory (`enable_memory`), SHA-256 hashed bearer profile tokens, and hard PII scrubbing (`forget_me`).

To protect diner security, TableTalk enforces the **Human-Confirms Payment Rule**: the AI assistant is strictly prohibited from touching credit card data or approving charges. When a peak reservation requires a deposit, the server generates a secure self-service link where a human explicitly confirms payment.

### How It Was Built
The backend is crafted in Rust with Axum 0.8, Tokio, and SQLite in WAL mode with `BEGIN IMMEDIATE` lock serialization to eliminate double-bookings under concurrent bursts. Tool schemas conform to JSON Schema Draft 2020-12 via `schemars`. The frontend web client is built with React 19, TypeScript, Tailwind CSS, and shadcn/ui, rendering accessible cards driven by server `structuredContent`. Conversational orchestration uses an extensible `LlmClient` trait backed by Groq's `llama-3.3-70b-versatile` for sub-300ms latency, with an alternative Amazon Bedrock client (`apac.amazon.nova-lite-v1:0`).

### Honest Limitations
- **Simulated Payments**: Financial transactions use mock checkout links; no live banking rails or payment cards are processed.
- **Single-Node SQLite**: Designed for local embedded speed with WAL mode; horizontal scaling requires migrating to PostgreSQL.
- **Polling Waitlist**: Waitlist updates currently rely on periodic status checks rather than push notifications.
- **Alexa+ UI Cards Unverified**: Because third-party custom UI card guidelines for Echo Show are not yet publicly released, visual card rendering on physical Alexa+ hardware remains unverified; spoken SSML is 100% compliant.

---

## 3. Track & Mini Challenge Selection

- **Selected Track**: **Alexa+ Track**
  - *Rationale*: TableTalk is designed from the ground up as a voice-first restaurant assistant specifically for the Alexa+ conversational paradigm, combining natural conversational negotiation with dual voice/screen card outputs.
- **Mini Challenges Recommendation**:
  - **AWS Builder Challenge**: **DO NOT CLAIM** (unless real AWS Bedrock API credits were used and verified against live AWS accounts; in our demo recording, Bedrock was implemented and tested with hermetic mocks, while the live video demo utilized Groq).
  - **Open Source Mini Challenge**: **DO NOT CLAIM** (unless a separate qualifying public open-source contribution to the MCP SDK or related upstream project was merged).

---

## 4. Product Feedback Answers (Official 5-Question Format)

### Question 1: What did you build, and which tools/APIs/SDKs did you use?
We built **TableTalk**, an intelligent restaurant assistant MCP server that powers conversational dining bookings, smart capacity negotiation, allergen-safe menu searches, and pickup ordering for voice assistants like Alexa+.

We used:
1. **Rust MCP SDK (`rmcp`)**: Protocol version `2025-11-25`, procedural macros (`#[tool]`), and Streamable HTTP integration.
2. **Streamable HTTP Transport**: Single-endpoint multiplexed request-response and Server-Sent Events (SSE).
3. **MCP Inspector CLI (`@modelcontextprotocol/inspector`)**: Protocol verification, tool schema inspection, and automated lifecycle testing.
4. **SQLx 0.8 & SQLite**: Embedded schema migrations, connection pooling with WAL mode, and `BEGIN IMMEDIATE` transaction serialization.
5. **Groq Cloud API (`llama-3.3-70b-versatile`)**: Primary live LLM inference engine for the 3-minute demo video.
6. **AWS SDK for Rust (`aws-sdk-bedrockruntime` & `aws-config`)**: Implemented Amazon Bedrock Nova Lite integration (`apac.amazon.nova-lite-v1:0`) with mock test suites.

### Question 2: What worked well?
- **Streamable HTTP Multiplexing**: Unifying requests and SSE streams onto `/mcp` eliminated the fragile dual-endpoint architecture of older MCP HTTP transports.
- **Rust Concurrency & WAL Mode**: Combining `BEGIN IMMEDIATE` with `PRAGMA busy_timeout = 5000` allowed 20 concurrent Tokio tasks to contest the same physical table with 0 deadlocks and 0 double-bookings.
- **Sub-300ms LPU Inference**: Groq's Llama 3.3 70B provided instant tool calling and natural speech responses that made voice conversations feel natural without awkward pauses.
- **Strict Protocol Validation**: HTTP status codes (HTTP 415 for invalid JSON, HTTP 422 for uninitialized requests, HTTP 403 for disallowed Origins) made debugging deterministic.

### Question 3: What needs work or caused friction?
- **Lack of Screen-Capability Discovery in MCP**: There is no standard protocol handshake for an MCP server to detect whether the host is an audio-only speaker (Echo Dot) or a screen device (Echo Show). Servers must redundantly emit dual representations on every call.
- **Pre-Release Guidance for Alexa+ Custom UI**: Documentation on how third-party developers can display custom interactive cards on Echo Show screens via MCP `structuredContent` is currently missing.
- **AWS SDK Smithy Document Types**: Mapping JSON schemas between Serde and `aws_smithy_types::Document` required cumbersome conversion boilerplate compared to standard JSON types.
- **Transitive Upstream Vulnerabilities in AWS Rust SDK**: Upstream dependencies in `aws-smithy-http-client` pull older versions of `h2` and `rustls` flagged by `cargo audit`.

### Question 4: How was your onboarding experience?
Onboarding was rapid. The Model Context Protocol 2025-11-25 specification was clear and well-structured. Compiling the Rust backend with Axum and SQLx took minimal time, and Vite + Tailwind CSS enabled fast iteration on the card rendering interface.

### Question 5: Would you build with this platform again? Why or why not?
**Yes.** The Model Context Protocol provides an open, vendor-neutral standard for connecting AI models to domain services. Pairing strongly-typed Rust MCP servers with conversational LLMs allows developers to enforce critical domain invariants (concurrency, money, privacy, allergen safety) that LLMs cannot guarantee on their own.

---

## 5. Feature Requests

1. **Host Display Capability Header (Critical)**:
   - *Proposal*: Add an optional client capability or header (e.g., `mcp-client-display: screen | voice_only`) so servers know whether to generate rich visual card payloads or optimize strictly for concise voice speech.
2. **Push Notifications for Voice Assistants (Important)**:
   - *Proposal*: Establish a standardized MCP notification path enabling servers to push proactive alerts (e.g., *"A table has opened up on your waitlist"*) back to a sleeping voice assistant session.
3. **Official Alexa+ Custom UI Card Guidance (Important)**:
   - *Proposal*: Publish reference designs and sandbox guidelines for rendering interactive widgets on Echo Show devices from MCP `structuredContent`.
4. **Standard Identity Mechanism for Streamable HTTP (Nice-to-Have)**:
   - *Proposal*: Provide an official standard for passing verified user identity claims (e.g., hashed user IDs) across Streamable HTTP without requiring full OAuth scaffolding for simple consumer skills.

---

## 6. Friction Log Summary

| Item | Task | Obstacle | Resolution / Workaround |
| :--- | :--- | :--- | :--- |
| **FL-1** | Streamable HTTP Header Compliance | Initial fetch checks returned HTTP 406 Not Acceptable. | Streamable HTTP requires `Accept: application/json, text/event-stream`; updated client headers accordingly. |
| **FL-2** | SQLite Concurrency Deadlocks | Standard `sqlx::Pool::begin()` uses `BEGIN DEFERRED`, causing lock escalation conflicts under simultaneous booking attempts. | Switched to explicit `BEGIN IMMEDIATE` with `busy_timeout = 5000ms`, cleanly serializing write queues. |
| **FL-3** | Smithy Document Conversion | AWS Bedrock SDK requires `aws_smithy_types::Document` instead of standard `serde_json::Value`. | Implemented recursive converter between Serde JSON values and Smithy Documents. |
| **FL-4** | Upstream AWS SDK Security Advisories | `cargo audit` reported 8 advisories in `aws-smithy-http-client` (h2, rustls). | Documented upstream status in `docs/PRODUCTION_READINESS.md` and added `.cargo/audit.toml` tracking. |

---

## 7. Video Upload Checklist

- [ ] Video duration is **under 3 minutes** (current script is timed at exactly 2:50).
- [ ] Uploaded to **YouTube** (set to Public or Unlisted) or **Vimeo**.
- [ ] Video title: `TableTalk — Voice-First Restaurant MCP Server (Alexa+ Track)`.
- [ ] Spoken language is **English**.
- [ ] No copyrighted background music.
- [ ] No third-party corporate logos or proprietary customer credentials displayed.
- [ ] `.env` file and terminal API keys kept off screen throughout the recording.
- [ ] Demo API key rotated after recording.

---

## 8. Final Submission Checklist

- [ ] **Public GitHub Repository**: Repo URL is accessible and detectable.
- [ ] **MIT License**: File present with copyright placeholder marked.
- [ ] **README Run Instructions**: Verified and working locally and via Docker Compose.
- [ ] **Working Demo in Video**: Video demonstrates live negotiation, holds, allergen search, and memory erasure.
- [ ] **Track Selected**: Alexa+ Track chosen.
- [ ] **Product Feedback**: Complete 5-question answers included in submission form.
- [ ] **Honest Declarations**: Groq declared as demo video LLM driver; Bedrock declared as implemented with mock coverage.
