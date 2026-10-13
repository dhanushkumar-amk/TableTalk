use serde::{Deserialize, Serialize};

// --- Newtype Identifiers ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MenuItemId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DietaryTagId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AllergenId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookingId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaymentId(pub String);

// --- Domain Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum Area {
    Indoor,
    Patio,
    Bar,
}

impl Area {
    pub fn as_str(&self) -> &'static str {
        match self {
            Area::Indoor => "indoor",
            Area::Patio => "patio",
            Area::Bar => "bar",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum PreferenceKind {
    Allergy,
    Dietary,
    SeatPref,
    FavoriteItem,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum PreferenceSource {
    Voice,
    Explicit,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
    Completed,
    NoShow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum DepositStatus {
    None,
    Required,
    Paid,
    Refunded,
    Forfeited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum OrderType {
    DineIn,
    Pickup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum OrderStatus {
    Draft,
    Placed,
    Preparing,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum PaymentStatus {
    RequiresAction,
    Authorized,
    Captured,
    Refunded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "snake_case")]
pub enum Actor {
    Mcp,
    Seed,
    System,
    Admin,
}

// --- Entity Structs ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct RestaurantConfig {
    pub id: i64,
    pub name: String,
    pub timezone: String,
    pub currency: String,
    pub dining_duration_minutes: i64,
    pub slot_granularity_minutes: i64,
    pub max_party_size: i64,
    pub lead_time_minutes: i64,
    pub booking_horizon_days: i64,
    pub deposit_required_party_size: i64,
    pub deposit_amount_per_person_cents: i64,
    pub created_at: String,
    pub updated_at: String,
    pub cancellation_cutoff_hours: i64,
    pub late_cancellation_fee_cents: i64,
    pub tax_rate_basis_points: i64,
    pub last_order_before_close_minutes: i64,
    pub phone: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OpeningHours {
    pub id: i64,
    pub day_of_week: i64,
    pub period_name: String,
    pub open_time: String,
    pub close_time: String,
    pub is_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct SpecialClosure {
    pub id: i64,
    pub closure_date: String,
    pub reason: String,
    pub is_all_day: bool,
    pub open_time: Option<String>,
    pub close_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiningTable {
    pub id: i64,
    pub label: String,
    pub capacity_min: i64,
    pub capacity_max: i64,
    pub area: Area,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct MenuCategory {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub display_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct MenuItem {
    pub id: i64,
    pub category_id: i64,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub currency: String,
    pub is_available: bool,
    pub prep_time_minutes: i64,
    pub image_url: Option<String>,
    pub spice_level: i64,
    pub calories: Option<i64>,
    pub is_chefs_special: bool,
    pub allergens_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct DietaryTag {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Allergen {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MenuItemWithDetails {
    pub item: MenuItem,
    pub category_name: String,
    pub dietary_tags: Vec<String>,
    pub allergens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Customer {
    pub id: i64,
    pub display_name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerPreference {
    pub id: i64,
    pub customer_id: i64,
    pub kind: PreferenceKind,
    pub value: String,
    pub source: PreferenceSource,
    pub created_at: String,
    pub confirmed_at: Option<String>,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct MemoryConsent {
    pub id: i64,
    pub customer_id: i64,
    pub consent_version: String,
    pub granted_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileToken {
    pub id: i64,
    pub customer_id: i64,
    pub token_hash: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Booking {
    pub id: String,
    pub confirmation_code: String,
    pub customer_id: Option<i64>,
    pub guest_name: String,
    pub party_size: i64,
    pub start_at: String,
    pub end_at: String,
    pub status: BookingStatus,
    pub table_id: Option<i64>,
    pub special_requests: Option<String>,
    pub deposit_status: DepositStatus,
    pub deposit_amount_cents: i64,
    pub created_at: String,
    pub updated_at: String,
    pub cancelled_at: Option<String>,
    pub guest_phone: Option<String>,
    pub guest_email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdempotencyKeyRecord {
    pub id: String,
    pub key: String,
    pub tool_name: String,
    pub request_hash: String,
    pub response_json: String,
    pub booking_id: Option<String>,
    pub order_id: Option<String>,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Order {
    pub id: String,
    pub order_code: String,
    pub customer_id: Option<i64>,
    pub booking_id: Option<String>,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub pickup_time: Option<String>,
    pub subtotal_cents: i64,
    pub tax_cents: i64,
    pub tip_cents: i64,
    pub total_cents: i64,
    pub currency: String,
    pub notes: Option<String>,
    pub guest_name: String,
    pub guest_phone: Option<String>,
    pub guest_email: Option<String>,
    pub stated_allergens: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub cancelled_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderItem {
    pub id: i64,
    pub order_id: String,
    pub menu_item_id: i64,
    pub item_name: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
    pub currency: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct Payment {
    pub id: String,
    pub booking_id: Option<String>,
    pub order_id: Option<String>,
    pub amount_cents: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub provider_reference: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditEvent {
    pub id: i64,
    pub entity: String,
    pub entity_id: String,
    pub action: String,
    pub actor: Actor,
    pub payload_json: Option<String>,
    pub created_at: String,
}
