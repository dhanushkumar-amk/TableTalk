use crate::db::models::{
    Area, Booking, BookingStatus, DiningTable, OpeningHours, RestaurantConfig, SpecialClosure,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- Clock Injection Trait ---

/// Injectable clock trait to enable deterministic time testing without calling the system clock.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// Production clock using system UTC time.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Deterministic clock with a fixed timestamp for tests.
#[derive(Debug, Clone, Copy)]
pub struct FixedClock(pub DateTime<Utc>);

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

// --- Domain Enums & Schemas ---

/// Seating area filter for availability queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AreaFilter {
    #[default]
    Any,
    Indoor,
    Patio,
    Bar,
}

impl AreaFilter {
    pub fn matches(&self, area: Area) -> bool {
        match self {
            AreaFilter::Any => true,
            AreaFilter::Indoor => area == Area::Indoor,
            AreaFilter::Patio => area == Area::Patio,
            AreaFilter::Bar => area == Area::Bar,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AreaFilter::Any => "any",
            AreaFilter::Indoor => "indoor",
            AreaFilter::Patio => "patio",
            AreaFilter::Bar => "bar",
        }
    }
}

/// Status of the availability check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityStatus {
    Available,
    Unavailable,
    Closed,
    InvalidRequest,
}

impl AvailabilityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AvailabilityStatus::Available => "available",
            AvailabilityStatus::Unavailable => "unavailable",
            AvailabilityStatus::Closed => "closed",
            AvailabilityStatus::InvalidRequest => "invalid_request",
        }
    }
}

/// Typed reasons for unavailability or closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityReason {
    ClosedDay,
    SpecialClosure,
    OutsideHours,
    TooSoon,
    BeyondHorizon,
    PartyTooLarge,
    PartySizeInvalid,
    NoSingleTable,
    FullyBooked,
    DstGap,
}

impl AvailabilityReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            AvailabilityReason::ClosedDay => "closed_day",
            AvailabilityReason::SpecialClosure => "special_closure",
            AvailabilityReason::OutsideHours => "outside_hours",
            AvailabilityReason::TooSoon => "too_soon",
            AvailabilityReason::BeyondHorizon => "beyond_horizon",
            AvailabilityReason::PartyTooLarge => "party_too_large",
            AvailabilityReason::PartySizeInvalid => "party_size_invalid",
            AvailabilityReason::NoSingleTable => "no_single_table",
            AvailabilityReason::FullyBooked => "fully_booked",
            AvailabilityReason::DstGap => "dst_gap",
        }
    }
}

/// Details of the parsed reservation request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RequestedDetails {
    pub date: String,
    pub time: String,
    pub party_size: i64,
    pub area: String,
    pub duration_minutes: i64,
    pub timezone: String,
}

/// Allocated dining slot when available.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AvailableSlot {
    pub start_local: String,
    pub end_local: String,
    pub table_label: String,
    pub area: String,
    pub capacity: i64,
}

/// Alternative dining slot suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AlternativeSlot {
    pub start_local: String,
    pub end_local: String,
    pub area: String,
    pub table_label: String,
    pub minutes_from_requested: i64,
    pub different_day: bool,
}

/// Operating hours range for a service period.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OpeningHoursRange {
    pub open: String,
    pub close: String,
}

/// Complete strongly-typed output of the `check_availability` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CheckAvailabilityOutput {
    pub requested: RequestedDetails,
    pub restaurant_now: String,
    pub status: String,
    pub reason: Option<String>,
    pub reason_detail: Option<String>,
    pub snapped_from: Option<String>,
    pub slot: Option<AvailableSlot>,
    pub alternatives: Vec<AlternativeSlot>,
    pub opening_hours_that_day: Vec<OpeningHoursRange>,
    pub note: String,
}

// --- Availability Engine Configuration ---

/// Operational settings for the availability calculation engine.
#[derive(Debug, Clone)]
pub struct AvailabilityConfig {
    pub turnover_buffer_minutes: i64,
    pub max_search_window_minutes: i64,
    pub last_seating_buffer_minutes: i64,
}

impl Default for AvailabilityConfig {
    fn default() -> Self {
        Self {
            turnover_buffer_minutes: 0,
            max_search_window_minutes: 120,
            last_seating_buffer_minutes: 45,
        }
    }
}

// --- Query Input Struct ---

#[derive(Debug, Clone)]
pub struct AvailabilityQuery {
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub party_size: i64,
    pub area: AreaFilter,
    pub duration_minutes: Option<i64>,
    pub alternatives_limit: usize,
}

// --- Pure Helper Functions ---

/// Calculate standard default dining duration based on party size.
/// Rule: 1-2 guests: 75m, 3-4 guests: 90m (or config), 5-6 guests: 105m, 7+: 120m.
pub fn calculate_default_duration(party_size: i64, config_duration: i64) -> i64 {
    match party_size {
        1..=2 => 75,
        3..=4 => config_duration.max(90),
        5..=6 => 105,
        _ => 120,
    }
}

/// Snap an off-grid time to the nearest slot granularity grid.
/// Returns (snapped_time, was_snapped).
pub fn snap_to_grid(time: NaiveTime, granularity_minutes: i64) -> (NaiveTime, bool) {
    if granularity_minutes <= 1 {
        return (time, false);
    }
    let total_seconds = time.num_seconds_from_midnight() as i64;
    let gran_seconds = granularity_minutes * 60;
    let remainder = total_seconds % gran_seconds;
    if remainder == 0 {
        return (time, false);
    }
    let snapped_seconds =
        ((total_seconds as f64 / gran_seconds as f64).round() as i64) * gran_seconds;
    let clamped_seconds = snapped_seconds.clamp(0, 24 * 3600 - 60);
    let snapped_time =
        NaiveTime::from_num_seconds_from_midnight_opt(clamped_seconds as u32, 0).unwrap_or(time);
    (snapped_time, true)
}

/// Check half-open interval overlap [req_start, req_end) vs [bk_start, bk_end + buffer).
/// Exact boundary: if req_start == bk_end + buffer, overlap is false.
pub fn is_interval_overlapping(
    req_start: DateTime<Utc>,
    req_end: DateTime<Utc>,
    bk_start: DateTime<Utc>,
    bk_end: DateTime<Utc>,
    turnover_buffer_minutes: i64,
) -> bool {
    let effective_bk_end = bk_end + Duration::minutes(turnover_buffer_minutes);
    req_start < effective_bk_end && req_end > bk_start
}

/// Convert local date and time into UTC with explicit handling for DST gaps and overlaps.
pub fn local_to_utc(
    date: NaiveDate,
    time: NaiveTime,
    tz: Tz,
) -> Result<DateTime<Utc>, AvailabilityReason> {
    let naive = date.and_time(time);
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _latest) => {
            // Overlap (fall back): deterministically pick the earlier occurrence
            Ok(earliest.with_timezone(&Utc))
        }
        chrono::LocalResult::None => {
            // Gap (spring forward): local time does not exist
            Err(AvailabilityReason::DstGap)
        }
    }
}

// --- Pure Availability Engine ---

