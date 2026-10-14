import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { OrderPage } from "../pages/OrderPage"

function renderOrderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={["/order"]}>
          <OrderPage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Order Page (/order)", () => {
  it("renders menu list and empty cart sidebar", () => {
    renderOrderPage()

    expect(screen.getByText("Pickup Order")).toBeTruthy()
    expect(screen.getByText("Your Pickup Cart")).toBeTruthy()
    expect(screen.getByText(/Your pickup cart is empty/i)).toBeTruthy()
  })

  it("adds item to cart, increments quantity, and updates total price", async () => {
    const user = userEvent.setup()
    renderOrderPage()

    // Add first item (Palak Chaat)
    const addBtns = screen.getAllByRole("button", { name: /^add$/i })
    await user.click(addBtns[0])

    // Should appear in cart summary
    expect(screen.getAllByText("Crisp Palak Chaat").length).toBeGreaterThan(0)
    expect(screen.getByText("1 items")).toBeTruthy()

    // Increment quantity
    const incBtn = screen.getByRole("button", { name: "Increase" })
    await user.click(incBtn)
    expect(screen.getByText("2 items")).toBeTruthy()
  })

  it("triggers calm allergen conflict dialog when avoided allergen matches item", async () => {
    const user = userEvent.setup()
    renderOrderPage()

    // Add Palak Chaat (contains dairy)
    const addBtns = screen.getAllByRole("button", { name: /^add$/i })
    await user.click(addBtns[0])

    // Click "Dairy" in "Anything to avoid?"
    const dairyBtn = screen.getByRole("button", { name: "Dairy" })
    await user.click(dairyBtn)

    // Conflict dialog opens
    expect(await screen.findByText(/Allergen Notice: Dairy/i)).toBeTruthy()
    expect(screen.getAllByText(/Crisp Palak Chaat/i).length).toBeGreaterThan(0)
    expect(screen.getByRole("button", { name: /Remove Conflicting Item/i })).toBeTruthy()

    // Confirm never says "safe"
    const modalText = screen.getByRole("dialog").textContent || ""
    expect(modalText.toLowerCase()).not.toContain("safe")
    expect(modalText.toLowerCase()).not.toContain("allergy-free")
  })

  it("validates customer name, places pickup order, and displays code block confirmation", async () => {
    const user = userEvent.setup()
    renderOrderPage()

    // Add item
    const addBtns = screen.getAllByRole("button", { name: /^add$/i })
    await user.click(addBtns[0])

    // Enter customer name
    const nameInput = screen.getByPlaceholderText(/e\.g\. Vikram Malhotra/i)
    await user.type(nameInput, "Kavita Rao")

    // Place order
    const placeBtn = screen.getByRole("button", { name: /Place Pickup Order/i })
    await user.click(placeBtn)

    // Order confirmation view
    expect(await screen.findByText("Pickup Order Confirmed!")).toBeTruthy()
    expect(screen.getByText(/Ready for Pickup/i)).toBeTruthy()
    expect(screen.getByRole("button", { name: /Cancel Order/i })).toBeTruthy()
  })

  it("opens cancel order dialog and allows confirming cancellation", async () => {
    const user = userEvent.setup()
    renderOrderPage()

    // Add and place order
    const addBtns = screen.getAllByRole("button", { name: /^add$/i })
    await user.click(addBtns[0])
    const nameInput = screen.getByPlaceholderText(/e\.g\. Vikram Malhotra/i)
    await user.type(nameInput, "Kavita Rao")
    await user.click(screen.getByRole("button", { name: /Place Pickup Order/i }))

    // Click Cancel Order
    const cancelBtn = await screen.findByRole("button", { name: /Cancel Order/i })
    await user.click(cancelBtn)

    expect(await screen.findByText("Cancel Pickup Order?")).toBeTruthy()
    const confirmCancelBtn = screen.getByRole("button", { name: "Confirm Cancellation" })
    await user.click(confirmCancelBtn)

    // Should return to empty cart ordering view
    expect(await screen.findByText(/Your pickup cart is empty/i)).toBeTruthy()
  })
})
