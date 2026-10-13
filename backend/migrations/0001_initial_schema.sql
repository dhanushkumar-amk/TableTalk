-- TableTalk Initial Schema Migration
-- SQLite 3.35+ with foreign keys and strict constraints
-- ISO-8601 UTC string timestamps and integer minor unit currency (cents/paise)

-- 1. Restaurant Configuration (Single-Row Table)
CREATE TABLE IF NOT EXISTS restaurant (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    timezone TEXT NOT NULL,
    currency TEXT NOT NULL,
    dining_duration_minutes INTEGER NOT NULL CHECK (dining_duration_minutes > 0),
    slot_granularity_minutes INTEGER NOT NULL CHECK (slot_granularity_minutes > 0),
    max_party_size INTEGER NOT NULL CHECK (max_party_size > 0),
    lead_time_minutes INTEGER NOT NULL CHECK (lead_time_minutes >= 0),
    booking_horizon_days INTEGER NOT NULL CHECK (booking_horizon_days > 0),
    deposit_required_party_size INTEGER NOT NULL DEFAULT 6 CHECK (deposit_required_party_size > 0),
    deposit_amount_per_person_cents INTEGER NOT NULL DEFAULT 50000 CHECK (deposit_amount_per_person_cents >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- 2. Operating Hours (0 = Sunday, 1 = Monday, ..., 6 = Saturday)
CREATE TABLE IF NOT EXISTS opening_hours (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    day_of_week INTEGER NOT NULL CHECK (day_of_week BETWEEN 0 AND 6),
    period_name TEXT NOT NULL,
    open_time TEXT NOT NULL,
    close_time TEXT NOT NULL,
    is_closed INTEGER NOT NULL DEFAULT 0 CHECK (is_closed IN (0, 1)),
    CHECK (is_closed = 1 OR open_time < close_time)
);
CREATE INDEX IF NOT EXISTS idx_opening_hours_day ON opening_hours(day_of_week);

-- 3. Special Closures (Holidays, private events, emergency closures)
CREATE TABLE IF NOT EXISTS special_closures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    closure_date TEXT NOT NULL,
    reason TEXT NOT NULL,
    is_all_day INTEGER NOT NULL DEFAULT 1 CHECK (is_all_day IN (0, 1)),
    open_time TEXT,
    close_time TEXT
);

-- 4. Dining Tables
CREATE TABLE IF NOT EXISTS dining_tables (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label TEXT NOT NULL UNIQUE,
    capacity_min INTEGER NOT NULL CHECK (capacity_min > 0),
    capacity_max INTEGER NOT NULL CHECK (capacity_max >= capacity_min),
    area TEXT NOT NULL CHECK (area IN ('indoor', 'patio', 'bar')),
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1))
);

-- 5. Menu Categories
CREATE TABLE IF NOT EXISTS menu_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    display_order INTEGER NOT NULL DEFAULT 0
);

-- 6. Menu Items
CREATE TABLE IF NOT EXISTS menu_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES menu_categories(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    price_cents INTEGER NOT NULL CHECK (price_cents >= 0),
    currency TEXT NOT NULL,
    is_available INTEGER NOT NULL DEFAULT 1 CHECK (is_available IN (0, 1)),
    prep_time_minutes INTEGER NOT NULL CHECK (prep_time_minutes >= 0),
    image_url TEXT,
    spice_level INTEGER NOT NULL CHECK (spice_level BETWEEN 0 AND 3),
    calories INTEGER CHECK (calories IS NULL OR calories >= 0),
    is_chefs_special INTEGER NOT NULL DEFAULT 0 CHECK (is_chefs_special IN (0, 1))
);
CREATE INDEX IF NOT EXISTS idx_menu_items_category_avail ON menu_items(category_id, is_available);

-- 7. Dietary Tags (Vegan, Vegetarian, Gluten-Free, Halal, etc.)
CREATE TABLE IF NOT EXISTS dietary_tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL
);

