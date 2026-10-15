import { useState } from "react"
import {
  Sun,
  Moon,
  Smartphone,
  Tablet,
  Monitor,
  Volume2,
  RotateCcw,
  CheckCircle,
} from "lucide-react"
import { renderCard } from "../cards/registry"
import type { CardEventHandlers } from "../cards/events"
import { useTheme } from "@/lib/theme"
import { StatusBadge } from "@/components/primitives/StatusBadge"

// Real fixtures generated directly from server execution against seeded DB
import searchMenuFound from "../cards/fixtures/search_menu_found.json"
import searchMenuEmpty from "../cards/fixtures/search_menu_empty.json"
import checkAvailabilityAvailable from "../cards/fixtures/check_availability_available.json"
import checkAvailabilityFullyBooked from "../cards/fixtures/check_availability_fully_booked.json"
import checkAvailabilityClosed from "../cards/fixtures/check_availability_closed.json"
import createBookingConfirmed from "../cards/fixtures/create_booking_confirmed.json"
import createBookingDepositRequired from "../cards/fixtures/create_booking_deposit_required.json"
import getBookingFound from "../cards/fixtures/get_booking_found.json"
import getBookingNotFound from "../cards/fixtures/get_booking_not_found.json"
import cancelBookingCancelled from "../cards/fixtures/cancel_booking_cancelled.json"
import negotiateBookingOptions from "../cards/fixtures/negotiate_booking_options.json"
import negotiateBookingNoOptions from "../cards/fixtures/negotiate_booking_no_options.json"
import placeOrderPlaced from "../cards/fixtures/place_order_placed.json"
import placeOrderAllergenConflict from "../cards/fixtures/place_order_allergen_conflict.json"
import getOrderFound from "../cards/fixtures/get_order_found.json"
import cancelOrderCancelled from "../cards/fixtures/cancel_order_cancelled.json"
import getDepositStatusOpen from "../cards/fixtures/get_deposit_status_open.json"
import getDepositStatusPaid from "../cards/fixtures/get_deposit_status_paid.json"
import refreshPaymentSessionRefreshed from "../cards/fixtures/refresh_payment_session_refreshed.json"
import acceptOfferConfirmed from "../cards/fixtures/accept_offer_confirmed.json"
import acceptOfferHold from "../cards/fixtures/accept_offer_hold.json"
import releaseOfferReleased from "../cards/fixtures/release_offer_released.json"
import joinWaitlistJoined from "../cards/fixtures/join_waitlist_joined.json"
import getWaitlistStatusWaiting from "../cards/fixtures/get_waitlist_status_waiting.json"
import leaveWaitlistLeft from "../cards/fixtures/leave_waitlist_left.json"
import getProfileFound from "../cards/fixtures/get_profile_found.json"
import enableMemoryEnabled from "../cards/fixtures/enable_memory_enabled.json"
import rememberPreferenceRemembered from "../cards/fixtures/remember_preference_remembered.json"
import forgetPreferenceForgotten from "../cards/fixtures/forget_preference_forgotten.json"
import forgetMeForgotten from "../cards/fixtures/forget_me_forgotten.json"
import getUsualOrderFound from "../cards/fixtures/get_usual_order_found.json"
import pingRestaurant from "../cards/fixtures/ping_restaurant.json"
import invalidPayload from "../cards/fixtures/invalid_payload.json"

export interface GalleryItem {
  id: string
  title: string
  toolName: string
  category: string
  statusLabel: string
  envelope: any
}

