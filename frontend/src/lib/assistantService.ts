import type { TableTalkToolName, ToolResponseEnvelope } from "@/cards/types"

// Fixture imports
import checkAvailabilityFixture from "@/cards/fixtures/check_availability_available.json"
import searchMenuFixture from "@/cards/fixtures/search_menu_found.json"
import placeOrderFixture from "@/cards/fixtures/place_order_placed.json"
import joinWaitlistFixture from "@/cards/fixtures/join_waitlist_joined.json"
import depositRequiredFixture from "@/cards/fixtures/create_booking_deposit_required.json"
import bookingConfirmedFixture from "@/cards/fixtures/create_booking_confirmed.json"
import negotiateOptionsFixture from "@/cards/fixtures/negotiate_booking_options.json"
import cancelBookingFixture from "@/cards/fixtures/cancel_booking_cancelled.json"
import cancelOrderFixture from "@/cards/fixtures/cancel_order_cancelled.json"
import acceptOfferFixture from "@/cards/fixtures/accept_offer_confirmed.json"
import leaveWaitlistFixture from "@/cards/fixtures/leave_waitlist_left.json"

export interface AssistantResponse {
  spokenText: string
  toolName: TableTalkToolName
  envelope: ToolResponseEnvelope<any>
  durationMs: number
  status: "success" | "error"
}

export interface LiveActivityItem {
  id: string
  timestamp: string
  toolName: TableTalkToolName
  status: "running" | "success" | "error"
  durationMs: number
  summary: string
}

/**
 * Dispatches a user query to the TableTalk assistant.
 * Analyzes intent, simulates or executes tool invocation, and returns spoken text + card envelope.
 */
export async function sendAssistantMessage(
  query: string,
  _profileToken?: string | null
): Promise<AssistantResponse> {
  const normalized = query.toLowerCase().trim()
  const startTime = performance.now()

  // Small artificial latency for natural voice cadence feel (180ms - 400ms)
  await new Promise((resolve) => setTimeout(resolve, 250))

  const durationMs = Math.round(performance.now() - startTime)

  // 1. Waitlist
  if (normalized.includes("waitlist") || normalized.includes("queue")) {
    return {
      spokenText:
        (joinWaitlistFixture.content?.[0] as { text: string })?.text ||
        "I've added you to the waitlist at position 2 for an indoor table. I'll notify you as soon as a party cancels.",
      toolName: "join_waitlist",
      envelope: joinWaitlistFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // 2. Deposit / Large Group demo
  if (
    normalized.includes("8") ||
    normalized.includes("eight") ||
    normalized.includes("deposit") ||
    normalized.includes("group")
  ) {
    return {
      spokenText:
        (depositRequiredFixture.content?.[0] as { text: string })?.text ||
        "A table for eight needs a deposit of two thousand rupees. I've put the payment on your screen. I'll hold the table for fifteen minutes.",
      toolName: "create_booking",
      envelope: depositRequiredFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // 3. Menu / Vegan / Allergen / Food search
  if (
    normalized.includes("vegan") ||
    normalized.includes("peanut") ||
    normalized.includes("menu") ||
    normalized.includes("food") ||
    normalized.includes("dish") ||
    normalized.includes("gluten") ||
    normalized.includes("curry")
  ) {
    return {
      spokenText:
        (searchMenuFixture.content?.[0] as { text: string })?.text ||
        "Here are our chef-recommended vegan selections with verified allergen labeling. Cross-contact risk is actively noted for every dish.",
      toolName: "search_menu",
      envelope: searchMenuFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // 4. Pickup Order
  if (
    normalized.includes("order") ||
    normalized.includes("pickup") ||
    normalized.includes("takeout")
  ) {
    return {
      spokenText:
        (placeOrderFixture.content?.[0] as { text: string })?.text ||
        "Your pickup order has been placed. Estimated prep time is 25 minutes. Pay at pickup is confirmed.",
      toolName: "place_order",
      envelope: placeOrderFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // 5. Booking / Reservation alternatives / Negotiation
  if (
    normalized.includes("alternative") ||
    normalized.includes("options") ||
    normalized.includes("conflict")
  ) {
    return {
      spokenText:
        (negotiateOptionsFixture.content?.[0] as { text: string })?.text ||
        "That exact slot is booked, but I found three ranked alternatives nearby. I can hold any of these for 5 minutes.",
      toolName: "negotiate_booking",
      envelope: negotiateOptionsFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // 6. Direct booking confirmation
  if (
    normalized.includes("confirm") ||
    normalized.includes("book it") ||
    normalized.includes("reserve table")
  ) {
    return {
      spokenText:
        (bookingConfirmedFixture.content?.[0] as { text: string })?.text ||
        "You're all set! I've booked Table T01 for you on Friday at 7:30 PM. Your confirmation code is TT-784920.",
      toolName: "create_booking",
      envelope: bookingConfirmedFixture as unknown as ToolResponseEnvelope<any>,
      durationMs,
      status: "success",
    }
  }

  // Default: Availability check
  return {
    spokenText:
      (checkAvailabilityFixture.content?.[0] as { text: string })?.text ||
      "I have an indoor table for you on Friday the sixteenth at seven thirty in the evening. Would you like me to book it?",
    toolName: "check_availability",
    envelope: checkAvailabilityFixture as unknown as ToolResponseEnvelope<any>,
    durationMs,
    status: "success",
  }
}

/**
 * Handles actions emitted by card events (e.g. Accept, Cancel, Leave Waitlist).
 */
export async function executeCardAction(
  action: "accept_offer" | "cancel_booking" | "cancel_order" | "leave_waitlist",
  _metadata?: Record<string, any>
): Promise<AssistantResponse> {
  const startTime = performance.now()
  await new Promise((resolve) => setTimeout(resolve, 200))
  const durationMs = Math.round(performance.now() - startTime)

  switch (action) {
    case "accept_offer":
      return {
        spokenText: "Your offer has been accepted! Table T01 is confirmed for your party.",
        toolName: "accept_offer",
        envelope: acceptOfferFixture as unknown as ToolResponseEnvelope<any>,
        durationMs,
        status: "success",
      }
    case "cancel_booking":
      return {
        spokenText: "Your reservation has been cancelled. Any eligible deposit has been refunded according to policy.",
        toolName: "cancel_booking",
        envelope: cancelBookingFixture as unknown as ToolResponseEnvelope<any>,
        durationMs,
        status: "success",
      }
    case "cancel_order":
      return {
        spokenText: "Your pickup order has been cancelled.",
        toolName: "cancel_order",
        envelope: cancelOrderFixture as unknown as ToolResponseEnvelope<any>,
        durationMs,
        status: "success",
      }
    case "leave_waitlist":
      return {
        spokenText: "You have been removed from the waitlist.",
        toolName: "leave_waitlist",
        envelope: leaveWaitlistFixture as unknown as ToolResponseEnvelope<any>,
        durationMs,
        status: "success",
      }
  }
}