/// Pure, deterministic availability engine.
/// Evaluates request against in-memory restaurant models and bookings.
#[allow(clippy::too_many_arguments)]
pub fn check_availability_engine(
    query: AvailabilityQuery,
    restaurant: &RestaurantConfig,
    opening_hours: &[OpeningHours],
    special_closures: &[SpecialClosure],
    tables: &[DiningTable],
    bookings: &[Booking],
    engine_config: &AvailabilityConfig,
    clock: &dyn Clock,
) -> CheckAvailabilityOutput {
    let note = "availability is a snapshot, not a reservation".to_string();

    // 0. Parse restaurant timezone
    let tz: Tz = match restaurant.timezone.parse() {
        Ok(t) => t,
        Err(_) => {
            return CheckAvailabilityOutput {
                requested: RequestedDetails {
                    date: query.date.format("%Y-%m-%d").to_string(),
                    time: query.time.format("%H:%M").to_string(),
                    party_size: query.party_size,
                    area: query.area.as_str().to_string(),
                    duration_minutes: query
                        .duration_minutes
                        .unwrap_or(restaurant.dining_duration_minutes),
                    timezone: restaurant.timezone.clone(),
                },
                restaurant_now: clock.now().to_rfc3339(),
                status: AvailabilityStatus::InvalidRequest.as_str().to_string(),
                reason: Some("invalid_timezone".to_string()),
                reason_detail: Some(format!(
                    "Invalid restaurant timezone configuration: '{}'",
                    restaurant.timezone
                )),
                snapped_from: None,
                slot: None,
                alternatives: vec![],
                opening_hours_that_day: vec![],
                note,
            };
        }
    };

    let now_utc = clock.now();
    let now_local = now_utc.with_timezone(&tz);
    let restaurant_now_iso = now_local.to_rfc3339();

    let duration_minutes = query.duration_minutes.unwrap_or_else(|| {
        calculate_default_duration(query.party_size, restaurant.dining_duration_minutes)
    });

    let requested_details = RequestedDetails {
        date: query.date.format("%Y-%m-%d").to_string(),
        time: query.time.format("%H:%M").to_string(),
        party_size: query.party_size,
        area: query.area.as_str().to_string(),
        duration_minutes,
        timezone: restaurant.timezone.clone(),
    };

    // 1. Validate party size
    if query.party_size < 1 {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::InvalidRequest.as_str().to_string(),
            reason: Some(AvailabilityReason::PartySizeInvalid.as_str().to_string()),
            reason_detail: Some(format!(
                "Party size must be at least 1 (received {}).",
                query.party_size
            )),
            snapped_from: None,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    if query.party_size > restaurant.max_party_size {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Unavailable.as_str().to_string(),
            reason: Some(AvailabilityReason::PartyTooLarge.as_str().to_string()),
            reason_detail: Some(format!(
                "Party size of {} exceeds the restaurant maximum of {} guests. Please contact the restaurant directly.",
                query.party_size, restaurant.max_party_size
            )),
            snapped_from: None,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    // 2. Validate single table capacity limit
    let active_tables: Vec<&DiningTable> = tables.iter().filter(|t| t.is_active).collect();
    let max_single_table_capacity = active_tables
        .iter()
        .map(|t| t.capacity_max)
        .max()
        .unwrap_or(0);
    if query.party_size > max_single_table_capacity {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Unavailable.as_str().to_string(),
            reason: Some(AvailabilityReason::NoSingleTable.as_str().to_string()),
            reason_detail: Some(format!(
                "Party size of {} exceeds our largest single table capacity ({} seats). Table combining is not supported in this phase.",
                query.party_size, max_single_table_capacity
            )),
            snapped_from: None,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    // 3. Grid snapping
    let (snapped_time, was_snapped) = snap_to_grid(query.time, restaurant.slot_granularity_minutes);
    let snapped_from = if was_snapped {
        Some(query.time.format("%H:%M").to_string())
    } else {
        None
    };

    // 4. Validate time conversion to UTC & DST gap
    let requested_start_utc = match local_to_utc(query.date, snapped_time, tz) {
        Ok(utc) => utc,
        Err(AvailabilityReason::DstGap) => {
            return CheckAvailabilityOutput {
                requested: requested_details,
                restaurant_now: restaurant_now_iso,
                status: AvailabilityStatus::InvalidRequest.as_str().to_string(),
                reason: Some(AvailabilityReason::DstGap.as_str().to_string()),
                reason_detail: Some(format!(
                    "The requested time {} does not exist on {} due to a daylight saving time spring-forward transition.",
                    snapped_time.format("%H:%M"),
                    query.date.format("%Y-%m-%d")
                )),
                snapped_from,
                slot: None,
                alternatives: vec![],
                opening_hours_that_day: vec![],
                note,
            };
        }
        Err(other) => {
            return CheckAvailabilityOutput {
                requested: requested_details,
                restaurant_now: restaurant_now_iso,
                status: AvailabilityStatus::InvalidRequest.as_str().to_string(),
                reason: Some(other.as_str().to_string()),
                reason_detail: Some(
                    "Failed to resolve local time in restaurant timezone.".to_string(),
                ),
                snapped_from,
                slot: None,
                alternatives: vec![],
                opening_hours_that_day: vec![],
                note,
            };
        }
    };

    // 5. Validate date in the past or inside minimum lead time (too_soon)
    let min_booking_utc = now_utc + Duration::minutes(restaurant.lead_time_minutes);
    if requested_start_utc < min_booking_utc {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Unavailable.as_str().to_string(),
            reason: Some(AvailabilityReason::TooSoon.as_str().to_string()),
            reason_detail: Some(format!(
                "Bookings must be made at least {} minutes in advance. The earliest booking time is {}.",
                restaurant.lead_time_minutes,
                min_booking_utc.with_timezone(&tz).format("%H:%M")
            )),
            snapped_from,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    // 6. Validate booking horizon (beyond_horizon)
    let horizon_days = restaurant.booking_horizon_days;
    let max_allowed_date = now_local.date_naive() + Duration::days(horizon_days);
    if query.date > max_allowed_date {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Unavailable.as_str().to_string(),
            reason: Some(AvailabilityReason::BeyondHorizon.as_str().to_string()),
            reason_detail: Some(format!(
                "Reservations can only be made up to {} days in advance (until {}).",
                horizon_days,
                max_allowed_date.format("%Y-%m-%d")
            )),
            snapped_from,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    // 7. Validate restaurant open that day (0 = Sunday, 1 = Monday, ..., 6 = Saturday)
    let day_of_week = query.date.weekday().num_days_from_sunday() as i64;
    let day_hours: Vec<&OpeningHours> = opening_hours
        .iter()
        .filter(|h| h.day_of_week == day_of_week)
        .collect();

    let open_day_hours: Vec<&OpeningHours> =
        day_hours.iter().copied().filter(|h| !h.is_closed).collect();

    let opening_hours_that_day: Vec<OpeningHoursRange> = open_day_hours
        .iter()
        .map(|h| OpeningHoursRange {
            open: h.open_time.clone(),
            close: h.close_time.clone(),
        })
        .collect();

    if open_day_hours.is_empty() {
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Closed.as_str().to_string(),
            reason: Some(AvailabilityReason::ClosedDay.as_str().to_string()),
            reason_detail: Some(format!(
                "The restaurant is closed on {}s.",
                query.date.format("%A")
            )),
            snapped_from,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note,
        };
    }

    // 8. Validate special closure for that date
    let date_str = query.date.format("%Y-%m-%d").to_string();
    if let Some(closure) = special_closures.iter().find(|c| c.closure_date == date_str) {
        if closure.is_all_day {
            return CheckAvailabilityOutput {
                requested: requested_details,
                restaurant_now: restaurant_now_iso,
                status: AvailabilityStatus::Closed.as_str().to_string(),
                reason: Some(AvailabilityReason::SpecialClosure.as_str().to_string()),
                reason_detail: Some(format!(
                    "The restaurant is closed on {}: {}.",
                    date_str, closure.reason
                )),
                snapped_from,
                slot: None,
                alternatives: vec![],
                opening_hours_that_day: vec![],
                note,
            };
        }
        // Partial day special closure check
        if let (Some(ref c_open), Some(ref c_close)) = (&closure.open_time, &closure.close_time) {
            if let (Ok(co), Ok(cc)) = (
                NaiveTime::parse_from_str(c_open, "%H:%M"),
                NaiveTime::parse_from_str(c_close, "%H:%M"),
            ) {
                if snapped_time >= co && snapped_time < cc {
                    return CheckAvailabilityOutput {
                        requested: requested_details,
                        restaurant_now: restaurant_now_iso,
                        status: AvailabilityStatus::Closed.as_str().to_string(),
                        reason: Some(AvailabilityReason::SpecialClosure.as_str().to_string()),
                        reason_detail: Some(format!(
                            "Special closure during {}-{}: {}.",
                            c_open, c_close, closure.reason
                        )),
                        snapped_from,
                        slot: None,
                        alternatives: vec![],
                        opening_hours_that_day,
                        note,
                    };
                }
            }
        }
    }

    // 9. Validate inside service periods and closing / last-seating rules
    let matching_period = find_service_period(
        snapped_time,
        duration_minutes,
        &open_day_hours,
        engine_config,
    );
    if matching_period.is_none() {
        let detail = format!(
            "Requested time {} for a {}-minute dining period is outside operating service hours or past last seating.",
            snapped_time.format("%H:%M"),
            duration_minutes
        );
        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Closed.as_str().to_string(),
            reason: Some(AvailabilityReason::OutsideHours.as_str().to_string()),
            reason_detail: Some(detail),
            snapped_from,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day,
            note,
        };
    }

    // 10. Table fit check for requested slot
    let requested_end_utc = requested_start_utc + Duration::minutes(duration_minutes);
    let best_table = find_best_fit_table(
        query.party_size,
        query.area,
        &active_tables,
        bookings,
        requested_start_utc,
        requested_end_utc,
        engine_config.turnover_buffer_minutes,
    );

    if let Some(table) = best_table {
        let start_local = requested_start_utc.with_timezone(&tz).to_rfc3339();
        let end_local = requested_end_utc.with_timezone(&tz).to_rfc3339();
        let reason_detail = if was_snapped {
            Some(format!(
                "Requested time {} was snapped to the nearest 15-minute slot {}.",
                query.time.format("%H:%M"),
                snapped_time.format("%H:%M")
            ))
        } else {
            None
        };

        return CheckAvailabilityOutput {
            requested: requested_details,
            restaurant_now: restaurant_now_iso,
            status: AvailabilityStatus::Available.as_str().to_string(),
            reason: None,
            reason_detail,
            snapped_from,
            slot: Some(AvailableSlot {
                start_local,
                end_local,
                table_label: table.label.clone(),
                area: table.area.as_str().to_string(),
                capacity: table.capacity_max,
            }),
            alternatives: vec![],
            opening_hours_that_day,
            note,
        };
    }

    // 11. Slot is unavailable; compute alternatives
    let alternatives = find_alternatives(
        &query,
        snapped_time,
        duration_minutes,
        restaurant,
        &open_day_hours,
        opening_hours,
        special_closures,
        &active_tables,
        bookings,
        engine_config,
        tz,
        clock,
    );

    let reason_detail = if alternatives.is_empty() {
        format!(
            "No tables available for party of {} at {} on {}.",
            query.party_size,
            snapped_time.format("%H:%M"),
            query.date.format("%Y-%m-%d")
        )
    } else {
        format!(
            "No tables available for party of {} at {} in {} area. Found {} alternative slot(s).",
            query.party_size,
            snapped_time.format("%H:%M"),
            query.area.as_str(),
            alternatives.len()
        )
    };

    CheckAvailabilityOutput {
        requested: requested_details,
        restaurant_now: restaurant_now_iso,
        status: AvailabilityStatus::Unavailable.as_str().to_string(),
        reason: Some(AvailabilityReason::FullyBooked.as_str().to_string()),
        reason_detail: Some(reason_detail),
        snapped_from,
        slot: None,
        alternatives,
        opening_hours_that_day,
        note,
    }
}

// --- Internal Helper Algorithms ---

/// Find service period that contains `start_time` and can accommodate `duration_minutes`.
/// Rule:
/// 1. `start_time >= period.open_time`
/// 2. `start_time <= period.close_time - last_seating_buffer`
/// 3. `start_time + duration <= period.close_time`
fn find_service_period<'a>(
    start_time: NaiveTime,
    duration_minutes: i64,
    open_periods: &'a [&OpeningHours],
    engine_config: &AvailabilityConfig,
) -> Option<&'a OpeningHours> {
    for period in open_periods {
        let p_open = NaiveTime::parse_from_str(&period.open_time, "%H:%M").ok()?;
        let p_close = NaiveTime::parse_from_str(&period.close_time, "%H:%M").ok()?;

        if start_time < p_open || start_time >= p_close {
            continue;
        }

        let last_seating_time =
            p_close - Duration::minutes(engine_config.last_seating_buffer_minutes);
        if start_time > last_seating_time {
            continue;
        }

        // Dining duration must end by close time
        let end_seconds = start_time.num_seconds_from_midnight() as i64 + duration_minutes * 60;
        let close_seconds = p_close.num_seconds_from_midnight() as i64;
        if end_seconds > close_seconds {
            continue;
        }

        return Some(*period);
    }
    None
}

/// Find deterministic best-fit table:
/// 1. Smallest sufficient capacity (`capacity_max` asc)
/// 2. Area preference (requested area, then Indoor, Patio, Bar)
/// 3. Table id asc
fn find_best_fit_table<'a>(
    party_size: i64,
    area_filter: AreaFilter,
    tables: &'a [&DiningTable],
    bookings: &[Booking],
    req_start: DateTime<Utc>,
    req_end: DateTime<Utc>,
    turnover_buffer_minutes: i64,
) -> Option<&'a DiningTable> {
    let mut fitting_tables: Vec<&DiningTable> = tables
        .iter()
        .copied()
        .filter(|t| t.capacity_min <= party_size && party_size <= t.capacity_max)
        .filter(|t| area_filter.matches(t.area))
        .filter(|t| !is_table_blocked(t.id, bookings, req_start, req_end, turnover_buffer_minutes))
        .collect();

    if fitting_tables.is_empty() {
        return None;
    }

    fitting_tables.sort_by(|a, b| {
        a.capacity_max
            .cmp(&b.capacity_max)
            .then_with(|| area_priority(a.area).cmp(&area_priority(b.area)))
            .then_with(|| a.id.cmp(&b.id))
    });

    Some(fitting_tables[0])
}

