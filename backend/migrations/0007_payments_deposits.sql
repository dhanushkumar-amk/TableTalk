-- 0007_payments_deposits.sql
-- Phase 12: Simulated booking deposits.
-- Adds deposit policy config, payment sessions (opaque tokens, hashed at rest),
-- an append-only ledger, and a rebuilt bookings table with the extended
-- deposit_status enum and payment deadline column.

-- ---------------------------------------------------------------------------
-- 0. Rebuild audit_events so checkout completions performed by a human at the
--    mock checkout can be attributed to actor 'guest' (no MCP tool may do so).
-- ---------------------------------------------------------------------------
CREATE TABLE audit_events_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor IN ('mcp', 'seed', 'system', 'admin', 'guest')),
    payload_json TEXT,
    created_at TEXT NOT NULL
);
INSERT INTO audit_events_new (id, entity, entity_id, action, actor, payload_json, created_at)
SELECT id, entity, entity_id, action, actor, payload_json, created_at FROM audit_events;
DROP TABLE audit_events;
ALTER TABLE audit_events_new RENAME TO audit_events;

-- ---------------------------------------------------------------------------
-- 1. Restaurant deposit policy configuration (defaults documented in
--    docs/PAYMENTS_DESIGN.md)
-- ---------------------------------------------------------------------------
ALTER TABLE restaurant ADD COLUMN deposit_max_amount_cents INTEGER NOT NULL DEFAULT 400000
    CHECK (deposit_max_amount_cents >= 0);
ALTER TABLE restaurant ADD COLUMN deposit_payment_deadline_minutes INTEGER NOT NULL DEFAULT 15
    CHECK (deposit_payment_deadline_minutes BETWEEN 1 AND 30);
ALTER TABLE restaurant ADD COLUMN deposit_refund_full_basis_points INTEGER NOT NULL DEFAULT 10000
    CHECK (deposit_refund_full_basis_points BETWEEN 0 AND 10000);
ALTER TABLE restaurant ADD COLUMN deposit_refund_partial_basis_points INTEGER NOT NULL DEFAULT 5000
    CHECK (deposit_refund_partial_basis_points BETWEEN 0 AND 10000);
ALTER TABLE restaurant ADD COLUMN deposit_refund_full_hours INTEGER NOT NULL DEFAULT 24
    CHECK (deposit_refund_full_hours >= 0);
ALTER TABLE restaurant ADD COLUMN deposit_refund_partial_hours INTEGER NOT NULL DEFAULT 2
    CHECK (deposit_refund_partial_hours >= 0);
-- Peak windows: comma-separated weekday numbers (0=Sunday .. 6=Saturday).
-- Empty string disables peak-window deposits (seeded default).
ALTER TABLE restaurant ADD COLUMN deposit_peak_days TEXT NOT NULL DEFAULT '';
ALTER TABLE restaurant ADD COLUMN deposit_peak_start_time TEXT;
ALTER TABLE restaurant ADD COLUMN deposit_peak_end_time TEXT;
-- When 1 (seeded default), voice text says the payment is simulated.
ALTER TABLE restaurant ADD COLUMN deposit_demo_mode INTEGER NOT NULL DEFAULT 1
    CHECK (deposit_demo_mode IN (0, 1));
-- Number of declined checkout attempts before the session stops accepting retries.
ALTER TABLE restaurant ADD COLUMN deposit_decline_retry_limit INTEGER NOT NULL DEFAULT 3
    CHECK (deposit_decline_retry_limit >= 0);

-- ---------------------------------------------------------------------------
-- 2. Rebuild bookings with extended deposit_status enum, deposit_amount_minor
--    (integer minor units), payment_deadline_at and cancel_reason.
--
--    SQLite cannot alter CHECK constraints, so the table is rebuilt. Foreign
--    key children (orders.booking_id ON DELETE SET NULL and
--    idempotency_keys.booking_id ON DELETE CASCADE) would lose data during the
--    implicit DELETE fired by DROP TABLE, so their referencing columns/rows are
--    snapshotted first and restored afterwards.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS _orders_booking_snapshot AS
    SELECT id, booking_id FROM orders WHERE booking_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS _idempotency_keys_snapshot AS
    SELECT id, key, tool_name, request_hash, response_json, booking_id, order_id, created_at, expires_at
    FROM idempotency_keys;

CREATE TABLE bookings_new (
    id TEXT PRIMARY KEY,
    confirmation_code TEXT NOT NULL UNIQUE,
    customer_id INTEGER REFERENCES customers(id) ON DELETE SET NULL,
    guest_name TEXT NOT NULL,
    party_size INTEGER NOT NULL CHECK (party_size > 0),
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'confirmed', 'cancelled', 'completed', 'no_show')),
    table_id INTEGER REFERENCES dining_tables(id) ON DELETE SET NULL,
    special_requests TEXT,
    deposit_status TEXT NOT NULL DEFAULT 'not_required'
        CHECK (deposit_status IN ('not_required', 'requires_payment', 'paid', 'refunded',
                                  'partially_refunded', 'forfeited', 'expired')),
    deposit_amount_minor INTEGER NOT NULL DEFAULT 0 CHECK (deposit_amount_minor >= 0),
    payment_deadline_at TEXT,
    cancel_reason TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    cancelled_at TEXT,
    guest_phone TEXT,
    guest_email TEXT,
    CHECK (end_at > start_at)
);

