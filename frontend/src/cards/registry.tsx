import React from "react"
import type { TableTalkToolName, ToolResponseEnvelope } from "./types"
import type { CardEventHandlers } from "./events"
import { validateCardPayload } from "./schemaValidation"

// Component imports
import { MenuCarousel } from "./components/MenuCarousel"
import { AvailabilityCard } from "./components/AvailabilityCard"
import { NegotiationOptionsCard } from "./components/NegotiationOptionsCard"
import { BookingConfirmationCard } from "./components/BookingConfirmationCard"
import { PaymentCard } from "./components/PaymentCard"
import { OrderSummaryCard } from "./components/OrderSummaryCard"
import { WaitlistCard } from "./components/WaitlistCard"
import { ProfileCard } from "./components/ProfileCard"
import { StatusCard } from "./components/StatusCard"
import { FallbackCard } from "./components/FallbackCard"

export interface CardRendererProps {
  toolName: TableTalkToolName | string
  envelope: ToolResponseEnvelope<any>
  events?: CardEventHandlers
}

/**
 * Explicit registry declaration of all 24 tools.
 * Each tool maps to its card renderer or has an explicit "no_card" entry.
 */
export const TOOL_CARD_DECLARATIONS: Record<
  string,
  { type: "card" | "no_card"; defaultCard?: string }
> = {
  ping_restaurant: { type: "card", defaultCard: "StatusCard" },
  search_menu: { type: "card", defaultCard: "MenuCarousel" },
  check_availability: { type: "card", defaultCard: "AvailabilityCard" },
  create_booking: { type: "card", defaultCard: "BookingConfirmationCard" },
  cancel_booking: { type: "card", defaultCard: "StatusCard" },
  get_booking: { type: "card", defaultCard: "BookingConfirmationCard" },
  place_order: { type: "card", defaultCard: "OrderSummaryCard" },
  get_order: { type: "card", defaultCard: "OrderSummaryCard" },
  cancel_order: { type: "card", defaultCard: "StatusCard" },
  enable_memory: { type: "card", defaultCard: "ProfileCard" },
  get_profile: { type: "card", defaultCard: "ProfileCard" },
  remember_preference: { type: "card", defaultCard: "StatusCard" },
  forget_preference: { type: "card", defaultCard: "StatusCard" },
  get_usual_order: { type: "card", defaultCard: "OrderSummaryCard" },
  rotate_profile_token: { type: "no_card" }, // System security tool: token rotation has no rich visual card
  forget_me: { type: "card", defaultCard: "StatusCard" },
  negotiate_booking: { type: "card", defaultCard: "NegotiationOptionsCard" },
  accept_offer: { type: "card", defaultCard: "BookingConfirmationCard" },
  release_offer: { type: "card", defaultCard: "StatusCard" },
  join_waitlist: { type: "card", defaultCard: "WaitlistCard" },
  get_waitlist_status: { type: "card", defaultCard: "WaitlistCard" },
  leave_waitlist: { type: "card", defaultCard: "StatusCard" },
  get_deposit_status: { type: "card", defaultCard: "PaymentCard" },
  refresh_payment_session: { type: "card", defaultCard: "PaymentCard" },
}

/**
 * Resolves and renders the appropriate card component for a given tool name and payload.
 * Gracefully handles validation failures, unknown statuses, and tool execution errors.
 */