export const GALLERY_FIXTURES: GalleryItem[] = [
  // 1. Menu Search
  {
    id: "menu-found",
    title: "Menu Search (Items Found)",
    toolName: "search_menu",
    category: "Menu",
    statusLabel: "found",
    envelope: searchMenuFound,
  },
  {
    id: "menu-empty",
    title: "Menu Search (No Matches)",
    toolName: "search_menu",
    category: "Menu",
    statusLabel: "empty",
    envelope: searchMenuEmpty,
  },

  // 2. Availability
  {
    id: "avail-available",
    title: "Check Availability (Available Slot)",
    toolName: "check_availability",
    category: "Availability",
    statusLabel: "available",
    envelope: checkAvailabilityAvailable,
  },
  {
    id: "avail-fully-booked",
    title: "Check Availability (Fully Booked + Alternatives)",
    toolName: "check_availability",
    category: "Availability",
    statusLabel: "fully_booked",
    envelope: checkAvailabilityFullyBooked,
  },
  {
    id: "avail-closed",
    title: "Check Availability (Closed Day)",
    toolName: "check_availability",
    category: "Availability",
    statusLabel: "closed",
    envelope: checkAvailabilityClosed,
  },

  // 3. Negotiation
  {
    id: "neg-options",
    title: "Negotiate Booking (Alternative Options Offered)",
    toolName: "negotiate_booking",
    category: "Negotiation",
    statusLabel: "options_found",
    envelope: negotiateBookingOptions,
  },
  {
    id: "neg-no-options",
    title: "Negotiate Booking (No Viable Options)",
    toolName: "negotiate_booking",
    category: "Negotiation",
    statusLabel: "no_viable_options",
    envelope: negotiateBookingNoOptions,
  },
  {
    id: "neg-accept-confirmed",
    title: "Accept Offer (Confirmed Reservation)",
    toolName: "accept_offer",
    category: "Negotiation",
    statusLabel: "confirmed",
    envelope: acceptOfferConfirmed,
  },
  {
    id: "neg-accept-hold",
    title: "Accept Offer (Table Held)",
    toolName: "accept_offer",
    category: "Negotiation",
    statusLabel: "held",
    envelope: acceptOfferHold,
  },
  {
    id: "neg-release",
    title: "Release Offer (Hold Released)",
    toolName: "release_offer",
    category: "Negotiation",
    statusLabel: "released",
    envelope: releaseOfferReleased,
  },

  // 4. Bookings
  {
    id: "book-confirmed",
    title: "Create Booking (Confirmed)",
    toolName: "create_booking",
    category: "Bookings",
    statusLabel: "confirmed",
    envelope: createBookingConfirmed,
  },
  {
    id: "book-dep-required",
    title: "Create Booking (Deposit Required)",
    toolName: "create_booking",
    category: "Bookings",
    statusLabel: "pending / deposit_required",
    envelope: createBookingDepositRequired,
  },
  {
    id: "book-get-found",
    title: "Get Booking (Reservation Found)",
    toolName: "get_booking",
    category: "Bookings",
    statusLabel: "found",
    envelope: getBookingFound,
  },
  {
    id: "book-get-not-found",
    title: "Get Booking (Not Found)",
    toolName: "get_booking",
    category: "Bookings",
    statusLabel: "not_found",
    envelope: getBookingNotFound,
  },
  {
    id: "book-cancelled",
    title: "Cancel Booking (Cancelled)",
    toolName: "cancel_booking",
    category: "Bookings",
    statusLabel: "cancelled",
    envelope: cancelBookingCancelled,
  },

  // 5. Payments & Deposits
  {
    id: "pay-deposit-open",
    title: "Deposit Payment (Pending / Open)",
    toolName: "get_deposit_status",
    category: "Payments",
    statusLabel: "open",
    envelope: getDepositStatusOpen,
  },
  {
    id: "pay-deposit-paid",
    title: "Deposit Payment (Paid / Confirmed)",
    toolName: "get_deposit_status",
    category: "Payments",
    statusLabel: "paid",
    envelope: getDepositStatusPaid,
  },
  {
    id: "pay-refresh",
    title: "Refresh Payment Session",
    toolName: "refresh_payment_session",
    category: "Payments",
    statusLabel: "refreshed",
    envelope: refreshPaymentSessionRefreshed,
  },

  // 6. Orders
  {
    id: "order-placed",
    title: "Place Order (Placed Successfully)",
    toolName: "place_order",
    category: "Orders",
    statusLabel: "placed",
    envelope: placeOrderPlaced,
  },
  {
    id: "order-allergen-conflict",
    title: "Place Order (Allergen Conflict Blocked)",
    toolName: "place_order",
    category: "Orders",
    statusLabel: "allergen_conflict",
    envelope: placeOrderAllergenConflict,
  },
  {
    id: "order-get-found",
    title: "Get Order (Found)",
    toolName: "get_order",
    category: "Orders",
    statusLabel: "found",
    envelope: getOrderFound,
  },
  {
    id: "order-cancelled",
    title: "Cancel Order (Cancelled)",
    toolName: "cancel_order",
    category: "Orders",
    statusLabel: "cancelled",
    envelope: cancelOrderCancelled,
  },
  {
    id: "order-usual",
    title: "Get Usual Order (Re-validated Against Menu)",
    toolName: "get_usual_order",
    category: "Orders",
    statusLabel: "suggested",
    envelope: getUsualOrderFound,
  },

  // 7. Waitlist
  {
    id: "wl-joined",
    title: "Join Waitlist (Joined with Position)",
    toolName: "join_waitlist",
    category: "Waitlist",
    statusLabel: "joined",
    envelope: joinWaitlistJoined,
  },
  {
    id: "wl-status-waiting",
    title: "Get Waitlist Status (Waiting in Queue)",
    toolName: "get_waitlist_status",
    category: "Waitlist",
    statusLabel: "waiting",
    envelope: getWaitlistStatusWaiting,
  },
  {
    id: "wl-leave",
    title: "Leave Waitlist (Left)",
    toolName: "leave_waitlist",
    category: "Waitlist",
    statusLabel: "left",
    envelope: leaveWaitlistLeft,
  },

  // 8. Profile & Privacy
  {
    id: "prof-get-found",
    title: "Get Profile (Saved Preferences & Inert Note)",
    toolName: "get_profile",
    category: "Profile & Privacy",
    statusLabel: "found",
    envelope: getProfileFound,
  },
  {
    id: "prof-enable",
    title: "Enable Memory (Explicit Opt-in)",
    toolName: "enable_memory",
    category: "Profile & Privacy",
    statusLabel: "enabled",
    envelope: enableMemoryEnabled,
  },
  {
    id: "prof-remember",
    title: "Remember Preference",
    toolName: "remember_preference",
    category: "Profile & Privacy",
    statusLabel: "remembered",
    envelope: rememberPreferenceRemembered,
  },
  {
    id: "prof-forget-pref",
    title: "Forget Preference",
    toolName: "forget_preference",
    category: "Profile & Privacy",
    statusLabel: "forgotten",
    envelope: forgetPreferenceForgotten,
  },
  {
    id: "prof-forget-me",
    title: "Forget Me (Hard Delete & PII Scrub)",
    toolName: "forget_me",
    category: "Profile & Privacy",
    statusLabel: "forgotten",
    envelope: forgetMeForgotten,
  },

  // 9. System & Fallback
  {
    id: "sys-ping",
    title: "Ping Restaurant (Server Status)",
    toolName: "ping_restaurant",
    category: "System & Fallback",
    statusLabel: "ok",
    envelope: pingRestaurant,
  },
  {
    id: "sys-invalid-fallback",
    title: "Invalid Payload (Safe Spoken Fallback)",
    toolName: "check_availability",
    category: "System & Fallback",
    statusLabel: "corrupted_payload",
    envelope: invalidPayload,
  },
]

