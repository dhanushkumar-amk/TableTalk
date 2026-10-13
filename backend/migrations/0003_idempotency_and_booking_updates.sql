-- Migration 0003: Idempotency Keys and Booking Contact/Policy Updates
-- SQLite 3.35+ with foreign keys and strict constraints

-- 18. Idempotency Keys
-- Stores idempotency records for create_booking to ensure at-most-once execution on retries.
-- Keys expire after 24 hours (documented retention policy).
CREATE TABLE IF NOT EXISTS idempotency_keys (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    tool_name TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response_json TEXT NOT NULL,
    booking_id TEXT REFERENCES bookings(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_key ON idempotency_keys(key);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires_at ON idempotency_keys(expires_at);

-- Optional guest contact details for booking verification and abuse tracking
ALTER TABLE bookings ADD COLUMN guest_phone TEXT;
ALTER TABLE bookings ADD COLUMN guest_email TEXT;

-- Policy configuration on restaurant table
ALTER TABLE restaurant ADD COLUMN cancellation_cutoff_hours INTEGER NOT NULL DEFAULT 6;
ALTER TABLE restaurant ADD COLUMN late_cancellation_fee_cents INTEGER NOT NULL DEFAULT 0;
