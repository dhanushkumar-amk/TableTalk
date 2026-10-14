# Coding style

- Handles money as integer minor units with checked arithmetic; never uses floats. Confidence: 0.9
- Uses basis points (integers) for percentages and rounding rather than floating point. Confidence: 0.8
- Models business flows as explicit state machines with enumerated allowed transitions; illegal transitions are rejected and tested. Confidence: 0.85
- Returns typed error variants for invalid state transitions. Confidence: 0.75
- Wraps state-changing writes in BEGIN IMMEDIATE transactions that re-check state inside the transaction. Confidence: 0.85
- Uses an injected clock for business logic instead of direct system-clock calls. Confidence: 0.8
- Requires idempotency (idempotency keys or natural idempotency) for every money/ledger-affecting operation. Confidence: 0.85
- Prefers append-only storage with balances derived from entries rather than edited in place. Confidence: 0.8
- Uses parameterized SQL only. Confidence: 0.85
- Avoids unwrap()/expect() on paths reachable from client input. Confidence: 0.8
- Adds new migrations rather than rewriting existing ones. Confidence: 0.8
