import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { render, screen, act } from "@testing-library/react"
import userEvent from "@testing-library/user-event"

import { renderCard, TOOL_CARD_DECLARATIONS } from "../cards/registry"
import { translateReasonCode, REASON_CODE_MAP } from "../cards/reasonMapping"
import { validateCardPayload } from "../cards/schemaValidation"
import { GALLERY_FIXTURES } from "../dev/CardGallery"
import outputSchemas from "../cards/schemas/output_schemas.json"

// Component imports
import { BookingConfirmationCard } from "../cards/components/BookingConfirmationCard"
import { NegotiationOptionsCard } from "../cards/components/NegotiationOptionsCard"
import { PaymentCard } from "../cards/components/PaymentCard"

describe("Card Registry & Unit Rendering", () => {
  it("renders every fixture from real server outputs without errors", () => {
    for (const fixture of GALLERY_FIXTURES) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )
      expect(container.firstChild).toBeTruthy()
    }
  })

  it("renders FallbackCard gracefully when payload is null, corrupt, or invalid", () => {
    const invalidEnvelope = {
      content: [{ type: "text", text: "Something went wrong in the speech pipeline." }],
      structuredContent: null,
    }

    render(
      renderCard({
        toolName: "check_availability",
        envelope: invalidEnvelope,
      })
    )

    expect(screen.getAllByText(/Something went wrong in the speech pipeline/i).length).toBeGreaterThan(0)
    expect(screen.queryByText(/NaN/)).toBeNull()
  })

  it("validates payloads correctly and rejects corrupted objects", () => {
    expect(validateCardPayload("search_menu", {}).valid).toBe(false)
    expect(validateCardPayload("search_menu", { items: [] }).valid).toBe(true)
    expect(validateCardPayload("unknown_random_tool", { status: "ok" }).valid).toBe(false)
    expect(validateCardPayload("check_availability", null).valid).toBe(false)
  })

  it("ensures every tool in output_schemas.json has a registered card or an explicit 'no_card' declaration", () => {
    const toolNames = Object.keys(outputSchemas)
    expect(toolNames.length).toBe(24)

    for (const name of toolNames) {
      const declaration = TOOL_CARD_DECLARATIONS[name]
      expect(
        declaration,
        `Tool '${name}' must have a registered card or an explicit 'no_card' entry`
      ).toBeDefined()
    }
  })
})

describe("Reason Code Mapping Exhaustiveness", () => {
  it("translates known reason codes to descriptive human phrases", () => {
    expect(translateReasonCode("fully_booked")).toBe("Fully booked at this time")
    expect(translateReasonCode("exact_time")).toBe("Exact match for your requested time")
    expect(translateReasonCode("deposit_required")).toBe("Table deposit required")
    expect(translateReasonCode("closed_day")).toBe("Restaurant is closed on this day")
  })

  it("falls back gracefully to Title Case for novel/unmapped server reason codes", () => {
    const result = translateReasonCode("special_unforeseen_closure")
    expect(result).toBe("Special Unforeseen Closure")
  })

  it("contains mappings for all canonical reason codes defined in the design spec", () => {
    const criticalReasons = [
      "exact_time",
      "same_day",
      "exact_capacity_fit",
      "good_capacity_fit",
      "comfortable_dining_window",
      "fully_booked",
      "closed_day",
      "special_closure",
      "outside_hours",
      "too_soon",
      "beyond_horizon",
      "party_too_large",
      "no_single_table",
      "deposit_required",
      "hold_expired",
      "item_unavailable",
      "allergen_conflict",
    ]

    for (const r of criticalReasons) {
      expect(REASON_CODE_MAP[r], `Missing human mapping for reason '${r}'`).toBeDefined()
    }
  })
})

describe("Countdown Timer & Fake Timers Logic", () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it("counts down correctly without exceeding deadline and stops cleanly at zero", () => {
    const structuredContent = {
      status: "options_found",
      message: "Options found",
      options: [
        {
          option_id: "opt_1",
          rank: 1,
          start_local: "19:30",
          end_local: "21:00",
          area: "indoor",
          table_label: "T01",
          minutes_from_requested: 0,
          score_reasons: ["exact_time"],
          hold: {
            hold_id: 1,
            start_local: "19:30",
            end_local: "21:00",
            area: "indoor",
            table_label: "T01",
            party_size: 2,
            seconds_remaining: 10,
            expires_at_local: "2026-10-16T19:30:10",
          },
        },
      ],
    }

    render(
      <NegotiationOptionsCard
        structuredContent={structuredContent as any}
      />
    )

    // Initial 10 seconds remaining
    expect(screen.getByText(/Hold: 00:10/i)).toBeTruthy()

    // Advance 5 seconds
    act(() => {
      vi.advanceTimersByTime(5000)
    })
    expect(screen.getByText(/Hold: 00:05/i)).toBeTruthy()

    // Advance past expiration
    act(() => {
      vi.advanceTimersByTime(10000)
    })
    expect(screen.getByText(/Hold: 00:00/i)).toBeTruthy()
  })
})

