/**
 * Exhaustive mapping of server reason codes to clear human phrases.
 * Tested in unit tests to ensure no server enum value lacks a human translation.
 */

export const REASON_CODE_MAP: Record<string, string> = {
  // Availability & Negotiation scoring reasons
  exact_time: "Exact match for your requested time",
  same_day: "Available today",
  exact_capacity_fit: "Perfect table size for your party",
  good_capacity_fit: "Comfortable seating for your party",
  generous_capacity_fit: "Spacious table for your group",
  comfortable_dining_window: "Full unhurried dining duration",
  requested_exact_match: "Exact match for your request",
  closest_time_match: "Nearest available time",
  adjacent_time_earlier: "Earlier dining time",
  adjacent_time_later: "Later dining time",
  alternative_area: "Different seating area",
  alternative_day: "Alternative date",
  larger_table_allocated: "Generous seating",
  split_party_recommendation: "Large party accommodation",
  table_combining_needed: "Requires table combining",
  within_service_window: "During regular meal service",

  // Booking & Availability negative / constraint reasons
  fully_booked: "Fully booked at this time",
  closed_day: "Restaurant is closed on this day",
  special_closure: "Closed for special event or holiday",
  outside_hours: "Outside regular service hours",
  too_soon: "Requires more advance notice (minimum 30 minutes)",
  beyond_horizon: "Outside 30-day reservation window",
  party_too_large: "Exceeds online party size (maximum 12 guests)",
  party_size_invalid: "Party size is invalid",
  no_single_table: "No single table large enough for party",
  active_bookings_limit: "Maximum active reservations reached (limit 3)",
  deposit_required: "Table deposit required",
  hold_expired: "Table hold has expired",
  idempotency_conflict: "Reservation key conflict",
  limit_exceeded: "Active reservation limit reached",
  not_cancellable: "Past cancellation deadline",
  already_cancelled: "Already cancelled",
  already_booked: "Table has already been booked",
  dst_gap: "Time falls into daylight saving adjustment gap",
  in_the_past: "Requested time is in the past",

  // Ordering & Menu reasons
  item_unavailable: "Item currently out of stock",
  allergen_conflict: "Contains allergen requested to avoid",
  unknown_item: "Dish not found on current menu",
  ambiguous_item: "Multiple matching dishes found",
  pickup_too_soon: "Kitchen requires more prep time",
  duplicate: "Preference or item is already registered",
  confirmation_required: "Explicit confirmation required for safety",
  no_history: "No prior order history found",
  not_found: "Record not found",
  options_found: "Alternative dining times available",
  no_viable_options: "No viable alternative tables available",
};

/**
 * Translates a reason code into a human-friendly string.
 * Falls back gracefully to a formatted version of the code if unrecognized.
 */
export function translateReasonCode(code: string | null | undefined): string {
  if (!code) return "";
  const mapped = REASON_CODE_MAP[code];
  if (mapped) return mapped;

  // Graceful fallback: convert snake_case to Title Case words
  return code
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}
