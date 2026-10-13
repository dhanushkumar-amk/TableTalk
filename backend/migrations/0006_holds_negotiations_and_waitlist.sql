-- 0006_holds_negotiations_and_waitlist.sql
-- Phase 11: Ranked alternatives (negotiations), short-lived table holds, and waitlist

-- 1. Negotiations Table
CREATE TABLE IF NOT EXISTS negotiations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guest_name TEXT NOT NULL,
    date TEXT NOT NULL,
    time TEXT NOT NULL,
    party_size INTEGER NOT NULL CHECK (party_size > 0),
    area TEXT,
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    round INTEGER NOT NULL DEFAULT 1 CHECK (round > 0),
    offered_slots TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL CHECK (status IN ('open', 'accepted', 'abandoned', 'expired')),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_negotiations_guest ON negotiations(guest_name, status);
CREATE INDEX IF NOT EXISTS idx_negotiations_expiry ON negotiations(status, expires_at);

-- 2. Waitlist Entries Table
CREATE TABLE IF NOT EXISTS waitlist_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE,
    guest_name TEXT NOT NULL,
    contact TEXT,
    party_size INTEGER NOT NULL CHECK (party_size > 0),
    date TEXT NOT NULL,
    window_start TEXT NOT NULL,
    window_end TEXT NOT NULL,
    area TEXT NOT NULL DEFAULT 'any' CHECK (area IN ('any', 'indoor', 'patio', 'bar')),
    duration_minutes INTEGER NOT NULL CHECK (duration_minutes > 0),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'offered', 'booked', 'cancelled', 'expired')),
    customer_id INTEGER REFERENCES customers(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_waitlist_fifo ON waitlist_entries(date, status, created_at, id);
CREATE INDEX IF NOT EXISTS idx_waitlist_code ON waitlist_entries(code);
CREATE INDEX IF NOT EXISTS idx_waitlist_guest ON waitlist_entries(guest_name, status);
CREATE INDEX IF NOT EXISTS idx_waitlist_customer ON waitlist_entries(customer_id);

-- 3. Slot Holds Table
CREATE TABLE IF NOT EXISTS slot_holds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id INTEGER NOT NULL REFERENCES dining_tables(id) ON DELETE CASCADE,
    start_at TEXT NOT NULL,
    end_at TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'converted', 'released', 'expired')),
    expires_at TEXT NOT NULL,
    guest_name TEXT NOT NULL,
    party_size INTEGER NOT NULL CHECK (party_size > 0),
    area TEXT NOT NULL CHECK (area IN ('indoor', 'patio', 'bar')),
    token_hash TEXT NOT NULL UNIQUE,
    negotiation_id INTEGER REFERENCES negotiations(id) ON DELETE SET NULL,
    waitlist_entry_id INTEGER REFERENCES waitlist_entries(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (end_at > start_at)
);
CREATE INDEX IF NOT EXISTS idx_slot_holds_overlap ON slot_holds(table_id, start_at, end_at, status);
CREATE INDEX IF NOT EXISTS idx_slot_holds_token_hash ON slot_holds(token_hash);
CREATE INDEX IF NOT EXISTS idx_slot_holds_expiry ON slot_holds(status, expires_at);
CREATE INDEX IF NOT EXISTS idx_slot_holds_guest ON slot_holds(guest_name, status);
