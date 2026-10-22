# Production Readiness & Gap Analysis

> An honest assessment of TableTalk's current architecture: what is implemented and hardened for demo vs what is required for enterprise production scale.

---

## 1. Readiness Matrix: Demo vs. Enterprise Production

| Dimension | Done for Demo (Current State) | Needed for Enterprise Production | Gap Severity |
| :--- | :--- | :--- | :--- |
| **Authentication & AuthZ** | Bearer profile tokens (`tt_prof_...`) with SHA-256 hashing; Origin allowlist; IP rate limiting. | Real OAuth 2.0 / OIDC conforming to the MCP Authorization specification; mTLS between client & MCP server; role-based access control (RBAC) for restaurant staff. | **Critical** |
| **Payment Processing** | Simulated deposits (`simulate_deposit_payment`) and mock checkout UI with state machine (`unpaid` -> `paid` / `failed`). AI prohibited from approving charges. | PCI-DSS Level 1 compliant gateway (Stripe / Razorpay / Square) with webhook signatures, cryptographic idempotency, and automated refunds. | **Critical** |
| **Database Architecture** | Single-node SQLite with WAL mode, `busy_timeout = 5s`, and `BEGIN IMMEDIATE` concurrency serialization. | Distributed multi-writer database (PostgreSQL with read replicas or Amazon Aurora Serverless); automated failover; connection pooling with PgBouncer. | **High** |
| **Data Encryption** | In-transit TLS via reverse proxy (Nginx / CloudFront). Profile tokens stored as SHA-256 hashes in DB. | Field-level envelope encryption (AWS KMS / Tink) for diner PII (guest names, phone numbers) at rest; KMS key rotation policies. | **High** |
| **Observability & Metrics** | Structured tracing spans with `tracing-subscriber`; unique `x-request-id` header on every request; panic handler with CatchPanicLayer. | Distributed tracing (OpenTelemetry / AWS X-Ray); Prometheus / Grafana metrics (p95/p99 tool latencies, pool saturation); PagerDuty alerting. | **Medium** |
| **Backups & Disaster Recovery** | Scripted online backup utility (`scripts/backup.sh`) using SQLite `VACUUM INTO` with `PRAGMA quick_check` verification. | Automated point-in-time recovery (PITR); offsite immutable cloud backups (S3 Glacier with object lock); RPO < 5 min, RTO < 15 min. | **Medium** |
| **Waitlist Matching** | Polling-based waitlist status check (`get_waitlist_status`); manual offer acceptance. | Asynchronous push notifications (WebSockets / SMS / Alexa Proactive Notifications / WhatsApp) when a held table opens up. | **Medium** |
| **Load & Stress Testing** | 20-worker concurrent race tests (`The Race` in integration test suite) for table booking and pickup ordering. | Distributed load testing with k6 or Locust simulating 5,000+ concurrent diners across multiple restaurant locations. | **Medium** |
| **LLM Orchestration** | Pluggable `LlmClient` supporting Groq Llama 3.3 and AWS Bedrock Nova Lite; session token budgets; rule-based fallback router on HTTP 429. | Multi-provider active-active failover with circuit breakers; prompt caching; real-time toxicity and prompt-injection filtering. | **Low** |

---

## 2. Documented Known Limitations

### 2.1 Simulated Payments
- All deposits and order payments are simulated through the mock checkout interface.
- No real banking or card network rails are contacted.
- **Production Requirement**: Integration with Stripe Elements or hosted checkout sessions with signed webhooks.

### 2.2 Demo-Only Data
- Seed data includes a single fictional restaurant ("Spice Route Bistro") with 10 tables and 15 menu items.
- Dynamic table clustering (joining two 2-top tables into a 4-top) is simulated through fixed table inventories.
- **Production Requirement**: Multi-tenant restaurant entity model supporting restaurant chains, floor maps, and dynamic table combination rules.

### 2.3 Bearer Profile Token Handling
- The memory system uses client-held bearer tokens (`tt_prof_...`).
- Possession of the token grants access to customer preferences and "forget me" actions without second-factor authentication.
- **Production Requirement**: Binding tokens to authenticated user identities (e.g., Alexa Customer ID or OAuth user claims).

### 2.4 Single-Node SQLite Architecture
- The database is a single local file (`data/tabletalk.db`).
- While WAL mode and `BEGIN IMMEDIATE` provide flawless serialization on a single machine, it cannot scale horizontally across multi-region serverless instances.
- **Production Requirement**: Migration to Amazon Aurora PostgreSQL.

### 2.5 Polling-Only Waitlist
- When a party joins the waitlist, checking status requires calling `get_waitlist_status`.
- **Production Requirement**: Push notification channels (SMS via Amazon SNS, or Alexa Proactive Suggestions) to alert the diner when a table opens up.

### 2.6 Alexa+ Card Rendering UNVERIFIED
- The React card registry (`frontend/src/cards/`) implements rich, accessible UI cards driven by `structuredContent` adhering to MCP schemas.
- However, because Alexa+ third-party custom UI card rendering specifications are not yet publicly released for independent developers, visual rendering on physical Echo Show hardware remains **UNVERIFIED**. The system provides 100% compliant spoken SSML / plain text representations for all voice-only channels.

---

## 3. Dependency & Security Audit Findings

### 3.1 Cargo Audit Findings
As of October 2026, `cargo audit` identified 8 security advisories originating in transitive dependencies from the official AWS SDK for Rust (`aws-config` and `aws-sdk-bedrockruntime`):
- `RUSTSEC-2026-0258` (`h2` v0.3.27, unbounded empty DATA frames) via `aws-smithy-http-client`.
- `RUSTSEC-2023-0071` (`rsa` v0.9.10, Marvin timing attack) via `aws-smithy-runtime`.
- `RUSTSEC-2026-0346` / `RUSTSEC-2026-0349` (`rustls` v0.21.12) via `hyper-rustls` in `aws-smithy-http-client`.
- `RUSTSEC-2026-0098` / `RUSTSEC-2026-0099` / `RUSTSEC-2026-0104` / `RUSTSEC-2026-0382` (`rustls-webpki` v0.101.7) via `aws-smithy-http-client`.

**Mitigation & Status**:
- These are upstream dependencies maintained by AWS in `aws-smithy-http-client` awaiting newer AWS SDK releases.
- Direct TableTalk code does not expose raw HTTP/2 parsing or direct RSA decryption to untrusted network peers.
- Documented and tracked in `.cargo/audit.toml`.

### 3.2 NPM Audit Findings
- Frontend audit completed with **0 vulnerabilities**.
- All dependencies pinned with exact lockfiles.

### 3.3 Secret Scan Verification
- Full git history scanned across all 99,200+ diff lines via `scripts/secret-scan.cjs`.
- **0 secrets detected** across all commits.
