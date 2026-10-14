/**
 * AUTO-GENERATED FILE - DO NOT EDIT MANUALLY.
 * Generated from TableTalk MCP Server output schemas.
 * Run 'npm run gen:types' to regenerate.
 */
/* eslint-disable @typescript-eslint/no-explicit-any */

export type DietaryTagFilter = "vegan" | "vegetarian" | "gluten_free" | "dairy_free" | "halal" | "nut_free";
export type AllergenFilter =
  "peanuts" | "tree_nuts" | "dairy" | "gluten" | "soy" | "sesame" | "mustard" | "shellfish" | "egg" | "fish";
export type SortOption = "relevance" | "price_asc" | "price_desc" | "popular";

export interface TableTalkAllTools {
  accept_offer: {
    booking?: BookingSummary | null;
    hold?: HoldSummary | null;
    message: string;
    offer_token?: string | null;
    /**
     * Present only when a simulated deposit is required to confirm the booking.
     */
    payment?: PaymentObject | null;
    status: string;
  };
  cancel_booking: {
    already_cancelled: boolean;
    booking?: BookingSummary | null;
    policy_note?: string | null;
    reason?: string | null;
    reason_detail?: string | null;
    /**
     * Present only when a deposit was captured and a refund was evaluated.
     */
    refund?: RefundSummary | null;
    status: string;
  };
  cancel_order: {
    already_cancelled: boolean;
    order?: OrderSummary | null;
    policy_note?: string | null;
    reason?: string | null;
    reason_detail?: string | null;
    status: string;
  };
  check_availability: {
    alternatives: AlternativeSlot[];
    note: string;
    opening_hours_that_day: OpeningHoursRange[];
    reason?: string | null;
    reason_detail?: string | null;
    requested: RequestedDetails;
    restaurant_now: string;
    slot?: AvailableSlot | null;
    snapped_from?: string | null;
    status: string;
  };
  create_booking: {
    alternatives: AlternativeSlot[];
    booking?: BookingSummary | null;
    cancellation_policy?: string | null;
    idempotent_replay: boolean;
    negotiation_id?: number | null;
    negotiation_options?: NegotiationOption[] | null;
    note: string;
    /**
     * Present only when a simulated deposit is required for this booking.
     */
    payment?: PaymentObject | null;
    profile_applied?: boolean | null;
    reason?: string | null;
    reason_detail?: string | null;
    seat_preference_honored?: boolean | null;
    status: string;
  };
  enable_memory: {
    consent_version: string;
    guest_name: string;
    message: string;
    preferences_saved: number;
    profile_token?: string | null;
    status: string;
  };
  forget_me: {
    message: string;
    removed_components: string[];
    scrubbed_business_records: ScrubbedBusinessRecordsSummary;
    status: string;
  };
  forget_preference: {
    kind: string;
    message: string;
    policy_note?: string | null;
    removed: boolean;
    status: string;
    value: string;
  };
  get_booking: {
    booking?: BookingSummary | null;
    /**
     * Present only when the booking carries a deposit (paid or pending).
     */
    deposit?: DepositInfo | null;
    message: string;
    status: string;
  };
  get_deposit_status: {
    amount?: Money | null;
    deadline_local?: string | null;
    deposit_status?: string | null;
    message: string;
    minutes_remaining?: number | null;
    /**
     * What the caller would receive if they cancelled right now.
     */
    refund_if_cancelled_now?: DepositStatusMoney | null;
    required: boolean;
    status: string;
  };
  get_order: {
    message: string;
    order?: OrderSummary | null;
    status: string;
  };
  get_profile: {
    consent_granted_at: string;
    consent_version: string;
    counts: ProfileCounts;
    data_export?: DataExportRecord | null;
    email_masked?: string | null;
    guest_name: string;
    needs_reconfirmation: string[];
    phone_masked?: string | null;
    preferences: StoredPreferenceView[];
    status: string;
  };
  get_usual_order: {
    allergen_notice: string;
    conflicting_items: ConflictingAllergyItem[];
    guest_name?: string | null;
    needs_allergy_reconfirmation: string[];
    price_changes: PriceDifference[];
    status: string;
    subtotal?: PriceDisplay | null;
    suggested_lines: SuggestedOrderLine[];
    tax?: PriceDisplay | null;
    total?: PriceDisplay | null;
    unavailable_items: string[];
  };
  get_waitlist_status: {
    message: string;
    status: string;
    waitlist?: WaitlistSummary | null;
  };
  join_waitlist: {
    area: string;
    code: string;
    date: string;
    expires_at_local: string;
    guest_name: string;
    message: string;
    party_size: number;
    queue_position: number;
    status: string;
    window_end: string;
    window_start: string;
  };
  leave_waitlist: {
    already_cancelled: boolean;
    code: string;
    message: string;
    status: string;
  };
  negotiate_booking: {
    message: string;
    negotiation_id?: number | null;
    options: NegotiationOption[];
    profile_applied?: boolean | null;
    round?: number | null;
    slot?: AvailableSlot | null;
    status: string;
  };
  ping_restaurant: {
    /**
     * Friendly greeting message confirming assistant status
     */
    message: string;
    /**
     * Identifier of the serving MCP implementation
     */
    server: string;
    /**
     * Crate package version of the TableTalk server
     */
    version: string;
  };
  place_order: {
    allergen_notice: string;
    alternatives: string[];
    candidates: ItemCandidate[];
    earliest_pickup_local?: string | null;
    idempotent_replay: boolean;
    needs_reconfirmation?: string[];
    order?: OrderSummary | null;
    price_changes: PriceChange[];
    profile_applied?: boolean | null;
    reason?: string | null;
    reason_detail?: string | null;
    status: string;
  };
  refresh_payment_session: {
    message: string;
    payment?: PaymentObject | null;
    /**
     * One of: refreshed | not_required | not_pending | deadline_passed | not_found.
     */
    status: string;
  };
  release_offer: {
    already_released: boolean;
    message: string;
    status: string;
  };
  remember_preference: {
    deduplicated: boolean;
    message: string;
    preference?: StoredPreferenceView | null;
    status: string;
  };
  rotate_profile_token: {
    message: string;
    new_profile_token?: string | null;
    status: string;
  };
  search_menu: {
    /**
     * Standard allergen handling notice for kitchen operations
     */
    allergen_notice: string;
    applied_filters: AppliedFilters;
    /**
     * Menu items matching criteria
     */
    items: MenuItemCard[];
    needs_reconfirmation?: string[];
    profile_applied?: boolean | null;
    /**
     * Human-readable echo of interpreted filters
     */
    query_summary: string;
    /**
     * Number of items returned in this response
     */
    returned: number;
    /**
     * Concrete suggestions if zero or few matches found
     */
    suggestions: string[];
    /**
     * Total number of items matching filters before pagination
     */
    total_matches: number;
  };
}
export interface BookingSummary {
  area: string;
  confirmation_code: string;
  created_at: string;
  end_local: string;
  guest_name: string;
  party_size: number;
  source?: string | null;
  special_requests?: string | null;
  start_local: string;
  status: string;
  table_label: string;
}
export interface HoldSummary {
  area: string;
  end_local: string;
  expires_at_local: string;
  hold_id: number;
  party_size: number;
  seconds_remaining: number;
  start_local: string;
  table_label: string;
}
/**
 * Payment object embedded in create_booking / accept_offer / refresh responses.
 */