-- 8. Allergens (Peanuts, Tree nuts, Dairy, Gluten, Shellfish, etc.)
-- Allergens and dietary tags are intentionally kept distinct.
CREATE TABLE IF NOT EXISTS allergens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL
);

-- 9. Join Table: Menu Item to Dietary Tags
CREATE TABLE IF NOT EXISTS menu_item_dietary_tags (
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES dietary_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (menu_item_id, tag_id)
);

-- 10. Join Table: Menu Item to Allergens
CREATE TABLE IF NOT EXISTS menu_item_allergens (
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id) ON DELETE CASCADE,
    allergen_id INTEGER NOT NULL REFERENCES allergens(id) ON DELETE CASCADE,
    PRIMARY KEY (menu_item_id, allergen_id)
);

-- 11. Customers
-- Note: Synthetic demo data only. Contains fictitious identifiers and synthetic contact details.
-- Real customer PII requires strict consent records, encryption at rest, and retention schedules.
CREATE TABLE IF NOT EXISTS customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    display_name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    created_at TEXT NOT NULL
);

-- 12. Customer Preferences (Session Memory & Personalization)
CREATE TABLE IF NOT EXISTS customer_preferences (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('allergy', 'dietary', 'seat_pref', 'favorite_item', 'note')),
    value TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('voice', 'explicit', 'history')),
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_customer_preferences_customer ON customer_preferences(customer_id);

-- 13. Bookings
CREATE TABLE IF NOT EXISTS bookings (
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
    deposit_status TEXT NOT NULL DEFAULT 'none' CHECK (deposit_status IN ('none', 'required', 'paid', 'refunded', 'forfeited')),
    deposit_amount_cents INTEGER NOT NULL DEFAULT 0 CHECK (deposit_amount_cents >= 0),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    cancelled_at TEXT,
    CHECK (end_at > start_at)
);
CREATE INDEX IF NOT EXISTS idx_bookings_time_table ON bookings(table_id, start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_bookings_customer ON bookings(customer_id);
CREATE INDEX IF NOT EXISTS idx_bookings_status ON bookings(status);

-- 14. Orders
CREATE TABLE IF NOT EXISTS orders (
    id TEXT PRIMARY KEY,
    customer_id INTEGER REFERENCES customers(id) ON DELETE SET NULL,
    booking_id TEXT REFERENCES bookings(id) ON DELETE SET NULL,
    order_type TEXT NOT NULL CHECK (order_type IN ('dine_in', 'pickup')),
    status TEXT NOT NULL CHECK (status IN ('draft', 'placed', 'preparing', 'ready', 'completed', 'cancelled')),
    pickup_time TEXT,
    subtotal_cents INTEGER NOT NULL CHECK (subtotal_cents >= 0),
    tax_cents INTEGER NOT NULL CHECK (tax_cents >= 0),
    tip_cents INTEGER NOT NULL DEFAULT 0 CHECK (tip_cents >= 0),
    total_cents INTEGER NOT NULL CHECK (total_cents >= 0),
    currency TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id);
CREATE INDEX IF NOT EXISTS idx_orders_booking ON orders(booking_id);

-- 15. Order Line Items
CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    menu_item_id INTEGER NOT NULL REFERENCES menu_items(id) ON DELETE RESTRICT,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price_cents INTEGER NOT NULL CHECK (unit_price_cents >= 0),
    notes TEXT
);

-- 16. Payments (Simulated Ledger Only - Never store PAN / cardholder data)
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY,
    booking_id TEXT REFERENCES bookings(id) ON DELETE SET NULL,
    order_id TEXT REFERENCES orders(id) ON DELETE SET NULL,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    currency TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('requires_action', 'authorized', 'captured', 'refunded', 'failed')),
    provider_reference TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CHECK (booking_id IS NOT NULL OR order_id IS NOT NULL)
);

-- 17. Audit Events
CREATE TABLE IF NOT EXISTS audit_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    actor TEXT NOT NULL CHECK (actor IN ('mcp', 'seed', 'system', 'admin')),
    payload_json TEXT,
    created_at TEXT NOT NULL
);
