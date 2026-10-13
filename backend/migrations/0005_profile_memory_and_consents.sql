-- 0005_profile_memory_and_consents.sql
-- Phase 10: Opt-in profile memory, consents, profile tokens, and allergy reconfirmation

-- 1. Memory Consents Table
CREATE TABLE IF NOT EXISTS memory_consents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    consent_version TEXT NOT NULL,
    granted_at TEXT NOT NULL,
    revoked_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_memory_consents_customer ON memory_consents(customer_id);

-- 2. Profile Tokens Table
CREATE TABLE IF NOT EXISTS profile_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_profile_tokens_lookup ON profile_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_profile_tokens_customer ON profile_tokens(customer_id);

-- 3. Extend customer_preferences with confirmed_at and severity
ALTER TABLE customer_preferences ADD COLUMN confirmed_at TEXT;
ALTER TABLE customer_preferences ADD COLUMN severity TEXT DEFAULT 'unknown';

-- 4. Indexes for order history and preference lookup
CREATE INDEX IF NOT EXISTS idx_orders_customer_history ON orders(customer_id, status, created_at);
CREATE INDEX IF NOT EXISTS idx_customer_preferences_lookup ON customer_preferences(customer_id, kind);