export interface PaymentObject {
  amount: Money;
  checkout_url: string;
  deadline_local: string;
  minutes_remaining: number;
  required: boolean;
  simulated: boolean;
  status: string;
}
/**
 * Structured money value: integer minor units plus a display string.
 */
export interface Money {
  amount_minor: number;
  currency: string;
  display: string;
}
/**
 * Refund outcome embedded in cancel_booking responses.
 */
export interface RefundSummary {
  amount: Money;
  explanation: string;
  percent_bps: number;
  percent_display: string;
}
export interface OrderSummary {
  created_at: string;
  guest_name: string;
  lines: OrderLineSummary[];
  order_code: string;
  payment_status: string;
  pickup_time_local: string;
  stated_allergens?: string | null;
  status: string;
  subtotal: PriceDisplay;
  tax: PriceDisplay;
  total: PriceDisplay;
}
export interface OrderLineSummary {
  allergens: string[];
  dietary_tags: string[];
  item_id: number;
  line_total: PriceDisplay;
  name: string;
  notes?: string | null;
  quantity: number;
  unit_price: PriceDisplay;
}
export interface PriceDisplay {
  /**
   * Price in minor currency units (paise)
   */
  amount_minor: number;
  /**
   * ISO currency code (e.g. INR)
   */
  currency: string;
  /**
   * Human-readable formatted price string (e.g. ₹380)
   */
  display: string;
}
/**
 * Alternative dining slot suggestion.
 */