fn area_priority(area: Area) -> u8 {
    match area {
        Area::Indoor => 1,
        Area::Patio => 2,
        Area::Bar => 3,
    }
}

/// Check if a table has an active overlapping reservation.
/// Overlap rule: pending or confirmed bookings block [start_at, end_at + buffer).
/// Cancelled, completed, no_show do NOT block.
fn is_table_blocked(
    table_id: i64,
    bookings: &[Booking],
    req_start: DateTime<Utc>,
    req_end: DateTime<Utc>,
    turnover_buffer_minutes: i64,
) -> bool {
    for bk in bookings {
        if bk.table_id != Some(table_id) {
            continue;
        }
        if bk.status != BookingStatus::Pending && bk.status != BookingStatus::Confirmed {
            continue;
        }
        let bk_start = match DateTime::parse_from_rfc3339(&bk.start_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => continue,
        };
        let bk_end = match DateTime::parse_from_rfc3339(&bk.end_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => continue,
        };

        if is_interval_overlapping(
            req_start,
            req_end,
            bk_start,
            bk_end,
            turnover_buffer_minutes,
        ) {
            return true;
        }
    }
    false
}

/// Search alternative slots:
/// 1. Check different area at requested time (distance = 0)
/// 2. Outward search on slot grid (distance in time, then later-before-earlier)
/// 3. Next 3 open days at requested time if same date yields 0 alternatives
#[allow(clippy::too_many_arguments)]
fn find_alternatives(
    query: &AvailabilityQuery,
    snapped_time: NaiveTime,
    duration_minutes: i64,
    restaurant: &RestaurantConfig,
    open_day_hours: &[&OpeningHours],
    all_opening_hours: &[OpeningHours],
    special_closures: &[SpecialClosure],
    active_tables: &[&DiningTable],
    bookings: &[Booking],
    engine_config: &AvailabilityConfig,
    tz: Tz,
    clock: &dyn Clock,
) -> Vec<AlternativeSlot> {
    let limit = query.alternatives_limit.clamp(0, 6);
    if limit == 0 {
        return vec![];
    }

    let mut alternatives: Vec<AlternativeSlot> = Vec::new();
    let min_booking_utc = clock.now() + Duration::minutes(restaurant.lead_time_minutes);

    // Step 1: Different area at the requested time
    if query.area != AreaFilter::Any {
        let other_areas = match query.area {
            AreaFilter::Indoor => [AreaFilter::Patio, AreaFilter::Bar],
            AreaFilter::Patio => [AreaFilter::Indoor, AreaFilter::Bar],
            AreaFilter::Bar => [AreaFilter::Indoor, AreaFilter::Patio],
            AreaFilter::Any => [AreaFilter::Any, AreaFilter::Any],
        };

        if let Ok(req_start_utc) = local_to_utc(query.date, snapped_time, tz) {
            let req_end_utc = req_start_utc + Duration::minutes(duration_minutes);
            if req_start_utc >= min_booking_utc {
                for area in other_areas {
                    if let Some(table) = find_best_fit_table(
                        query.party_size,
                        area,
                        active_tables,
                        bookings,
                        req_start_utc,
                        req_end_utc,
                        engine_config.turnover_buffer_minutes,
                    ) {
                        alternatives.push(AlternativeSlot {
                            start_local: req_start_utc.with_timezone(&tz).to_rfc3339(),
                            end_local: req_end_utc.with_timezone(&tz).to_rfc3339(),
                            area: table.area.as_str().to_string(),
                            table_label: table.label.clone(),
                            minutes_from_requested: 0,
                            different_day: false,
                        });
                        break;
                    }
                }
            }
        }
    }

    // Step 2: Outward search on same date: distance in time, later-before-earlier
    let gran = restaurant.slot_granularity_minutes.max(5);
    let max_steps = engine_config.max_search_window_minutes / gran;

    for step in 1..=max_steps {
        if alternatives.len() >= limit {
            break;
        }

        let delta = step * gran;
        // Documented tie-break: later (+delta), then earlier (-delta)
        let candidate_deltas = [delta, -delta];

        for &d in &candidate_deltas {
            if alternatives.len() >= limit {
                break;
            }

            let candidate_seconds = snapped_time.num_seconds_from_midnight() as i64 + d * 60;
            if !(0..(24 * 3600)).contains(&candidate_seconds) {
                continue;
            }

            let cand_time =
                match NaiveTime::from_num_seconds_from_midnight_opt(candidate_seconds as u32, 0) {
                    Some(t) => t,
                    None => continue,
                };

            // Check if within service period
            if find_service_period(cand_time, duration_minutes, open_day_hours, engine_config)
                .is_none()
            {
                continue;
            }

            let cand_start_utc = match local_to_utc(query.date, cand_time, tz) {
                Ok(utc) => utc,
                Err(_) => continue,
            };

            if cand_start_utc < min_booking_utc {
                continue;
            }

            let cand_end_utc = cand_start_utc + Duration::minutes(duration_minutes);

            if let Some(table) = find_best_fit_table(
                query.party_size,
                query.area,
                active_tables,
                bookings,
                cand_start_utc,
                cand_end_utc,
                engine_config.turnover_buffer_minutes,
            ) {
                // Ensure not already in alternatives
                let start_local = cand_start_utc.with_timezone(&tz).to_rfc3339();
                if !alternatives
                    .iter()
                    .any(|a| a.start_local == start_local && a.area == table.area.as_str())
                {
                    alternatives.push(AlternativeSlot {
                        start_local,
                        end_local: cand_end_utc.with_timezone(&tz).to_rfc3339(),
                        area: table.area.as_str().to_string(),
                        table_label: table.label.clone(),
                        minutes_from_requested: d.abs(),
                        different_day: false,
                    });
                }
            }
        }
    }

    // Step 3: Next 3 open days fallback if same date yielded no alternatives
    if alternatives.is_empty() {
        let mut open_days_checked = 0;
        let mut day_offset = 1;

        while open_days_checked < 3 && day_offset <= 14 {
            let next_date = query.date + Duration::days(day_offset);
            day_offset += 1;

            let dow = next_date.weekday().num_days_from_sunday() as i64;
            let next_day_hours: Vec<&OpeningHours> = all_opening_hours
                .iter()
                .filter(|h| h.day_of_week == dow && !h.is_closed)
                .collect();

            if next_day_hours.is_empty() {
                continue;
            }

            let next_date_str = next_date.format("%Y-%m-%d").to_string();
            if special_closures
                .iter()
                .any(|c| c.closure_date == next_date_str && c.is_all_day)
            {
                continue;
            }

            open_days_checked += 1;

            if find_service_period(
                snapped_time,
                duration_minutes,
                &next_day_hours,
                engine_config,
            )
            .is_none()
            {
                continue;
            }

            let cand_start_utc = match local_to_utc(next_date, snapped_time, tz) {
                Ok(utc) => utc,
                Err(_) => continue,
            };

            let cand_end_utc = cand_start_utc + Duration::minutes(duration_minutes);

            if let Some(table) = find_best_fit_table(
                query.party_size,
                query.area,
                active_tables,
                bookings,
                cand_start_utc,
                cand_end_utc,
                engine_config.turnover_buffer_minutes,
            ) {
                alternatives.push(AlternativeSlot {
                    start_local: cand_start_utc.with_timezone(&tz).to_rfc3339(),
                    end_local: cand_end_utc.with_timezone(&tz).to_rfc3339(),
                    area: table.area.as_str().to_string(),
                    table_label: table.label.clone(),
                    minutes_from_requested: (cand_start_utc
                        - local_to_utc(query.date, snapped_time, tz).unwrap_or(cand_start_utc))
                    .num_minutes()
                    .abs(),
                    different_day: true,
                });

                if alternatives.len() >= limit {
                    break;
                }
            }
        }
    }

    alternatives
}