type ViewportWidth = "360px" | "768px" | "100%"

export default function CardGallery() {
  const { resolvedTheme, setTheme } = useTheme()
  const isDark = resolvedTheme === "dark"
  const [viewportWidth, setViewportWidth] = useState<ViewportWidth>("100%")
  const [textOnly, setTextOnly] = useState(false)
  const [reducedMotion, setReducedMotion] = useState(false)
  const [selectedCategory, setSelectedCategory] = useState<string>("All")
  const [lastEvent, setLastEvent] = useState<string | null>(null)

  const toggleTheme = () => {
    setTheme(isDark ? "light" : "dark")
  }

  const toggleReducedMotion = () => {
    setReducedMotion((prev) => {
      const next = !prev
      if (next) document.documentElement.classList.add("motion-reduce")
      else document.documentElement.classList.remove("motion-reduce")
      return next
    })
  }

  const categories = ["All", ...Array.from(new Set(GALLERY_FIXTURES.map((f) => f.category)))]

  const filteredFixtures =
    selectedCategory === "All"
      ? GALLERY_FIXTURES
      : GALLERY_FIXTURES.filter((f) => f.category === selectedCategory)

  // Event handlers that log to dev UI
  const eventHandlers: CardEventHandlers = {
    onAcceptOption: (e) => setLastEvent(`onAcceptOption: ${JSON.stringify(e)}`),
    onHoldOption: (e) => setLastEvent(`onHoldOption: ${JSON.stringify(e)}`),
    onSelectAlternative: (e) => setLastEvent(`onSelectAlternative: ${JSON.stringify(e)}`),
    onCancelBooking: (e) => setLastEvent(`onCancelBooking: ${JSON.stringify(e)}`),
    onCancelOrder: (e) => setLastEvent(`onCancelOrder: ${JSON.stringify(e)}`),
    onOpenCheckout: (e) => setLastEvent(`onOpenCheckout: ${JSON.stringify(e)}`),
    onCheckStatus: (e) => setLastEvent(`onCheckStatus: ${JSON.stringify(e)}`),
    onLeaveWaitlist: (e) => setLastEvent(`onLeaveWaitlist: ${JSON.stringify(e)}`),
    onCopyCode: (e) => setLastEvent(`onCopyCode: ${JSON.stringify(e)}`),
  }

  return (
    <div
      className={`min-h-screen bg-background text-foreground transition-colors duration-200 ${
        reducedMotion ? "motion-reduce" : ""
      }`}
    >
      {/* Top Dev Toolbar */}
      <header className="sticky top-0 z-30 flex flex-wrap items-center justify-between gap-3 border-b border-border bg-card/80 px-6 py-3 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-primary-foreground font-bold text-xs">
            TT
          </div>
          <div>
            <h1 className="text-sm font-bold tracking-tight">TableTalk Card Gallery</h1>
            <p className="text-[11px] text-muted-foreground">
              Phase 13 Component Harness • 33 Real Output Fixtures • 24 Registered Tools
            </p>
          </div>
        </div>

        {/* Toolbar Controls */}
        <div className="flex flex-wrap items-center gap-2 text-xs">
          {/* Viewport Width Controls */}
          <div className="flex items-center rounded-xl border border-border/80 bg-background p-1">
            <button
              type="button"
              onClick={() => setViewportWidth("360px")}
              aria-label="Mobile viewport 360px"
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 min-h-[40px] cursor-pointer transition-colors ${
                viewportWidth === "360px" ? "bg-muted font-semibold text-foreground shadow-2xs" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Smartphone className="size-3.5" aria-hidden="true" />
              <span>360px</span>
            </button>
            <button
              type="button"
              onClick={() => setViewportWidth("768px")}
              aria-label="Tablet viewport 768px"
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 min-h-[40px] cursor-pointer transition-colors ${
                viewportWidth === "768px" ? "bg-muted font-semibold text-foreground shadow-2xs" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Tablet className="size-3.5" aria-hidden="true" />
              <span>768px</span>
            </button>
            <button
              type="button"
              onClick={() => setViewportWidth("100%")}
              aria-label="Desktop viewport full width"
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 min-h-[40px] cursor-pointer transition-colors ${
                viewportWidth === "100%" ? "bg-muted font-semibold text-foreground shadow-2xs" : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Monitor className="size-3.5" aria-hidden="true" />
              <span>Full</span>
            </button>
          </div>

          {/* Text-only fallback toggle */}
          <button
            type="button"
            onClick={() => setTextOnly((prev) => !prev)}
            aria-pressed={textOnly}
            className={`flex items-center gap-1.5 rounded-xl border px-3 py-2 min-h-[44px] cursor-pointer transition-colors ${
              textOnly
                ? "bg-primary text-primary-foreground border-primary font-semibold"
                : "border-border/80 bg-background text-muted-foreground hover:bg-muted/60 hover:text-foreground"
            }`}
          >
            <Volume2 className="size-4" aria-hidden="true" />
            <span>Voice-Only Mode</span>
          </button>

          {/* Reduced-motion toggle */}
          <button
            type="button"
            onClick={toggleReducedMotion}
            aria-pressed={reducedMotion}
            className={`flex items-center gap-1.5 rounded-xl border px-3 py-2 min-h-[44px] cursor-pointer transition-colors ${
              reducedMotion
                ? "bg-accent text-accent-foreground border-accent font-semibold"
                : "border-border/80 bg-background text-muted-foreground hover:bg-muted/60 hover:text-foreground"
            }`}
          >
            <RotateCcw className="size-4" aria-hidden="true" />
            <span>Reduced Motion</span>
          </button>

          {/* Theme toggle */}
          <button
            type="button"
            onClick={toggleTheme}
            aria-label="Toggle light/dark theme"
            className="flex items-center gap-1.5 rounded-xl border border-border/80 bg-background px-3 py-2 min-h-[44px] cursor-pointer text-muted-foreground hover:bg-muted/60 hover:text-foreground transition-colors"
          >
            {isDark ? <Sun className="size-4 text-amber-500" /> : <Moon className="size-4" />}
            <span>{isDark ? "Light" : "Dark"}</span>
          </button>
        </div>
      </header>

      {/* Category Filter Pills & Event Log */}
      <div className="border-b border-border/80 bg-card/40 px-6 py-2.5 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-semibold text-muted-foreground mr-1">Filter:</span>
          {categories.map((cat) => (
            <button
              key={cat}
              type="button"
              onClick={() => setSelectedCategory(cat)}
              className={`rounded-full px-3 py-1.5 min-h-[36px] cursor-pointer transition-colors text-xs ${
                selectedCategory === cat
                  ? "bg-foreground text-background font-semibold"
                  : "bg-muted/60 text-muted-foreground hover:bg-muted hover:text-foreground"
              }`}
            >
              {cat}
            </button>
          ))}
        </div>

        {/* Live event notification badge */}
        {lastEvent && (
          <div className="flex items-center gap-1.5 rounded-lg bg-emerald-500/10 border border-emerald-500/30 px-2.5 py-1 text-[11px] text-emerald-700 dark:text-emerald-300 font-mono">
            <CheckCircle className="size-3.5" aria-hidden="true" />
            <span>Last Event: {lastEvent}</span>
          </div>
        )}
      </div>

      {/* Main Gallery Area */}
      <main className="p-6 flex flex-col items-center">
        <div
          style={{ width: viewportWidth, maxWidth: "100%" }}
          className="space-y-8 transition-[width] duration-300"
        >
          {filteredFixtures.map((fixture) => (
            <section
              key={fixture.id}
              className="space-y-3 rounded-2xl border border-border/80 bg-card/60 p-5 shadow-xs"
            >
              {/* Fixture Meta Header */}
              <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border/60 pb-3 text-xs">
                <div>
                  <span className="font-semibold text-foreground text-sm">{fixture.title}</span>
                  <span className="ml-2 font-mono text-[11px] text-muted-foreground">
                    ({fixture.toolName})
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <StatusBadge status="neutral" label={`status: ${fixture.statusLabel}`} />
                </div>
              </div>

              {/* Card Rendering */}
              {textOnly ? (
                // Voice-only / text-only simulated view
                <div className="rounded-xl border border-border/80 bg-muted/30 p-4 space-y-1.5">
                  <div className="flex items-center gap-1.5 text-xs font-semibold text-foreground">
                    <Volume2 className="h-4 w-4 text-primary" aria-hidden="true" />
                    <span>Voice Output (Screenless Echo / Audio-only)</span>
                  </div>
                  <p className="text-xs leading-relaxed text-foreground/90 font-serif italic">
                    "{fixture.envelope?.content?.[0]?.text || "No spoken speech synthesis."}"
                  </p>
                </div>
              ) : (
                // Rich Visual Card View
                renderCard({
                  toolName: fixture.toolName,
                  envelope: fixture.envelope,
                  events: eventHandlers,
                })
              )}
            </section>
          ))}
        </div>
      </main>
    </div>
  )
}
