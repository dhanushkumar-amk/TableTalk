import { describe, it, expect, vi } from "vitest"
import { render, screen, act } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { ThemeProvider } from "../lib/theme"
import { TopNav } from "../components/shell/TopNav"
import { BottomTabBar } from "../components/shell/BottomTabBar"
import { CommandPalette } from "../components/shell/CommandPalette"
import { NotFoundPage } from "../pages/NotFoundPage"
import {
  PageHeader,
  Section,
  EmptyState,
  StatusBadge,
  Money,
  CodeBlock,
  CountdownChip,
  SimulatedBanner,
  CardSkeleton,
  TextSkeleton,
} from "../components/primitives"
import { UtensilsCrossed } from "lucide-react"

function renderWithProviders(ui: React.ReactElement, initialEntries = ["/"]) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  return render(
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <MemoryRouter initialEntries={initialEntries}>{ui}</MemoryRouter>
      </ThemeProvider>
    </QueryClientProvider>
  )
}

describe("Design System Primitives", () => {
  it("renders PageHeader with title, eyebrow, and actions", () => {
    render(
      <PageHeader
        title="Reservations"
        eyebrow="Dining Room"
        description="Select a table"
        badge={<StatusBadge status="accent" label="Active" />}
        actions={<button type="button">New Booking</button>}
      />
    )

    expect(screen.getByText("Reservations")).toBeTruthy()
    expect(screen.getByText("Dining Room")).toBeTruthy()
    expect(screen.getByText("Select a table")).toBeTruthy()
    expect(screen.getByText("Active")).toBeTruthy()
    expect(screen.getByText("New Booking")).toBeTruthy()
  })

  it("renders Section with title, description, and children", () => {
    render(
      <Section title="Guest Preferences" description="Dietary tags">
        <p>No shellfish</p>
      </Section>
    )

    expect(screen.getByText("Guest Preferences")).toBeTruthy()
    expect(screen.getByText("Dietary tags")).toBeTruthy()
    expect(screen.getByText("No shellfish")).toBeTruthy()
  })

  it("renders EmptyState with icon, title, and action buttons", () => {
    render(
      <EmptyState
        icon={UtensilsCrossed}
        title="No items found"
        description="Try adjusting your filters"
        action={<button type="button">Reset</button>}
      />
    )

    expect(screen.getByText("No items found")).toBeTruthy()
    expect(screen.getByText("Try adjusting your filters")).toBeTruthy()
    expect(screen.getByText("Reset")).toBeTruthy()
  })

  it("renders StatusBadge with accessible text and never color alone", () => {
    render(<StatusBadge status="warning" label="Payment Required" />)
    const badge = screen.getByText("Payment Required")
    expect(badge).toBeTruthy()
    expect(badge.parentElement?.querySelector("svg")).toBeTruthy()
  })

  it("renders Money with correct currency formatting and accessible aria-label", () => {
    render(<Money amountMinor={150000} currency="INR" />)
    const el = screen.getByText("₹1,500")
    expect(el).toBeTruthy()
    expect(el.getAttribute("aria-label")).toContain("1,500 Rupees")
  })

  it("renders CodeBlock with monospace font and copy button", async () => {
    const user = userEvent.setup()
    const writeTextSpy = vi.spyOn(navigator.clipboard, "writeText")
    render(<CodeBlock code="TT-89ABC" label="Confirmation" />)

    expect(screen.getByText("TT-89ABC")).toBeTruthy()
    const copyBtn = screen.getByLabelText(/Copy/i)
    expect(copyBtn).toBeTruthy()

    await user.click(copyBtn)
    expect(writeTextSpy).toHaveBeenCalledWith("TT-89ABC")
  })

  it("renders CountdownChip with live ticking display", () => {
    render(<CountdownChip secondsRemaining={600} label="Hold remaining" />)
    expect(screen.getByText(/Hold remaining/i)).toBeTruthy()
    expect(screen.getByText("10:00")).toBeTruthy()
  })

  it("renders SimulatedBanner with calm non-misleading text", () => {
    render(<SimulatedBanner type="payment" />)
    expect(screen.getByText(/SIMULATED PAYMENT/i)).toBeTruthy()
  })

  it("renders Skeletons without crashing", () => {
    const { container: c1 } = render(<CardSkeleton />)
    const { container: c2 } = render(<TextSkeleton lines={4} />)
    expect(c1.firstChild).toBeTruthy()
    expect(c2.firstChild).toBeTruthy()
  })
})

describe("App Shell Components", () => {
  it("renders TopNav with brand wordmark, links, and search trigger", () => {
    renderWithProviders(<TopNav />)
    expect(screen.getByText("TableTalk")).toBeTruthy()
    expect(screen.getByText("Assistant")).toBeTruthy()
    expect(screen.getByText("Menu")).toBeTruthy()
    expect(screen.getByText("Reserve")).toBeTruthy()
    expect(screen.getByText("Order")).toBeTruthy()
    expect(screen.getByText("Manage")).toBeTruthy()
    expect(screen.getByText("Privacy")).toBeTruthy()
  })

  it("renders BottomTabBar with all 5 mobile items", () => {
    renderWithProviders(<BottomTabBar />)
    expect(screen.getAllByText("Assistant").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Menu").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Reserve").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Order").length).toBeGreaterThan(0)
    expect(screen.getAllByText("Manage").length).toBeGreaterThan(0)
  })

  it("renders NotFoundPage with helpful return options", () => {
    renderWithProviders(<NotFoundPage />, ["/random-missing-url"])
    expect(screen.getByText("Page not found")).toBeTruthy()
    expect(screen.getByText("Talk to Assistant")).toBeTruthy()
  })

  it("opens CommandPalette on keyboard shortcut or custom event", async () => {
    renderWithProviders(<CommandPalette />)
    expect(screen.queryByPlaceholderText(/Type a command/i)).toBeNull()

    act(() => {
      window.dispatchEvent(new CustomEvent("tabletalk:open-command-palette"))
    })

    expect(screen.getByPlaceholderText(/Type a command/i)).toBeTruthy()
    expect(screen.getByText("Voice Assistant")).toBeTruthy()
  })
})
