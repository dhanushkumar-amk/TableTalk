import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, waitFor } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { ThemeProvider } from "../lib/theme"
import CheckoutPage from "../pages/CheckoutPage"

function renderCheckoutPage(token: string) {
  return render(
    <ThemeProvider>
      <MemoryRouter>
        <CheckoutPage token={token} />
      </MemoryRouter>
    </ThemeProvider>
  )
}

describe("Checkout Page (/checkout/:token)", () => {
  const mockToken = "secret-token-xyz-12345"

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it("renders active session with restaurant name, deposit amount, party size, and simulated banner", async () => {
    const mockSession = {
      status: "open",
      simulated: true,
      restaurant_name: "The Saffron Nebula Bistro",
      amount: { amount_minor: 100000, currency: "INR", display: "₹1,000" },
      deadline_local: new Date(Date.now() + 600000).toISOString(),
      party_size: 4,
      start_at_local: new Date(Date.now() + 86400000).toISOString(),
    }

    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: true,
      json: async () => mockSession,
    } as Response)

    renderCheckoutPage(mockToken)

    // Wait for session to load
    await waitFor(() => {
      expect(screen.getByText("The Saffron Nebula Bistro")).toBeTruthy()
    })

    expect(screen.getByRole("heading", { name: "Table Deposit Hold" })).toBeTruthy()
    expect(screen.getByText("SIMULATED PAYMENT — no real money moves")).toBeTruthy()
    expect(screen.getByText("4 Guests")).toBeTruthy()
    expect(screen.getByRole("button", { name: /simulate success/i })).toBeTruthy()
    expect(screen.getByRole("button", { name: /simulate decline/i })).toBeTruthy()
    expect(screen.getByRole("button", { name: /simulate extra verification/i })).toBeTruthy()

    // Safety rule: token must NEVER appear on the screen
    expect(screen.queryByText(mockToken)).toBeNull()
  })

  it("handles simulate success outcome with confirmation code and back link", async () => {
    const user = userEvent.setup()

    const mockSession = {
      status: "open",
      simulated: true,
      restaurant_name: "The Saffron Nebula Bistro",
      amount: { amount_minor: 100000, currency: "INR", display: "₹1,000" },
      deadline_local: new Date(Date.now() + 600000).toISOString(),
    }

    const mockSuccessResponse = {
      status: "success",
      simulated: true,
      message: "Deposit confirmed. Your table is booked! Confirmation code: CONF-7890.",
      booking_confirmed: true,
    }

    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockSession,
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockSuccessResponse,
      } as Response)

    renderCheckoutPage(mockToken)

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /simulate success/i })).toBeTruthy()
    })

    await user.click(screen.getByRole("button", { name: /simulate success/i }))

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Deposit Confirmed" })).toBeTruthy()
    })

    expect(screen.getByText("CONF-7890")).toBeTruthy()
    const backBtn = screen.getByRole("link", { name: /back to assistant/i })
    expect(backBtn).toBeTruthy()
    expect(backBtn.getAttribute("href")).toBe("/assistant")
  })

  it("handles simulate decline outcome with calm message and allows retry", async () => {
    const user = userEvent.setup()

    const mockSession = {
      status: "open",
      simulated: true,
      amount: { amount_minor: 100000, currency: "INR", display: "₹1,000" },
      deadline_local: new Date(Date.now() + 600000).toISOString(),
    }

    const mockDeclineResponse = {
      status: "declined",
      simulated: true,
      message: "The payment was declined. The session stays open; 2 attempt(s) remaining.",
    }

    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockSession,
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockDeclineResponse,
      } as Response)

    renderCheckoutPage(mockToken)

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /simulate decline/i })).toBeTruthy()
    })

    await user.click(screen.getByRole("button", { name: /simulate decline/i }))

    await waitFor(() => {
      expect(screen.getByText("Payment Declined")).toBeTruthy()
    })

    expect(screen.getByText(/2 attempt\(s\) remaining/i)).toBeTruthy()
    // Retry buttons remain available
    expect(screen.getByRole("button", { name: /simulate success/i })).toBeTruthy()
  })

  it("handles step-up extra verification outcome", async () => {
    const user = userEvent.setup()

    const mockSession = {
      status: "open",
      simulated: true,
      amount: { amount_minor: 100000, currency: "INR", display: "₹1,000" },
      deadline_local: new Date(Date.now() + 600000).toISOString(),
    }

    const mockVerificationResponse = {
      status: "needs_verification",
      simulated: true,
      message: "Extra verification is required. Confirm again to complete the payment.",
    }

    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockSession,
      } as Response)
      .mockResolvedValueOnce({
        ok: true,
        json: async () => mockVerificationResponse,
      } as Response)

    renderCheckoutPage(mockToken)

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /simulate extra verification/i })).toBeTruthy()
    })

    await user.click(screen.getByRole("button", { name: /simulate extra verification/i }))

    await waitFor(() => {
      expect(screen.getByText("Step-up Verification")).toBeTruthy()
    })
    expect(screen.getByText(/Extra verification is required/i)).toBeTruthy()
  })

  it("displays generic not found state when token is invalid or completed", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: false,
      status: 404,
      json: async () => ({ status: "not_found" }),
    } as Response)

    renderCheckoutPage("invalid-token")

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Link No Longer Valid" })).toBeTruthy()
    })

    expect(
      screen.getByText(/it may have been completed, expired, or replaced/i)
    ).toBeTruthy()
    expect(screen.getByRole("link", { name: /back to assistant/i })).toBeTruthy()
  })

  it("displays expired hold state when deadline has elapsed", async () => {
    const mockExpiredSession = {
      status: "open",
      simulated: true,
      amount: { amount_minor: 100000, currency: "INR", display: "₹1,000" },
      deadline_local: new Date(Date.now() - 60000).toISOString(), // 1 minute in the past
    }

    vi.spyOn(globalThis, "fetch").mockResolvedValueOnce({
      ok: true,
      json: async () => mockExpiredSession,
    } as Response)

    renderCheckoutPage(mockToken)

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Hold Period Expired" })).toBeTruthy()
    })

    expect(
      screen.getByText(/the payment deadline passed and the temporary hold on your table was released/i)
    ).toBeTruthy()
  })
})
