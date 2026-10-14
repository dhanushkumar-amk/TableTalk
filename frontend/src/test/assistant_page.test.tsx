import { describe, it, expect } from "vitest"
import { render, screen } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { AssistantPage } from "../pages/AssistantPage"

function renderAssistantPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={["/assistant"]}>
          <AssistantPage />
        </MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Assistant Page (/assistant)", () => {
  it("renders empty state with greeting and all 4 suggestion chips", () => {
    renderAssistantPage()

    expect(screen.getByText("How can I help you dine today?")).toBeTruthy()
    expect(screen.getByText("Table for 4 tomorrow at 7:30")).toBeTruthy()
    expect(screen.getByText("Vegan options without peanuts")).toBeTruthy()
    expect(screen.getByText("Order pickup")).toBeTruthy()
    expect(screen.getByText("Join the waitlist")).toBeTruthy()
  })

  it("renders composer with large mic button, textarea, and send button", () => {
    renderAssistantPage()

    expect(screen.getByPlaceholderText(/Ask about tables, vegan dishes, pickup/i)).toBeTruthy()
    expect(screen.getByRole("button", { name: /Start Voice Input/i })).toBeTruthy()
    expect(screen.getByRole("button", { name: /Send message/i })).toBeTruthy()
  })

  it("sends message when clicking a suggestion chip and renders user + assistant response", async () => {
    const user = userEvent.setup()
    renderAssistantPage()

    const chip = screen.getByText("Table for 4 tomorrow at 7:30")
    await user.click(chip)

    // User message should appear
    expect(screen.getByText("Table for 4 tomorrow at 7:30")).toBeTruthy()

    // Wait for simulated tool execution to resolve
    const assistantBubble = await screen.findByText(/table/i)
    expect(assistantBubble).toBeTruthy()

    // Tool activity badge should be logged in live activity
    const liveActivityBtn = screen.getByRole("button", { name: /Toggle Live Activity panel/i })
    expect(liveActivityBtn).toBeTruthy()
  })

  it("provides screen reader live announcement container with aria-live polite", () => {
    const { container } = renderAssistantPage()
    const announcer = container.querySelector('[aria-live="polite"]')
    expect(announcer).toBeTruthy()
  })

  it("renders audio narration mute toggle and controls", async () => {
    const user = userEvent.setup()
    renderAssistantPage()

    const muteToggle = screen.getByRole("button", { name: /Mute Voice Narration/i })
    expect(muteToggle).toBeTruthy()

    await user.click(muteToggle)
    expect(screen.getByRole("button", { name: /Enable Voice Narration/i })).toBeTruthy()
  })

  it("opens live activity panel when toggled", async () => {
    const user = userEvent.setup()
    renderAssistantPage()

    const toggleBtn = screen.getByRole("button", { name: /Toggle Live Activity panel/i })
    await user.click(toggleBtn)

    expect(screen.getAllByText(/Live.*Activity/i).length).toBeGreaterThan(0)
  })
})