export interface AlternativeSlot {
  area: string;
  different_day: boolean;
  end_local: string;
  minutes_from_requested: number;
  start_local: string;
  table_label: string;
}
/**
 * Operating hours range for a service period.
 */
export interface OpeningHoursRange {
  close: string;
  open: string;
}
/**
 * Details of the parsed reservation request.
 */
export interface RequestedDetails {
  area: string;
  date: string;
  duration_minutes: number;
  party_size: number;
  time: string;
  timezone: string;
}
/**
 * Allocated dining slot when available.
 */
export interface AvailableSlot {
  area: string;
  capacity: number;
  end_local: string;
  start_local: string;
  table_label: string;
}
export interface NegotiationOption {
  area: string;
  end_local: string;
  minutes_from_requested: number;
  option_id: string;
  rank: number;
  score_reasons: string[];
  start_local: string;
  table_label?: string | null;
}
export interface ScrubbedBusinessRecordsSummary {
  scrubbed_bookings: number;
  scrubbed_orders: number;
}
/**
 * Deposit details surfaced by get_booking.
 */
export interface DepositInfo {
  amount: Money;
  deadline_local?: string | null;
  minutes_remaining?: number | null;
  status: string;
}
export interface DepositStatusMoney {
  amount: Money;
  explanation: string;
  percent_bps: number;
  percent_display: string;
}
export interface ProfileCounts {
  preference_count: number;
  total_bookings: number;
  total_orders: number;
}
export interface DataExportRecord {
  booking_ids: string[];
  consent_granted_at: string;
  consent_version: string;
  display_name: string;
  email?: string | null;
  order_ids: string[];
  phone?: string | null;
  preferences: StoredPreferenceView[];
}
export interface StoredPreferenceView {
  confirmed_at?: string | null;
  created_at: string;
  id: number;
  kind: string;
  severity?: string | null;
  value: string;
}
export interface ConflictingAllergyItem {
  conflicting_allergens: string[];
  item_id: number;
  item_name: string;
}
export interface PriceDifference {
  current_price: PriceDisplay;
  item_id: number;
  item_name: string;
  previous_price: PriceDisplay;
}
export interface SuggestedOrderLine {
  item_id: number;
  item_name: string;
  line_total: PriceDisplay;
  quantity: number;
  unit_price: PriceDisplay;
  user_provided_note?: string | null;
}
export interface WaitlistSummary {
  area: string;
  code: string;
  date: string;
  expires_at_local: string;
  guest_name: string;
  offer?: WaitlistOfferDetail | null;
  party_size: number;
  queue_position?: number | null;
  status: string;
  window_end: string;
  window_start: string;
}
export interface WaitlistOfferDetail {
  area: string;
  end_local: string;
  expires_at_local: string;
  offer_token: string;
  seconds_remaining: number;
  start_local: string;
  table_label: string;
}
export interface ItemCandidate {
  item_id: number;
  name: string;
  price: PriceDisplay;
}
export interface PriceChange {
  current_price: PriceDisplay;
  item_id: number;
  item_name: string;
}
/**
 * Normalized filter values applied in database query
 */