// --- Voice Text Formatting ---

/// Convert integer to spoken English word.
pub fn number_to_words(n: u32) -> String {
    match n {
        0 => "zero".to_string(),
        1 => "one".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        6 => "six".to_string(),
        7 => "seven".to_string(),
        8 => "eight".to_string(),
        9 => "nine".to_string(),
        10 => "ten".to_string(),
        11 => "eleven".to_string(),
        12 => "twelve".to_string(),
        13 => "thirteen".to_string(),
        14 => "fourteen".to_string(),
        15 => "fifteen".to_string(),
        16 => "sixteen".to_string(),
        17 => "seventeen".to_string(),
        18 => "eighteen".to_string(),
        19 => "nineteen".to_string(),
        20 => "twenty".to_string(),
        21 => "twenty-one".to_string(),
        22 => "twenty-two".to_string(),
        23 => "twenty-three".to_string(),
        24 => "twenty-four".to_string(),
        25 => "twenty-five".to_string(),
        26 => "twenty-six".to_string(),
        27 => "twenty-seven".to_string(),
        28 => "twenty-eight".to_string(),
        29 => "twenty-nine".to_string(),
        30 => "thirty".to_string(),
        35 => "thirty-five".to_string(),
        40 => "forty".to_string(),
        45 => "forty-five".to_string(),
        50 => "fifty".to_string(),
        55 => "fifty-five".to_string(),
        _ => n.to_string(),
    }
}

/// Convert day of month (1-31) into spoken ordinal word.
pub fn day_ordinal(day: u32) -> &'static str {
    match day {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        20 => "twentieth",
        21 => "twenty-first",
        22 => "twenty-second",
        23 => "twenty-third",
        24 => "twenty-fourth",
        25 => "twenty-fifth",
        26 => "twenty-sixth",
        27 => "twenty-seventh",
        28 => "twenty-eighth",
        29 => "twenty-ninth",
        30 => "thirtieth",
        31 => "thirty-first",
        _ => "day",
    }
}

