import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { ManagePage } from "../pages/ManagePage"

function renderManagePage(initialEntries = ["/manage"]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={initialEntries}>
          <ManagePage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Manage Page (/manage)", () => {
  it("renders 3 navigation tabs and lookup form", () => {
    renderManagePage()

    expect(screen.getByRole("tab", { name: /Booking/i })).toBeTruthy()
    expect(screen.getByRole("tab", { name: /Order/i })).toBeTruthy()
    expect(screen.getByRole("tab", { name: /Waitlist/i })).toBeTruthy()
    expect(screen.getByPlaceholderText(/e\.g\. TT-849201/i)).toBeTruthy()
    expect(screen.getByPlaceholderText(/e\.g\. Priya Sharma/i)).toBeTruthy()
  })

  it("switches tabs and updates code input placeholder", async () => {
    const user = userEvent.setup()
    renderManagePage()

    const orderTab = screen.getByRole("tab", { name: /Order/i })
    await user.click(orderTab)

    expect(screen.getByPlaceholderText(/e\.g\. TT-ORD-1092/i)).toBeTruthy()
  })

  it("looks up dining reservation and displays summary with cancellation action", async () => {
    const user = userEvent.setup()
    renderManagePage()

    const codeInput = screen.getByPlaceholderText(/e\.g\. TT-849201/i)
    const nameInput = screen.getByPlaceholderText(/e\.g\. Priya Sharma/i)

    await user.type(codeInput, "TT-849201")
    await user.type(nameInput, "Priya Sharma")

    const lookupBtn = screen.getByRole("button", { name: /Look up Details/i })
    await user.click(lookupBtn)

    // Result card displays
    expect(await screen.findByText("Dining Reservation")).toBeTruthy()
    expect(screen.getByText(/T01/)).toBeTruthy()
    expect(screen.getByRole("button", { name: /Cancel Booking/i })).toBeTruthy()

    // Test cancellation confirmation dialog
    await user.click(screen.getByRole("button", { name: /Cancel Booking/i }))
    expect(await screen.findByText("Cancel Reservation?")).toBeTruthy()
    await user.click(screen.getByRole("button", { name: "Confirm Cancellation" }))

    // Status changes to Cancelled
    expect(await screen.findByText("Cancelled")).toBeTruthy()
  })

  it("looks up pickup order and displays timeline stepper and items", async () => {
    const user = userEvent.setup()
    renderManagePage(["/manage?tab=order"])

    const codeInput = screen.getByPlaceholderText(/e\.g\. TT-ORD-1092/i)
    const nameInput = screen.getByPlaceholderText(/e\.g\. Priya Sharma/i)

    await user.type(codeInput, "TT-ORD-1092")
    await user.type(nameInput, "Vikram Malhotra")
    await user.click(screen.getByRole("button", { name: /Look up Details/i }))

    expect(await screen.findByText("Pickup Order")).toBeTruthy()
    expect(screen.getByText("Order Timeline")).toBeTruthy()
    expect(screen.getByText(/Dum Gosht Biryani/i)).toBeTruthy()
  })

  it("displays generic not found message without revealing whether code vs name was incorrect", async () => {
    const user = userEvent.setup()
    renderManagePage()

    const codeInput = screen.getByPlaceholderText(/e\.g\. TT-849201/i)
    const nameInput = screen.getByPlaceholderText(/e\.g\. Priya Sharma/i)

    await user.type(codeInput, "invalid")
    await user.type(nameInput, "unknown")
    await user.click(screen.getByRole("button", { name: /Look up Details/i }))

    // Generic friendly message
    const errorMsg = await screen.findByText(/No matching record found\. Please verify the confirmation code and guest name/i)
    expect(errorMsg).toBeTruthy()
  })
})
