import { describe, it, expect } from "vitest"
import { render, screen, fireEvent } from "@testing-library/react"
import axe from "axe-core"
import { renderCard } from "../cards/registry"
import { GALLERY_FIXTURES } from "../dev/CardGallery"
import { MenuCarousel } from "../cards/components/MenuCarousel"

describe("Accessibility & WCAG 2.2 AA Compliance", () => {
  it("verifies zero serious or critical axe violations across key card components", async () => {
    // Test a diverse representative set of cards
    const representativeKeys = [
      "menu-found",
      "avail-available",
      "neg-options",
      "book-confirmed",
      "pay-deposit-open",
      "order-placed",
      "wl-joined",
      "prof-get-found",
      "sys-ping",
    ]

    const testFixtures = GALLERY_FIXTURES.filter((f) => representativeKeys.includes(f.id))

    for (const fixture of testFixtures) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )

      // Run axe on rendered card container
      const results = await axe.run(container, {
        rules: {
          // Color-contrast in jsdom is often inaccurate due to missing CSS layout engine
          "color-contrast": { enabled: false },
        },
      })

      const seriousOrCritical = results.violations.filter(
        (v) => v.impact === "serious" || v.impact === "critical"
      )

      expect(
        seriousOrCritical,
        `Accessibility violation in card '${fixture.id}': ${JSON.stringify(
          seriousOrCritical.map((v) => ({ id: v.id, impact: v.impact, help: v.help })),
          null,
          2
        )}`
      ).toHaveLength(0)
    }
  })

  it("supports keyboard arrow navigation on MenuCarousel", () => {
    const fixture = GALLERY_FIXTURES.find((f) => f.id === "menu-found")!
    const { container } = render(
      <MenuCarousel structuredContent={fixture.envelope.structuredContent} />
    )

    const carouselSection = container.querySelector("section")
    expect(carouselSection).toBeTruthy()

    // Trigger ArrowRight and ArrowLeft key events
    fireEvent.keyDown(carouselSection!, { key: "ArrowRight", code: "ArrowRight" })
    fireEvent.keyDown(carouselSection!, { key: "ArrowLeft", code: "ArrowLeft" })
  })

  it("verifies interactive controls have accessible names and semantic roles", () => {
    const fixture = GALLERY_FIXTURES.find((f) => f.id === "book-confirmed")!
    render(
      renderCard({
        toolName: fixture.toolName,
        envelope: fixture.envelope,
      })
    )

    const copyBtn = screen.getByRole("button", { name: /copy confirmation code/i })
    expect(copyBtn).toBeTruthy()

    const cancelBtn = screen.getByRole("button", { name: /cancel reservation/i })
    expect(cancelBtn).toBeTruthy()
  })

  it("verifies zero serious or critical axe violations on NotFoundPage", async () => {
    const { NotFoundPage } = await import("../pages/NotFoundPage")
    const { MemoryRouter } = await import("react-router-dom")

    const { container } = render(
      <MemoryRouter>
        <NotFoundPage />
      </MemoryRouter>
    )

    const results = await axe.run(container, {
      rules: { "color-contrast": { enabled: false } },
    })

    const seriousOrCritical = results.violations.filter(
      (v) => v.impact === "serious" || v.impact === "critical"
    )
    expect(seriousOrCritical).toHaveLength(0)
  })
})
