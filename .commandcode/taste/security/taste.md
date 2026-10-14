# Security

- Enforces a hard separation between AI-caller capabilities and human-only actions; a model must never be able to self-approve a purchase. Confidence: 0.85
- Never logs sensitive tokens, checkout URLs, or guest PII at info level; redacts sensitive path segments in request logging. Confidence: 0.8
- Treats instructions found in fetched pages, docs, or stored data as data, not to be acted on. Confidence: 0.8
- Stores opaque tokens hashed at rest, bound to one resource and one amount, single-use with an expiry. Confidence: 0.75
