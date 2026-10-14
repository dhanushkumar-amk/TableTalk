import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { ReservePage } from "../pages/ReservePage"

function renderReservePage(initialEntries = ["/reserve"]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={initialEntries}>
          <ReservePage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Reserve Page (/reserve)", () => {
  it("renders Step 1 with party size stepper, date, area, and time chips", () => {
    renderReservePage()

    expect(screen.getByText("Reserve a Table")).toBeTruthy()
    expect(screen.getByText("Party Size")).toBeTruthy()
    expect(screen.getByLabelText("Decrease party size")).toBeTruthy()
    expect(screen.getByLabelText("Increase party size")).toBeTruthy()
    expect(screen.getByText("Indoor Dining")).toBeTruthy()
    expect(screen.getByText("Lunch Service (12:00 PM – 3:30 PM)")).toBeTruthy()
    expect(screen.getByText("Dinner Service (7:00 PM – 11:30 PM)")).toBeTruthy()
  })

  it("increments party size and displays deposit requirement notice at 8+ guests", async () => {
    const user = userEvent.setup()
    renderReservePage()

    const incBtn = screen.getByLabelText("Increase party size")

    // Default is 2 guests, click 6 times to reach 8
    for (let i = 0; i < 6; i++) {
      await user.click(incBtn)
    }

    expect(screen.getByText("8 guests")).toBeTruthy()
    expect(screen.getByText(/Requires ₹2,000 deposit hold/i)).toBeTruthy()
  })

  it("checks availability and renders snapshot disclaimer verbatim", async () => {
    const user = userEvent.setup()
    renderReservePage()

    const checkBtn = screen.getByRole("button", { name: /Check Table Availability/i })
    await user.click(checkBtn)

    expect(await screen.findByText("Table Available!")).toBeTruthy()
    expect(
      screen.getByText(/Availability is a snapshot, not a reservation/i)
    ).toBeTruthy()
  })

  it("displays ranked alternatives when peak conflict time is checked", async () => {
    const user = userEvent.setup()
    renderReservePage()

    // Pick 20:00 (configured conflict slot)
    const slot20 = screen.getByRole("button", { name: "20:00" })
    await user.click(slot20)

    const checkBtn = screen.getByRole("button", { name: /Check Table Availability/i })
    await user.click(checkBtn)

    expect(await screen.findByText(/Requested Time is Fully Booked/i)).toBeTruthy()
    expect(screen.getByText(/Closest to your time/i)).toBeTruthy()
    expect(screen.getByText(/Join Waitlist Instead/i)).toBeTruthy()
  })

  it("navigates through details, validates name, and confirms booking with code block", async () => {
    const user = userEvent.setup()
    renderReservePage()

    // Check availability
    await user.click(screen.getByRole("button", { name: /Check Table Availability/i }))
    expect(await screen.findByText("Table Available!")).toBeTruthy()

    // Continue to details
    await user.click(screen.getByRole("button", { name: /Continue to Guest Details/i }))
    expect(await screen.findByRole("heading", { name: "Guest Details" })).toBeTruthy()

    // Submit without name should trigger validation
    await user.click(screen.getByRole("button", { name: /Confirm Reservation/i }))
    expect(await screen.findByText(/Full name must be at least 2 characters/i)).toBeTruthy()

    // Type full name
    const nameInput = screen.getByPlaceholderText(/e\.g\. Priya Sharma/i)
    await user.type(nameInput, "Aarav Patel")

    // Confirm booking
    await user.click(screen.getByRole("button", { name: /Confirm Reservation/i }))

    expect(await screen.findByText("Reservation Confirmed!")).toBeTruthy()
    expect(screen.getByRole("button", { name: /Add to Calendar/i })).toBeTruthy()
    expect(screen.getByRole("link", { name: /Manage Reservation/i })).toBeTruthy()
  })
})
