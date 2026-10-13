-- Migration 0004: Order Updates and Idempotency Scope
-- SQLite 3.35+ with foreign keys and strict constraints

-- 1. Order table additions for pickup orders, guest tracking and cancellation
ALTER TABLE orders ADD COLUMN order_code TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_orders_order_code ON orders(order_code);
ALTER TABLE orders ADD COLUMN guest_name TEXT NOT NULL DEFAULT '';
ALTER TABLE orders ADD COLUMN guest_phone TEXT;
ALTER TABLE orders ADD COLUMN guest_email TEXT;
ALTER TABLE orders ADD COLUMN stated_allergens TEXT;
ALTER TABLE orders ADD COLUMN cancelled_at TEXT;

-- 2. Order line items additions for immutable price/name snapshots
ALTER TABLE order_items ADD COLUMN item_name TEXT NOT NULL DEFAULT '';
ALTER TABLE order_items ADD COLUMN currency TEXT NOT NULL DEFAULT 'INR';

-- 3. Restaurant config updates for tax, kitchen lead buffer, and contact phone
ALTER TABLE restaurant ADD COLUMN tax_rate_basis_points INTEGER NOT NULL DEFAULT 500;
ALTER TABLE restaurant ADD COLUMN last_order_before_close_minutes INTEGER NOT NULL DEFAULT 30;
ALTER TABLE restaurant ADD COLUMN phone TEXT NOT NULL DEFAULT '+91 80 4567 8900';

-- 4. Idempotency key scope: recreate table with (tool_name, key) unique constraint and order_id reference
CREATE TABLE idempotency_keys_new (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    request_hash TEXT NOT NULL,
    response_json TEXT NOT NULL,
    booking_id TEXT REFERENCES bookings(id) ON DELETE CASCADE,
    order_id TEXT REFERENCES orders(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    UNIQUE(tool_name, key)
);

INSERT INTO idempotency_keys_new (id, key, tool_name, request_hash, response_json, booking_id, order_id, created_at, expires_at)
SELECT id, key, tool_name, request_hash, response_json, booking_id, NULL, created_at, expires_at FROM idempotency_keys;

DROP TABLE idempotency_keys;
ALTER TABLE idempotency_keys_new RENAME TO idempotency_keys;

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_tool_key ON idempotency_keys(tool_name, key);
CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires_at ON idempotency_keys(expires_at);