INSERT INTO bookings_new (
    id, confirmation_code, customer_id, guest_name, party_size, start_at, end_at, status,
    table_id, special_requests, deposit_status, deposit_amount_minor, created_at, updated_at,
    cancelled_at, guest_phone, guest_email
)
SELECT
    id, confirmation_code, customer_id, guest_name, party_size, start_at, end_at, status,
    table_id, special_requests,
    CASE deposit_status
        WHEN 'none' THEN 'not_required'
        WHEN 'required' THEN 'requires_payment'
        ELSE deposit_status
    END,
    deposit_amount_cents, created_at, updated_at, cancelled_at, guest_phone, guest_email
FROM bookings;

DROP TABLE bookings;
ALTER TABLE bookings_new RENAME TO bookings;

-- Restore the child references nullified/cascaded by the DROP above.
UPDATE orders
SET booking_id = (SELECT booking_id FROM _orders_booking_snapshot s WHERE s.id = orders.id)
WHERE EXISTS (SELECT 1 FROM _orders_booking_snapshot s WHERE s.id = orders.id);

DELETE FROM idempotency_keys;
INSERT INTO idempotency_keys (id, key, tool_name, request_hash, response_json, booking_id, order_id, created_at, expires_at)
SELECT id, key, tool_name, request_hash, response_json, booking_id, order_id, created_at, expires_at
FROM _idempotency_keys_snapshot;

DROP TABLE _orders_booking_snapshot;
DROP TABLE _idempotency_keys_snapshot;

CREATE INDEX IF NOT EXISTS idx_bookings_time_table ON bookings(table_id, start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_bookings_customer ON bookings(customer_id);
CREATE INDEX IF NOT EXISTS idx_bookings_status ON bookings(status);
-- Deadline sweep index (lazy expiry + background sweeper)
CREATE INDEX IF NOT EXISTS idx_bookings_payment_deadline ON bookings(status, payment_deadline_at);

-- ---------------------------------------------------------------------------
-- 3. Payment sessions: one session per checkout, opaque token stored hashed.
--    A session is bound to exactly one booking and one amount, is single use
--    (status transitions open -> completed), and never outlives the booking's
--    payment deadline.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_sessions (
    id TEXT PRIMARY KEY,
    booking_id TEXT NOT NULL REFERENCES bookings(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('open', 'completed', 'expired', 'cancelled')),
    expires_at TEXT NOT NULL,
    decline_count INTEGER NOT NULL DEFAULT 0 CHECK (decline_count >= 0),
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_payment_sessions_booking ON payment_sessions(booking_id);
CREATE INDEX IF NOT EXISTS idx_payment_sessions_expiry ON payment_sessions(status, expires_at);

-- ---------------------------------------------------------------------------
-- 4. Rebuild payments with kind, provider, session link, state machine
--    statuses, payer fields (scrubbed by forget_me) and a partial unique index
--    preventing two captured deposits for the same booking.
-- ---------------------------------------------------------------------------
DROP TABLE payments;

CREATE TABLE payments (
    id TEXT PRIMARY KEY,
    booking_id TEXT REFERENCES bookings(id) ON DELETE SET NULL,
    order_id TEXT REFERENCES orders(id) ON DELETE SET NULL,
    session_id TEXT REFERENCES payment_sessions(id) ON DELETE SET NULL,
    kind TEXT NOT NULL CHECK (kind IN ('deposit', 'refund', 'order')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'mockpay',
    provider_reference TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('requires_action', 'authorized', 'captured',
                                          'partially_refunded', 'refunded', 'failed',
                                          'expired', 'voided')),
    payer_name TEXT,
    payer_contact TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (booking_id IS NOT NULL OR order_id IS NOT NULL)
);
-- At most one non-terminal-capture deposit chain per booking: prevents double captures.
CREATE UNIQUE INDEX IF NOT EXISTS idx_payments_one_captured_deposit_per_booking
    ON payments(booking_id)
    WHERE kind = 'deposit' AND status IN ('captured', 'partially_refunded', 'refunded');
CREATE INDEX IF NOT EXISTS idx_payments_booking ON payments(booking_id);
CREATE INDEX IF NOT EXISTS idx_payments_session ON payments(session_id);

-- ---------------------------------------------------------------------------
-- 5. Append-only ledger. Balances are always derived from entries; rows are
--    never updated or deleted (enforced by triggers).
--    amount_minor sign convention: positive = money in (authorize, capture),
--    negative = money out (void, refund, adjustment).
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS ledger_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    payment_id TEXT NOT NULL REFERENCES payments(id) ON DELETE RESTRICT,
    entry_type TEXT NOT NULL CHECK (entry_type IN ('authorize', 'capture', 'void', 'refund', 'adjustment')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor != 0),
    idempotency_key TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ledger_entries_payment ON ledger_entries(payment_id);
CREATE INDEX IF NOT EXISTS idx_ledger_entries_key ON ledger_entries(idempotency_key);

CREATE TRIGGER trg_ledger_entries_block_update
BEFORE UPDATE ON ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'ledger_entries is append-only: UPDATE rejected');
END;

CREATE TRIGGER trg_ledger_entries_block_delete
BEFORE DELETE ON ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'ledger_entries is append-only: DELETE rejected');
END;