describe("Copy to Clipboard Interaction", () => {
  it("copies booking confirmation code and emits onCopyCode event", async () => {
    const onCopyCode = vi.fn()
    const user = userEvent.setup()

    render(
      <BookingConfirmationCard
        booking={{
          confirmation_code: "TT-TEST-CODE",
          guest_name: "Test Diner",
          party_size: 2,
          area: "indoor",
          table_label: "T01",
          start_local: "2026-10-16T19:30:00+05:30",
          end_local: "2026-10-16T21:00:00+05:30",
          status: "confirmed",
          created_at: "2026-10-16T12:00:00Z",
        }}
        events={{ onCopyCode }}
      />
    )

    const writeSpy = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined)
    const copyBtn = screen.getByRole("button", { name: /copy confirmation code/i })
    await user.click(copyBtn)

    expect(writeSpy).toHaveBeenCalledWith("TT-TEST-CODE")
    expect(onCopyCode).toHaveBeenCalledWith({
      code: "TT-TEST-CODE",
      label: "Booking Confirmation Code",
    })
    writeSpy.mockRestore()
  })
})

describe("Card Event Emission Contracts", () => {
  it("emits onAcceptOption when Accept Option is clicked", async () => {
    const onAcceptOption = vi.fn()
    const user = userEvent.setup()

    const structuredContent = {
      status: "options_found",
      message: "Options found",
      options: [
        {
          option_id: "opt_test_1",
          rank: 1,
          start_local: "19:30",
          end_local: "21:00",
          area: "indoor",
          table_label: "T01",
          minutes_from_requested: 0,
          score_reasons: ["exact_time"],
        },
      ],
    }

    render(
      <NegotiationOptionsCard
        structuredContent={structuredContent as any}
        events={{ onAcceptOption }}
      />
    )

    const acceptBtn = screen.getByRole("button", { name: /accept option/i })
    await user.click(acceptBtn)

    expect(onAcceptOption).toHaveBeenCalledWith({
      optionId: "opt_test_1",
    })
  })

  it("requires confirmation before emitting destructive cancel action", async () => {
    const onCancelBooking = vi.fn()
    const user = userEvent.setup()

    render(
      <BookingConfirmationCard
        booking={{
          confirmation_code: "TT-CANCEL-ME",
          guest_name: "Aarav Sharma",
          party_size: 2,
          area: "patio",
          table_label: "P01",
          start_local: "2026-10-16T19:30:00+05:30",
          end_local: "2026-10-16T21:00:00+05:30",
          status: "confirmed",
          created_at: "2026-10-16T12:00:00Z",
        }}
        events={{ onCancelBooking }}
      />
    )

    // First click opens confirm step
    const cancelBtn = screen.getByRole("button", { name: /cancel reservation/i })
    await user.click(cancelBtn)
    expect(onCancelBooking).not.toHaveBeenCalled()

    // Second click confirms
    const confirmBtn = screen.getByRole("button", { name: /yes, cancel reservation/i })
    await user.click(confirmBtn)
    expect(onCancelBooking).toHaveBeenCalledWith({
      confirmationCode: "TT-CANCEL-ME",
      guestName: "Aarav Sharma",
    })
  })

  it("emits onOpenCheckout with URL target without rendering checkout URL text", async () => {
    const onOpenCheckout = vi.fn()
    const user = userEvent.setup()

    render(
      <PaymentCard
        payment={{
          amount: { amount_minor: 200000, currency: "INR", display: "₹2000" },
          checkout_url: "http://localhost:5173/checkout/tt_pay_secret_token_12345",
          deadline_local: "2026-10-16T19:45:00+05:30",
          minutes_remaining: 15,
          required: true,
          simulated: true,
          status: "requires_payment",
        }}
        events={{ onOpenCheckout }}
      />
    )

    const payBtn = screen.getByRole("button", { name: /pay deposit \(simulated\)/i })
    await user.click(payBtn)

    expect(onOpenCheckout).toHaveBeenCalledWith({
      checkoutUrl: "http://localhost:5173/checkout/tt_pay_secret_token_12345",
    })

    // Assert checkout_url is NOT visible as text on screen
    expect(screen.queryByText(/tt_pay_secret_token_12345/)).toBeNull()
  })
})