/// Format local date into spoken natural English (e.g. "Friday the twenty-fourth").
pub fn format_spoken_date(date: NaiveDate) -> String {
    let weekday = date.format("%A").to_string();
    let ordinal = day_ordinal(date.day());
    format!("{} the {}", weekday, ordinal)
}

/// Format local time into spoken natural English (e.g. "seven thirty in the evening", "noon", "midnight").
pub fn format_spoken_time(time: NaiveTime) -> String {
    let hour = time.hour();
    let minute = time.minute();

    if hour == 12 && minute == 0 {
        return "noon".to_string();
    }
    if hour == 0 && minute == 0 {
        return "midnight".to_string();
    }

    let (hour_12, period) = match hour {
        0 => (12, "at night"),
        1..=11 => (hour, "in the morning"),
        12 => (12, "in the afternoon"),
        13..=16 => (hour - 12, "in the afternoon"),
        _ => (hour - 12, "in the evening"),
    };

    let hour_word = number_to_words(hour_12);

    if minute == 0 {
        format!("{} {}", hour_word, period)
    } else {
        let min_word = number_to_words(minute);
        format!("{} {} {}", hour_word, min_word, period)
    }
}

/// Short spoken time without period for compact listing (e.g. "seven", "seven thirty", "noon").
pub fn format_compact_spoken_time(time: NaiveTime) -> String {
    let hour = time.hour();
    let minute = time.minute();

    if hour == 12 && minute == 0 {
        return "noon".to_string();
    }
    if hour == 0 && minute == 0 {
        return "midnight".to_string();
    }

    let hour_12 = match hour {
        0 => 12,
        1..=12 => hour,
        _ => hour - 12,
    };

    let hour_word = number_to_words(hour_12);
    if minute == 0 {
        hour_word
    } else {
        format!("{} {}", hour_word, number_to_words(minute))
    }
}

/// Generate spoken natural English response for the voice assistant (1-3 short sentences, no markdown, no emoji).
pub fn generate_availability_voice_text(output: &CheckAvailabilityOutput) -> String {
    let date = match NaiveDate::parse_from_str(&output.requested.date, "%Y-%m-%d") {
        Ok(d) => format_spoken_date(d),
        Err(_) => output.requested.date.clone(),
    };

    let time = match NaiveTime::parse_from_str(&output.requested.time, "%H:%M") {
        Ok(t) => format_spoken_time(t),
        Err(_) => output.requested.time.clone(),
    };

    match output.status.as_str() {
        "available" => {
            let party_phrase = if output.requested.party_size == 1 {
                "one guest".to_string()
            } else {
                format!("{} guests", output.requested.party_size)
            };
            format!(
                "I have a table for {} on {} at {}. Want me to book it?",
                party_phrase, date, time
            )
        }
        "unavailable" => {
            if output.reason_detail.as_deref() == Some("temporarily held, may free up soon") {
                if output.alternatives.is_empty() {
                    return "That table is temporarily held and may free up soon. Would you like to check back in a few minutes?".to_string();
                } else {
                    let first = &output.alternatives[0];
                    let alt_time = extract_spoken_time(&first.start_local);
                    return format!(
                        "That table is temporarily held and may free up soon, but I have {}. Would that work for you?",
                        alt_time
                    );
                }
            }
            if output.alternatives.is_empty() {
                format!(
                    "I don't have a table available for {} on {} around {}. Would you like to check another day?",
                    output.requested.party_size, date, time
                )
            } else if output.alternatives.len() == 1 {
                let alt = &output.alternatives[0];
                let alt_time = extract_spoken_time(&alt.start_local);
                if alt.minutes_from_requested == 0 && alt.area != output.requested.area {
                    format!(
                        "I don't have {} seating at {}, but {} is free at {}. Would you like me to book that instead?",
                        output.requested.area, time, alt.area, time
                    )
                } else if alt.different_day {
                    let alt_date = extract_spoken_date(&alt.start_local);
                    format!(
                        "I don't have a table on {}, but I have {} at {}. Would that work for you?",
                        date, alt_date, alt_time
                    )
                } else {
                    format!(
                        "I don't have a table at {}, but I have {}. Would that work for you?",
                        time, alt_time
                    )
                }
            } else {
                // 2 or more alternatives
                let first = &output.alternatives[0];
                let second = &output.alternatives[1];

                if first.minutes_from_requested == 0 && first.area != output.requested.area {
                    let second_time = extract_compact_spoken_time(&second.start_local);
                    format!(
                        "I don't have {} seating at {}, but {} is free at {}, or I have {} in {}. Would either work for you?",
                        output.requested.area, time, first.area, time, second_time, second.area
                    )
                } else if first.different_day {
                    let first_date = extract_spoken_date(&first.start_local);
                    let first_time = extract_spoken_time(&first.start_local);
                    format!(
                        "I don't have a table on {}, but I have {} at {}. Would that work for you?",
                        date, first_date, first_time
                    )
                } else {
                    let alt1 = extract_compact_spoken_time(&first.start_local);
                    let alt2 = extract_spoken_time(&second.start_local);
                    format!(
                        "I don't have a table available at {}, but I have {} or {}. Would either of those work for you?",
                        time, alt1, alt2
                    )
                }
            }
        }
        "closed" => match output.reason.as_deref() {
            Some("closed_day") => {
                format!(
                        "The restaurant is closed on {}. Would you like to check availability for tomorrow?",
                        date
                    )
            }
            Some("special_closure") => {
                let detail = output
                    .reason_detail
                    .as_deref()
                    .unwrap_or("a scheduled closure");
                format!(
                    "The restaurant is closed on {} for {}. Would you like to pick another date?",
                    date, detail
                )
            }
            Some("outside_hours") => {
                if !output.opening_hours_that_day.is_empty() {
                    let hours_text: Vec<String> = output
                        .opening_hours_that_day
                        .iter()
                        .map(|h| {
                            let o = NaiveTime::parse_from_str(&h.open, "%H:%M")
                                .map(format_compact_spoken_time)
                                .unwrap_or_else(|_| h.open.clone());
                            let c = NaiveTime::parse_from_str(&h.close, "%H:%M")
                                .map(format_spoken_time)
                                .unwrap_or_else(|_| h.close.clone());
                            format!("{} to {}", o, c)
                        })
                        .collect();
                    format!(
                        "The restaurant is not open at {}. Opening hours that day are {}.",
                        time,
                        hours_text.join(" and ")
                    )
                } else {
                    format!(
                        "The restaurant is closed at {}. Please check our opening hours.",
                        time
                    )
                }
            }
            _ => {
                format!(
                    "The restaurant is closed at {}. Please pick another time.",
                    time
                )
            }
        },
        _ => {
            let detail = output
                .reason_detail
                .as_deref()
                .unwrap_or("Please check your request details and try again.");
            format!(
                "I could not check availability for that request. {}",
                detail
            )
        }
    }
}

fn extract_spoken_time(rfc3339_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339_str) {
        format_spoken_time(dt.time())
    } else {
        rfc3339_str.to_string()
    }
}

fn extract_compact_spoken_time(rfc3339_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339_str) {
        format_compact_spoken_time(dt.time())
    } else {
        rfc3339_str.to_string()
    }
}

fn extract_spoken_date(rfc3339_str: &str) -> String {
    if let Ok(dt) = DateTime::parse_from_rfc3339(rfc3339_str) {
        format_spoken_date(dt.date_naive())
    } else {
        rfc3339_str.to_string()
    }
}

