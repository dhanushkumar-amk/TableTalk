import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { HomePage } from "../pages/HomePage"

function renderLandingPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={["/"]}>
          <HomePage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Landing Page (HomePage)", () => {
  it("renders the hero headline and subtext", () => {
    renderLandingPage()
    expect(screen.getByText("Book a table by just asking.")).toBeTruthy()
    expect(
      screen.getByText(/checks live table capacity, negotiates alternatives/i)
    ).toBeTruthy()
  })

  it("renders the two primary CTAs with correct routing links", () => {
    renderLandingPage()
    const talkCta = screen.getAllByRole("link", { name: /talk to the assistant/i })[0]
    const menuCta = screen.getAllByRole("link", { name: /view menu/i })[0]

    expect(talkCta).toBeTruthy()
    expect(talkCta.getAttribute("href")).toBe("/assistant")
    expect(menuCta).toBeTruthy()
    expect(menuCta.getAttribute("href")).toBe("/menu")
  })

  it("renders the 3-step How It Works flow", () => {
    renderLandingPage()
    expect(screen.getByText("How it works")).toBeTruthy()
    expect(screen.getByText("Ask naturally")).toBeTruthy()
    expect(screen.getByText("Explore options")).toBeTruthy()
    expect(screen.getByText("Confirm instantly")).toBeTruthy()
  })

  it("renders all 6 bento feature grid items", () => {
    renderLandingPage()
    expect(screen.getByText("Smart alternatives")).toBeTruthy()
    expect(screen.getByText("Allergen-aware menu")).toBeTruthy()
    expect(screen.getByText("Pickup orders")).toBeTruthy()
    expect(screen.getByText("Deposits (simulated)")).toBeTruthy()
    expect(screen.getByText("Fair FIFO waitlist")).toBeTruthy()
    expect(screen.getByText("Privacy-first memory")).toBeTruthy()
  })

  it("contains safety-critical disclaimer with zero absolute 'safe' or 'allergy-free' guarantees", () => {
    const { container } = renderLandingPage()
    const textContent = container.textContent || ""

    // Never state allergy-free or 100% safe
    expect(textContent.toLowerCase()).not.toContain("allergy-free")
    expect(textContent.toLowerCase()).not.toContain("100% safe")
    expect(textContent.toLowerCase()).not.toContain("completely safe")

    // Must disclose cross-contact risk notice
    expect(textContent).toMatch(/cross-contact/i)
  })

  it("renders trust strip and hackathon attribution footer without unauthorized logos", () => {
    const { container } = renderLandingPage()
    expect(
      screen.getByText(/Demo project\. No real payments\. Your data can be deleted any time\./i)
    ).toBeTruthy()

    expect(
      screen.getByText(/Built for the Amazon Developer Hackathon \(Alexa\+ track\)/i)
    ).toBeTruthy()

    // Ensure no third-party branding logos
    expect(container.querySelector("img")).toBeNull()
  })
})
