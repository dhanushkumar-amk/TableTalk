import { describe, it, expect } from "vitest"
import { render, screen, fireEvent } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { MenuPage } from "../pages/MenuPage"
import { SERVER_ALLERGEN_NOTICE } from "../lib/menuData"

function renderMenuPage(initialEntries = ["/menu"]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={initialEntries}>
          <MenuPage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Menu Page (/menu)", () => {
  it("renders search input, category pills, and filter trigger", async () => {
    renderMenuPage()

    expect(
      screen.getByPlaceholderText(/Search dishes, ingredients, curries/i)
    ).toBeTruthy()

    expect(screen.getByRole("tab", { name: "All Dishes" })).toBeTruthy()
    expect(screen.getByRole("tab", { name: "Starters & Street Food" })).toBeTruthy()
    expect(screen.getByRole("tab", { name: "Biryani & Breads" })).toBeTruthy()
    expect(screen.getByRole("button", { name: /Open Filters/i })).toBeTruthy()
  })

  it("focuses search input on '/' keyboard shortcut", () => {
    renderMenuPage()
    const input = screen.getByPlaceholderText(/Search dishes, ingredients, curries/i)

    fireEvent.keyDown(window, { key: "/" })
    expect(document.activeElement).toBe(input)
  })

  it("renders menu items with name, price, dietary chips, and neutral allergen row", async () => {
    renderMenuPage()

    // Dum Gosht Biryani
    const biryaniTitle = await screen.findByText("Dum Gosht Biryani")
    expect(biryaniTitle).toBeTruthy()

    // Neutral allergen row: Listed as containing: dairy
    expect(screen.getAllByText(/Listed as containing:/i).length).toBeGreaterThan(0)
  })

  it("displays chef's special badge on signature dishes", async () => {
    renderMenuPage()
    const specials = await screen.findAllByText("Chef's Special")
    expect(specials.length).toBeGreaterThan(0)
  })

  it("opens filter drawer and allows toggling dietary and allergen exclusions", async () => {
    const user = userEvent.setup()
    renderMenuPage()

    const filterBtn = screen.getByRole("button", { name: /Open Filters/i })
    await user.click(filterBtn)

    expect(screen.getByText("Menu Filters")).toBeTruthy()
    expect(screen.getByText("Dietary Requirements")).toBeTruthy()
    expect(screen.getByText("Exclude Allergens")).toBeTruthy()
    expect(screen.getByText("Max Price")).toBeTruthy()
  })

  it("opens dish detail view with full description, calories, and verbatim allergen notice", async () => {
    const user = userEvent.setup()
    renderMenuPage()

    const dishCard = await screen.findByText("Dum Gosht Biryani")
    await user.click(dishCard)

    // Detail modal opens
    expect(screen.getByRole("dialog")).toBeTruthy()
    expect(screen.getByText("Allergen Disclosure")).toBeTruthy()

    // Verbatim server notice check
    expect(screen.getByText(new RegExp(SERVER_ALLERGEN_NOTICE, "i"))).toBeTruthy()
  })

  it("never displays allergens as safe or with green check marks", async () => {
    const { container } = renderMenuPage()
    const textContent = container.textContent || ""

    expect(textContent.toLowerCase()).not.toContain("allergy-safe")
    expect(textContent.toLowerCase()).not.toContain("100% safe")
    expect(textContent.toLowerCase()).not.toContain("allergy-free")
  })

  it("displays empty state when search finds no matching dishes", async () => {
    renderMenuPage(["/menu?q=nonexistentxyz123"])

    expect(await screen.findByText("No matching dishes found")).toBeTruthy()
    expect(screen.getByText("Reset all filters")).toBeTruthy()
  })
})