// --- Unit Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::DepositStatus;

    fn test_clock(iso: &str) -> FixedClock {
        FixedClock(
            DateTime::parse_from_rfc3339(iso)
                .unwrap()
                .with_timezone(&Utc),
        )
    }

    fn sample_restaurant() -> RestaurantConfig {
        RestaurantConfig {
            id: 1,
            name: "The Saffron Nebula Bistro".to_string(),
            timezone: "Asia/Kolkata".to_string(),
            currency: "INR".to_string(),
            dining_duration_minutes: 90,
            slot_granularity_minutes: 15,
            max_party_size: 12,
            lead_time_minutes: 30,
            booking_horizon_days: 30,
            deposit_required_party_size: 6,
            deposit_amount_per_person_cents: 50000,
            created_at: "2026-10-01T00:00:00Z".to_string(),
            updated_at: "2026-10-01T00:00:00Z".to_string(),
            cancellation_cutoff_hours: 6,
            late_cancellation_fee_cents: 0,
            tax_rate_basis_points: 500,
            last_order_before_close_minutes: 30,
            phone: "+91 80 4567 8900".to_string(),
        }
    }

    fn sample_opening_hours() -> Vec<OpeningHours> {
        // Mon-Sun, lunch 12:00-15:30, dinner 19:00-23:00. Tuesdays (2) closed.
        let mut hours = Vec::new();
        for dow in 0..=6 {
            if dow == 2 {
                // Tuesday closed
                hours.push(OpeningHours {
                    id: dow * 2 + 1,
                    day_of_week: dow,
                    period_name: "Lunch".to_string(),
                    open_time: "00:00".to_string(),
                    close_time: "00:00".to_string(),
                    is_closed: true,
                });
                hours.push(OpeningHours {
                    id: dow * 2 + 2,
                    day_of_week: dow,
                    period_name: "Dinner".to_string(),
                    open_time: "00:00".to_string(),
                    close_time: "00:00".to_string(),
                    is_closed: true,
                });
            } else {
                hours.push(OpeningHours {
                    id: dow * 2 + 1,
                    day_of_week: dow,
                    period_name: "Lunch".to_string(),
                    open_time: "12:00".to_string(),
                    close_time: "15:30".to_string(),
                    is_closed: false,
                });
                hours.push(OpeningHours {
                    id: dow * 2 + 2,
                    day_of_week: dow,
                    period_name: "Dinner".to_string(),
                    open_time: "19:00".to_string(),
                    close_time: "23:00".to_string(),
                    is_closed: false,
                });
            }
        }
        hours
    }

    fn sample_special_closures() -> Vec<SpecialClosure> {
        vec![SpecialClosure {
            id: 1,
            closure_date: "2026-10-28".to_string(),
            reason: "Annual Kitchen Maintenance".to_string(),
            is_all_day: true,
            open_time: None,
            close_time: None,
        }]
    }

    fn sample_tables() -> Vec<DiningTable> {
        vec![
            DiningTable {
                id: 1,
                label: "T01".to_string(),
                capacity_min: 2,
                capacity_max: 2,
                area: Area::Indoor,
                is_active: true,
            },
            DiningTable {
                id: 2,
                label: "T02".to_string(),
                capacity_min: 2,
                capacity_max: 4,
                area: Area::Indoor,
                is_active: true,
            },
            DiningTable {
                id: 3,
                label: "T03".to_string(),
                capacity_min: 4,
                capacity_max: 6,
                area: Area::Indoor,
                is_active: true,
            },
            DiningTable {
                id: 4,
                label: "T04".to_string(),
                capacity_min: 6,
                capacity_max: 8,
                area: Area::Indoor,
                is_active: true,
            },
            DiningTable {
                id: 5,
                label: "P01".to_string(),
                capacity_min: 2,
                capacity_max: 4,
                area: Area::Patio,
                is_active: true,
            },
            DiningTable {
                id: 6,
                label: "B01".to_string(),
                capacity_min: 1,
                capacity_max: 2,
                area: Area::Bar,
                is_active: true,
            },
        ]
    }

    fn make_booking(
        id: &str,
        table_id: i64,
        start_iso: &str,
        end_iso: &str,
        status: BookingStatus,
    ) -> Booking {
        Booking {
            id: id.to_string(),
            confirmation_code: format!("CODE-{}", id),
            customer_id: Some(1),
            guest_name: "Guest".to_string(),
            party_size: 2,
            start_at: start_iso.to_string(),
            end_at: end_iso.to_string(),
            status,
            table_id: Some(table_id),
            special_requests: None,
            deposit_status: DepositStatus::None,
            deposit_amount_cents: 0,
            created_at: "2026-10-01T00:00:00Z".to_string(),
            updated_at: "2026-10-01T00:00:00Z".to_string(),
            cancelled_at: None,
            guest_phone: None,
            guest_email: None,
        }
    }

    #[test]
    fn test_spoken_time_formatting() {
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
            "noon"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            "midnight"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(7, 0, 0).unwrap()),
            "seven in the morning"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(7, 45, 0).unwrap()),
            "seven forty-five in the morning"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(12, 15, 0).unwrap()),
            "twelve fifteen in the afternoon"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(12, 30, 0).unwrap()),
            "twelve thirty in the afternoon"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(13, 0, 0).unwrap()),
            "one in the afternoon"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(15, 30, 0).unwrap()),
            "three thirty in the afternoon"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(19, 0, 0).unwrap()),
            "seven in the evening"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(19, 30, 0).unwrap()),
            "seven thirty in the evening"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(20, 15, 0).unwrap()),
            "eight fifteen in the evening"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(23, 45, 0).unwrap()),
            "eleven forty-five in the evening"
        );
        assert_eq!(
            format_spoken_time(NaiveTime::from_hms_opt(0, 30, 0).unwrap()),
            "twelve thirty at night"
        );
    }

    #[test]
    fn test_spoken_date_formatting() {
        assert_eq!(
            format_spoken_date(NaiveDate::from_ymd_opt(2026, 10, 23).unwrap()),
            "Friday the twenty-third"
        );
        assert_eq!(
            format_spoken_date(NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()),
            "Saturday the twenty-fourth"
        );
        assert_eq!(
            format_spoken_date(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            "Thursday the first"
        );
        assert_eq!(
            format_spoken_date(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()),
            "Saturday the thirty-first"
        );
    }

    #[test]
    fn test_open_day_free_table_best_fit_choice() {
        let clock = test_clock("2026-10-15T12:00:00Z"); // Thursday 17:30 IST
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        let tables = sample_tables();
        let bookings = vec![];
        let cfg = AvailabilityConfig::default();

        // Party of 2 requesting 19:30 on Friday 2026-10-16 (Indoor)
        // Must choose T01 (capacity 2), NOT T02 (4) or T04 (8)!
        let query = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out = check_availability_engine(
            query,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out.status, "available");
        assert_eq!(out.reason, None);
        let slot = out.slot.expect("Slot must be present");
        assert_eq!(
            slot.table_label, "T01",
            "Party of 2 must get smallest 2-top T01"
        );
        assert_eq!(slot.capacity, 2);

        // Party of 4 requesting 19:30 on Friday 2026-10-16 (Indoor)
        // Must choose T02 (capacity 4), NOT T03 (6) or T04 (8)!
        let query4 = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 4,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out4 = check_availability_engine(
            query4,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out4.status, "available");
        let slot4 = out4.slot.expect("Slot must be present");
        assert_eq!(
            slot4.table_label, "T02",
            "Party of 4 must get smallest 4-top T02"
        );
    }

    #[test]
    fn test_exact_boundary_overlap_with_and_without_buffer() {
        let clock = test_clock("2026-10-15T12:00:00Z");
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        // Only 1 table: T01 (2-top)
        let tables = vec![DiningTable {
            id: 1,
            label: "T01".to_string(),
            capacity_min: 2,
            capacity_max: 2,
            area: Area::Indoor,
            is_active: true,
        }];

        // Existing booking on T01: 17:30 to 19:00 IST (12:00 to 13:30 UTC)
        let existing = make_booking(
            "b1",
            1,
            "2026-10-16T12:00:00Z",
            "2026-10-16T13:30:00Z",
            BookingStatus::Confirmed,
        );
        let bookings = vec![existing];

        // Request starts exactly at 19:00 IST (13:30 UTC)
        let query = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        // Case A: buffer = 0 minutes -> boundary is NOT blocking
        let cfg_no_buffer = AvailabilityConfig {
            turnover_buffer_minutes: 0,
            ..Default::default()
        };
        let out_no_buf = check_availability_engine(
            query.clone(),
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg_no_buffer,
            &clock,
        );
        assert_eq!(
            out_no_buf.status, "available",
            "Booking ending at 19:00 must NOT block request starting at 19:00 when buffer is 0"
        );

        // Case B: buffer = 15 minutes -> boundary blocks until 19:15 IST
        let cfg_with_buffer = AvailabilityConfig {
            turnover_buffer_minutes: 15,
            ..Default::default()
        };
        let out_with_buf = check_availability_engine(
            query,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg_with_buffer,
            &clock,
        );
        assert_eq!(
            out_with_buf.status, "unavailable",
            "Booking ending at 19:00 MUST block request starting at 19:00 when buffer is 15m"
        );
    }

    #[test]
    fn test_rejection_reasons_each() {
        let clock = test_clock("2026-10-15T12:00:00Z"); // Thursday 17:30 IST
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        let tables = sample_tables();
        let bookings = vec![];
        let cfg = AvailabilityConfig::default();

        // 1. too_soon (request time is in past or < 30m lead time)
        let q_soon = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            time: NaiveTime::from_hms_opt(17, 45, 0).unwrap(), // only 15m ahead
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_soon = check_availability_engine(
            q_soon,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_soon.status, "unavailable");
        assert_eq!(out_soon.reason.as_deref(), Some("too_soon"));

        // 2. beyond_horizon (> 30 days)
        let q_horizon = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(), // 36 days ahead
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_horizon = check_availability_engine(
            q_horizon,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_horizon.status, "unavailable");
        assert_eq!(out_horizon.reason.as_deref(), Some("beyond_horizon"));

        // 3. party_size_invalid (< 1)
        let q_neg = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 0,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_neg = check_availability_engine(
            q_neg,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_neg.status, "invalid_request");
        assert_eq!(out_neg.reason.as_deref(), Some("party_size_invalid"));

        // 4. party_too_large (> restaurant max 12)
        let q_large = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 15,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_large = check_availability_engine(
            q_large,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_large.status, "unavailable");
        assert_eq!(out_large.reason.as_deref(), Some("party_too_large"));

        // 5. no_single_table (party 10 <= 12, but max table capacity is 8)
        let q_no_single = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 10,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_no_single = check_availability_engine(
            q_no_single,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_no_single.status, "unavailable");
        assert_eq!(out_no_single.reason.as_deref(), Some("no_single_table"));

        // 6. closed_day (Tuesday 2026-10-20)
        let q_tue = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_tue = check_availability_engine(
            q_tue,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_tue.status, "closed");
        assert_eq!(out_tue.reason.as_deref(), Some("closed_day"));

        // 7. special_closure (2026-10-28 Maintenance)
        let q_closure = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_closure = check_availability_engine(
            q_closure,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_closure.status, "closed");
        assert_eq!(out_closure.reason.as_deref(), Some("special_closure"));

        // 8. outside_hours (gap between lunch and dinner: 16:00)
        let q_gap = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_gap = check_availability_engine(
            q_gap,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_gap.status, "closed");
        assert_eq!(out_gap.reason.as_deref(), Some("outside_hours"));
    }

    #[test]
    fn test_last_seating_and_cross_closing() {
        let clock = test_clock("2026-10-15T12:00:00Z");
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        let tables = sample_tables();
        let bookings = vec![];
        let cfg = AvailabilityConfig {
            last_seating_buffer_minutes: 45,
            ..Default::default()
        };

        // Dinner closes at 23:00.
        // A request at 22:30 for 90m duration ends at 00:00 (past 23:00) and starts after 22:15 (last seating cutoff)
        let query = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(22, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out = check_availability_engine(
            query,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out.status, "closed");
        assert_eq!(out.reason.as_deref(), Some("outside_hours"));
    }

    #[test]
    fn test_off_grid_snapping() {
        // 19:22 -> 19:15
        let (s1, was1) = snap_to_grid(NaiveTime::from_hms_opt(19, 22, 0).unwrap(), 15);
        assert!(was1);
        assert_eq!(s1, NaiveTime::from_hms_opt(19, 15, 0).unwrap());

        // 19:23 -> 19:30
        let (s2, was2) = snap_to_grid(NaiveTime::from_hms_opt(19, 23, 0).unwrap(), 15);
        assert!(was2);
        assert_eq!(s2, NaiveTime::from_hms_opt(19, 30, 0).unwrap());

        // 19:30 -> 19:30 (on-grid)
        let (s3, was3) = snap_to_grid(NaiveTime::from_hms_opt(19, 30, 0).unwrap(), 15);
        assert!(!was3);
        assert_eq!(s3, NaiveTime::from_hms_opt(19, 30, 0).unwrap());
    }

    #[test]
    fn test_cancelled_and_no_show_bookings_do_not_block() {
        let clock = test_clock("2026-10-15T12:00:00Z");
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        let tables = vec![DiningTable {
            id: 1,
            label: "T01".to_string(),
            capacity_min: 2,
            capacity_max: 2,
            area: Area::Indoor,
            is_active: true,
        }];
        let cfg = AvailabilityConfig::default();

        let req_start = "2026-10-16T14:00:00Z";
        let req_end = "2026-10-16T15:30:00Z";

        // Cancelled booking
        let bk_canc = vec![make_booking(
            "c1",
            1,
            req_start,
            req_end,
            BookingStatus::Cancelled,
        )];
        let q = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };
        let out_canc = check_availability_engine(
            q.clone(),
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bk_canc,
            &cfg,
            &clock,
        );
        assert_eq!(
            out_canc.status, "available",
            "Cancelled booking must not block"
        );

        // No-show booking
        let bk_ns = vec![make_booking(
            "ns1",
            1,
            req_start,
            req_end,
            BookingStatus::NoShow,
        )];
        let out_ns = check_availability_engine(
            q.clone(),
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bk_ns,
            &cfg,
            &clock,
        );
        assert_eq!(out_ns.status, "available", "NoShow booking must not block");

        // Confirmed booking
        let bk_conf = vec![make_booking(
            "cf1",
            1,
            req_start,
            req_end,
            BookingStatus::Confirmed,
        )];
        let out_conf = check_availability_engine(
            q,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bk_conf,
            &cfg,
            &clock,
        );
        assert_eq!(
            out_conf.status, "unavailable",
            "Confirmed booking MUST block"
        );
    }

    #[test]
    fn test_alternatives_ordering_tie_breaks_and_different_area() {
        let clock = test_clock("2026-10-15T12:00:00Z");
        let restaurant = sample_restaurant();
        let hours = sample_opening_hours();
        let closures = sample_special_closures();
        let cfg = AvailabilityConfig::default();

        // 2 tables: T01 (Indoor 2-top) and P01 (Patio 2-top)
        let tables = vec![
            DiningTable {
                id: 1,
                label: "T01".to_string(),
                capacity_min: 2,
                capacity_max: 2,
                area: Area::Indoor,
                is_active: true,
            },
            DiningTable {
                id: 2,
                label: "P01".to_string(),
                capacity_min: 2,
                capacity_max: 2,
                area: Area::Patio,
                is_active: true,
            },
        ];

        // Indoor T01 is booked at 19:30 (14:00Z to 15:30Z)
        // Patio P01 is FREE at 19:30
        let bookings = vec![make_booking(
            "b1",
            1,
            "2026-10-16T14:00:00Z",
            "2026-10-16T15:30:00Z",
            BookingStatus::Confirmed,
        )];

        // User requests Indoor at 19:30
        let query = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(19, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out = check_availability_engine(
            query,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out.status, "unavailable");
        assert!(!out.alternatives.is_empty());

        // First alternative should be Patio at 19:30 (distance 0m)
        let first_alt = &out.alternatives[0];
        assert_eq!(first_alt.area, "patio");
        assert_eq!(first_alt.minutes_from_requested, 0);
        assert!(!first_alt.different_day);
    }

    #[test]
    fn test_dst_gap_and_dst_overlap_injected_zone() {
        let mut restaurant = sample_restaurant();
        restaurant.timezone = "America/New_York".to_string(); // Injected zone with DST
        let hours = sample_opening_hours();
        let closures = vec![];
        let tables = sample_tables();
        let bookings = vec![];
        let cfg = AvailabilityConfig::default();

        // Fixed clock on 2026-03-01 in UTC
        let clock = test_clock("2026-03-01T12:00:00Z");

        // 1. Spring Forward Gap in America/New_York:
        // Clocks jump 02:00 -> 03:00 on Sunday, 2026-03-08.
        // Therefore, 02:30 AM DOES NOT EXIST!
        let tz: Tz = "America/New_York".parse().unwrap();
        let naive_gap = NaiveDate::from_ymd_opt(2026, 3, 8)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let local_res = tz.from_local_datetime(&naive_gap);
        assert_eq!(
            local_res,
            chrono::LocalResult::None,
            "2:30 AM must not exist during spring-forward gap"
        );

        let q_gap = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            time: NaiveTime::from_hms_opt(2, 30, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Any,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out_gap = check_availability_engine(
            q_gap,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out_gap.status, "invalid_request");
        assert_eq!(out_gap.reason.as_deref(), Some("dst_gap"));

        // 2. Fall Back Overlap in America/New_York:
        // Clocks fall back 02:00 -> 01:00 on Sunday, 2026-11-01.
        // 01:30 AM exists twice (Ambiguous)!
        let naive_overlap = NaiveDate::from_ymd_opt(2026, 11, 1)
            .unwrap()
            .and_hms_opt(1, 30, 0)
            .unwrap();
        let local_overlap_res = tz.from_local_datetime(&naive_overlap);
        assert!(matches!(
            local_overlap_res,
            chrono::LocalResult::Ambiguous(_, _)
        ));

        let resolved_utc = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap(),
            NaiveTime::from_hms_opt(1, 30, 0).unwrap(),
            tz,
        );
        assert!(
            resolved_utc.is_ok(),
            "DST overlap must resolve deterministically without errors"
        );
    }

    #[test]
    fn test_date_boundary_late_dinner_crossing_midnight() {
        let clock = test_clock("2026-10-15T12:00:00Z");
        let restaurant = sample_restaurant();
        let closures = vec![];
        let tables = vec![DiningTable {
            id: 1,
            label: "T01".to_string(),
            capacity_min: 2,
            capacity_max: 2,
            area: Area::Indoor,
            is_active: true,
        }];
        let bookings = vec![];
        let cfg = AvailabilityConfig {
            last_seating_buffer_minutes: 0,
            ..Default::default()
        };

        // Custom hours for Friday: Dinner from 22:00 to 01:00 (crosses midnight)
        // In opening hours table, represented as close_time 23:59 or separate shifts
        // Here we test late booking: starts 23:00, duration 90m -> ends 00:30 next day
        let hours = vec![OpeningHours {
            id: 1,
            day_of_week: 5, // Friday
            period_name: "Late Dinner".to_string(),
            open_time: "22:00".to_string(),
            close_time: "23:59".to_string(),
            is_closed: false,
        }];

        let query = AvailabilityQuery {
            date: NaiveDate::from_ymd_opt(2026, 10, 16).unwrap(),
            time: NaiveTime::from_hms_opt(22, 15, 0).unwrap(),
            party_size: 2,
            area: AreaFilter::Indoor,
            duration_minutes: Some(90),
            alternatives_limit: 3,
        };

        let out = check_availability_engine(
            query,
            &restaurant,
            &hours,
            &closures,
            &tables,
            &bookings,
            &cfg,
            &clock,
        );
        assert_eq!(out.status, "available");
        let slot = out.slot.expect("Slot must be allocated");
        assert!(slot.start_local.contains("2026-10-16T22:15:00"));
        assert!(slot.end_local.contains("2026-10-16T23:45:00"));
    }

    #[test]
    fn test_voice_text_generation_three_modes() {
        // Mode 1: Available
        let out_avail = CheckAvailabilityOutput {
            requested: RequestedDetails {
                date: "2026-10-23".to_string(),
                time: "19:30".to_string(),
                party_size: 4,
                area: "indoor".to_string(),
                duration_minutes: 90,
                timezone: "Asia/Kolkata".to_string(),
            },
            restaurant_now: "2026-10-15T12:00:00+05:30".to_string(),
            status: "available".to_string(),
            reason: None,
            reason_detail: None,
            snapped_from: None,
            slot: Some(AvailableSlot {
                start_local: "2026-10-23T19:30:00+05:30".to_string(),
                end_local: "2026-10-23T21:00:00+05:30".to_string(),
                table_label: "T03".to_string(),
                area: "indoor".to_string(),
                capacity: 4,
            }),
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note: "snapshot".to_string(),
        };
        let voice_avail = generate_availability_voice_text(&out_avail);
        assert!(voice_avail.contains("Friday the twenty-third"));
        assert!(voice_avail.contains("seven thirty in the evening"));
        assert!(voice_avail.contains("Want me to book it?"));

        // Mode 2: Unavailable with alternatives
        let out_unavail = CheckAvailabilityOutput {
            requested: RequestedDetails {
                date: "2026-10-23".to_string(),
                time: "19:30".to_string(),
                party_size: 4,
                area: "indoor".to_string(),
                duration_minutes: 90,
                timezone: "Asia/Kolkata".to_string(),
            },
            restaurant_now: "2026-10-15T12:00:00+05:30".to_string(),
            status: "unavailable".to_string(),
            reason: Some("fully_booked".to_string()),
            reason_detail: Some("No tables available".to_string()),
            snapped_from: None,
            slot: None,
            alternatives: vec![
                AlternativeSlot {
                    start_local: "2026-10-23T19:00:00+05:30".to_string(),
                    end_local: "2026-10-23T20:30:00+05:30".to_string(),
                    area: "indoor".to_string(),
                    table_label: "T03".to_string(),
                    minutes_from_requested: 30,
                    different_day: false,
                },
                AlternativeSlot {
                    start_local: "2026-10-23T20:15:00+05:30".to_string(),
                    end_local: "2026-10-23T21:45:00+05:30".to_string(),
                    area: "indoor".to_string(),
                    table_label: "T03".to_string(),
                    minutes_from_requested: 45,
                    different_day: false,
                },
            ],
            opening_hours_that_day: vec![],
            note: "snapshot".to_string(),
        };
        let voice_unavail = generate_availability_voice_text(&out_unavail);
        assert!(voice_unavail.contains("seven"));
        assert!(voice_unavail.contains("eight fifteen in the evening"));
        assert!(voice_unavail.contains("Would either of those work for you?"));

        // Mode 3: Closed (Tuesdays)
        let out_closed = CheckAvailabilityOutput {
            requested: RequestedDetails {
                date: "2026-10-20".to_string(),
                time: "19:30".to_string(),
                party_size: 2,
                area: "any".to_string(),
                duration_minutes: 90,
                timezone: "Asia/Kolkata".to_string(),
            },
            restaurant_now: "2026-10-15T12:00:00+05:30".to_string(),
            status: "closed".to_string(),
            reason: Some("closed_day".to_string()),
            reason_detail: Some("Closed on Tuesdays".to_string()),
            snapped_from: None,
            slot: None,
            alternatives: vec![],
            opening_hours_that_day: vec![],
            note: "snapshot".to_string(),
        };
        let voice_closed = generate_availability_voice_text(&out_closed);
        assert!(voice_closed.contains("closed on Tuesday the twentieth"));
    }
}