export interface AppliedFilters {
  category?: string | null;
  dietary: DietaryTagFilter[];
  exclude_allergens: AllergenFilter[];
  include_unavailable: boolean;
  limit: number;
  max_price_inr?: number | null;
  query?: string | null;
  sort: SortOption;
}
export interface MenuItemCard {
  allergens: string[];
  available: boolean;
  calories?: number | null;
  category: string;
  chefs_special: boolean;
  description: string;
  dietary_tags: string[];
  id: number;
  image_url?: string | null;
  match_reason: string;
  name: string;
  prep_time_minutes: number;
  price: PriceDisplay;
  spice_level: number;
}


/* ==================== Derived Tool Output Types & Registry ==================== */

export type PingRestaurantOutput = TableTalkAllTools["ping_restaurant"];
export type SearchMenuOutput = TableTalkAllTools["search_menu"];
export type CheckAvailabilityOutput = TableTalkAllTools["check_availability"];
export type CreateBookingOutput = TableTalkAllTools["create_booking"];
export type CancelBookingOutput = TableTalkAllTools["cancel_booking"];
export type GetBookingOutput = TableTalkAllTools["get_booking"];
export type PlaceOrderOutput = TableTalkAllTools["place_order"];
export type GetOrderOutput = TableTalkAllTools["get_order"];
export type CancelOrderOutput = TableTalkAllTools["cancel_order"];
export type EnableMemoryOutput = TableTalkAllTools["enable_memory"];
export type GetProfileOutput = TableTalkAllTools["get_profile"];
export type RememberPreferenceOutput = TableTalkAllTools["remember_preference"];
export type ForgetPreferenceOutput = TableTalkAllTools["forget_preference"];
export type GetUsualOrderOutput = TableTalkAllTools["get_usual_order"];
export type RotateProfileTokenOutput = TableTalkAllTools["rotate_profile_token"];
export type ForgetMeOutput = TableTalkAllTools["forget_me"];
export type NegotiateBookingOutput = TableTalkAllTools["negotiate_booking"];
export type AcceptOfferOutput = TableTalkAllTools["accept_offer"];
export type ReleaseOfferOutput = TableTalkAllTools["release_offer"];
export type JoinWaitlistOutput = TableTalkAllTools["join_waitlist"];
export type GetWaitlistStatusOutput = TableTalkAllTools["get_waitlist_status"];
export type LeaveWaitlistOutput = TableTalkAllTools["leave_waitlist"];
export type GetDepositStatusOutput = TableTalkAllTools["get_deposit_status"];
export type RefreshPaymentSessionOutput = TableTalkAllTools["refresh_payment_session"];

export interface ToolStructuredPayloadMap {
  ping_restaurant: PingRestaurantOutput;
  search_menu: SearchMenuOutput;
  check_availability: CheckAvailabilityOutput;
  create_booking: CreateBookingOutput;
  cancel_booking: CancelBookingOutput;
  get_booking: GetBookingOutput;
  place_order: PlaceOrderOutput;
  get_order: GetOrderOutput;
  cancel_order: CancelOrderOutput;
  enable_memory: EnableMemoryOutput;
  get_profile: GetProfileOutput;
  remember_preference: RememberPreferenceOutput;
  forget_preference: ForgetPreferenceOutput;
  get_usual_order: GetUsualOrderOutput;
  rotate_profile_token: RotateProfileTokenOutput;
  forget_me: ForgetMeOutput;
  negotiate_booking: NegotiateBookingOutput;
  accept_offer: AcceptOfferOutput;
  release_offer: ReleaseOfferOutput;
  join_waitlist: JoinWaitlistOutput;
  get_waitlist_status: GetWaitlistStatusOutput;
  leave_waitlist: LeaveWaitlistOutput;
  get_deposit_status: GetDepositStatusOutput;
  refresh_payment_session: RefreshPaymentSessionOutput;
}

export type TableTalkToolName = keyof ToolStructuredPayloadMap;

export interface ToolResponseEnvelope<T = unknown> {
  content?: Array<{ type: string; text?: string; [key: string]: unknown }>;
  structuredContent?: T;
  isError?: boolean;
}