export function renderCard({ toolName, envelope, events }: CardRendererProps): React.ReactElement {
  const spokenText = envelope.content?.[0]?.text
  const payload = envelope.structuredContent
  const isError = envelope.isError === true

  // If tool has an explicit "no_card" entry, return a minimal fallback notice
  const declaration = TOOL_CARD_DECLARATIONS[toolName]
  if (declaration?.type === "no_card") {
    return (
      <StatusCard
        title="Security Notice"
        status="token_rotated"
        message="Profile security token rotated successfully. The token is kept in memory only."
        spokenText={spokenText}
      />
    )
  }

  // Runtime validation
  const validation = validateCardPayload(toolName, payload)
  if (!validation.valid || !payload) {
    return (
      <FallbackCard
        toolName={toolName}
        spokenText={spokenText}
        errorReason={validation.error || "Unexpected payload structure"}
      />
    )
  }

  const status = typeof payload.status === "string" ? payload.status : "default"

  // Route by tool name and status
  switch (toolName) {
    case "search_menu":
      return <MenuCarousel structuredContent={payload} spokenText={spokenText} />

    case "check_availability":
      return (
        <AvailabilityCard
          structuredContent={payload}
          spokenText={spokenText}
          events={events}
        />
      )

    case "negotiate_booking":
      return (
        <NegotiationOptionsCard
          structuredContent={payload}
          spokenText={spokenText}
          events={events}
        />
      )

    case "create_booking":
      if (status === "pending" && payload.payment) {
        return (
          <PaymentCard
            payment={payload.payment}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      if (status === "confirmed" && payload.booking) {
        return (
          <BookingConfirmationCard
            booking={payload.booking}
            cancellationPolicy={payload.cancellation_policy}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Booking Request"
          status={status}
          message={payload.reason_detail}
          reason={payload.reason}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "get_booking":
      if (status === "found" && payload.booking) {
        return (
          <BookingConfirmationCard
            booking={payload.booking}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Booking Search"
          status={status}
          message={payload.message || "No reservation found for the provided details."}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "cancel_booking":
      return (
        <StatusCard
          title="Reservation Cancellation"
          status={status}
          message={
            status === "cancelled"
              ? "Your table reservation has been cancelled."
              : payload.reason_detail || "Cancellation request processed."
          }
          policyNote={payload.policy_note}
          reason={payload.reason}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "place_order":
      if (status === "placed" && payload.order) {
        return (
          <OrderSummaryCard
            order={payload.order}
            priceChanges={payload.price_changes}
            allergenNotice={payload.allergen_notice}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Order Placement"
          status={status}
          message={payload.reason_detail || "Order could not be placed."}
          reason={payload.reason}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "get_order":
      if (status === "found" && payload.order) {
        return (
          <OrderSummaryCard
            order={payload.order}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Order Lookup"
          status={status}
          message={payload.message || "No order found."}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "cancel_order":
      return (
        <StatusCard
          title="Order Cancellation"
          status={status}
          message={
            status === "cancelled"
              ? "Your pickup order has been cancelled."
              : payload.reason_detail || "Order cancellation status updated."
          }
          policyNote={payload.policy_note}
          reason={payload.reason}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "get_usual_order":
      if (status === "suggested" && payload.suggested_lines?.length > 0) {
        // Adapt suggested order to OrderSummary format
        const mockOrder = {
          order_code: "USUAL-ORDER",
          guest_name: payload.guest_name || "Guest",
          status: "suggested",
          pickup_time_local: "Today at earliest kitchen availability",
          lines: payload.suggested_lines.map((l: any) => ({
            item_id: l.item_id,
            name: l.item_name,
            quantity: l.quantity,
            unit_price: l.unit_price,
            line_total: l.line_total,
            allergens: [],
            dietary_tags: [],
            notes: l.user_provided_note,
          })),
          subtotal: payload.subtotal,
          tax: payload.tax,
          total: payload.total,
          payment_status: "unpaid_pay_at_pickup",
          created_at: new Date().toISOString(),
        }
        return (
          <OrderSummaryCard
            order={mockOrder}
            priceChanges={payload.price_changes}
            allergenNotice={payload.allergen_notice}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Your Usual Order"
          status={status}
          message="No prior order history was found for your profile. Ask for today's specials or browse the menu!"
          spokenText={spokenText}
        />
      )

    case "get_deposit_status":
    case "refresh_payment_session":
      return (
        <PaymentCard
          payment={payload.payment}
          depositStatusOutput={payload}
          spokenText={spokenText}
          events={events}
        />
      )

    case "accept_offer":
      if (status === "confirmed" && payload.booking) {
        return (
          <BookingConfirmationCard
            booking={payload.booking}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      if (payload.payment) {
        return (
          <PaymentCard
            payment={payload.payment}
            spokenText={spokenText}
            events={events}
          />
        )
      }
      return (
        <StatusCard
          title="Offer Acceptance"
          status={status}
          message={payload.message || "Offer status updated."}
          spokenText={spokenText}
          isError={isError}
        />
      )

    case "join_waitlist":
      return (
        <WaitlistCard
          joinOutput={payload}
          spokenText={spokenText}
          events={events}
        />
      )

    case "get_waitlist_status":
      return (
        <WaitlistCard
          statusOutput={payload}
          spokenText={spokenText}
          events={events}
        />
      )

    case "leave_waitlist":
      return (
        <StatusCard
          title="Waitlist Status"
          status={status}
          message={payload.message || "You have left the waitlist."}
          spokenText={spokenText}
        />
      )

    case "release_offer":
      return (
        <StatusCard
          title="Offer Released"
          status={status}
          message={payload.message || "Held table has been released."}
          spokenText={spokenText}
        />
      )

    case "get_profile":
      return <ProfileCard profile={payload} spokenText={spokenText} />

    case "enable_memory":
      return <ProfileCard enableMemory={payload} spokenText={spokenText} />

    case "remember_preference":
      return (
        <StatusCard
          title="Memory Updated"
          status={status}
          message={payload.message || "Dining preference remembered."}
          spokenText={spokenText}
        />
      )

    case "forget_preference":
      return (
        <StatusCard
          title="Preference Removed"
          status={status}
          message={payload.message || "Preference forgotten."}
          policyNote={payload.policy_note}
          spokenText={spokenText}
        />
      )

    case "forget_me":
      return (
        <StatusCard
          title="Privacy: Data Erased"
          status={status}
          message={payload.message || "All saved preferences and personal data have been permanently erased."}
          spokenText={spokenText}
        />
      )

    case "ping_restaurant":
      return (
        <StatusCard
          title="TableTalk Service"
          status="ok"
          message={`Connected to ${payload.service || "TableTalk"} v${payload.version || "0.1.0"}. Server active and responding.`}
          spokenText={spokenText}
        />
      )

    default:
      // Unknown tool fallback
      return (
        <FallbackCard
          toolName={toolName}
          spokenText={spokenText}
          errorReason="No registered visual card component for this tool"
        />
      )
  }
}
