import { describe, it, expect } from "vitest"
import { render } from "@testing-library/react"
import { renderCard } from "../cards/registry"
import { GALLERY_FIXTURES } from "../dev/CardGallery"
import { AllergenNotice } from "../cards/components/AllergenNotice"
import { ProfileCard } from "../cards/components/ProfileCard"

describe("Safety-Critical Assertions: Allergen, Payment, and Privacy Integrity", () => {
  it("verifies NO card across all 33 fixtures contains prohibited safety words ('safe', 'guaranteed', 'allergy-free')", () => {
    const prohibitedRegex = /\b(safe|guaranteed|allergy-free|allergen-free)\b/i

    for (const fixture of GALLERY_FIXTURES) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )

      const textContent = container.textContent || ""
      const match = textContent.match(prohibitedRegex)
      expect(
        match,
        `Prohibited safety term "${match?.[0]}" found in card for fixture '${fixture.id}' (${fixture.toolName})!`
      ).toBeNull()
    }
  })

  it("verifies NO raw bearer profile token appears in the DOM across all rendered fixtures", () => {
    // Pattern for TableTalk bearer tokens: tt_prof_...
    const tokenRegex = /tt_prof_[a-zA-Z0-9_-]{10,}/i

    for (const fixture of GALLERY_FIXTURES) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )

      const textContent = container.textContent || ""
      expect(
        tokenRegex.test(textContent),
        `Bearer profile token leaked in DOM text for fixture '${fixture.id}'!`
      ).toBe(false)
    }
  })

  it("verifies checkout_url is NEVER displayed as visible text on any card", () => {
    // Pattern for checkout URLs
    const checkoutUrlRegex = /https?:\/\/[^\s]+\/checkout\/tt_pay_[a-zA-Z0-9_-]+/i

    for (const fixture of GALLERY_FIXTURES) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )

      const textContent = container.textContent || ""
      expect(
        checkoutUrlRegex.test(textContent),
        `Checkout URL leaked as visible text in fixture '${fixture.id}'!`
      ).toBe(false)
    }
  })

  it("verifies internal database UUIDs do NOT appear as visible text on cards", () => {
    // Standard RFC4122 UUID pattern
    const uuidRegex = /\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b/i

    for (const fixture of GALLERY_FIXTURES) {
      const { container } = render(
        renderCard({
          toolName: fixture.toolName,
          envelope: fixture.envelope,
        })
      )

      const textContent = container.textContent || ""
      expect(
        uuidRegex.test(textContent),
        `Internal database UUID leaked in card text for fixture '${fixture.id}'!`
      ).toBe(false)
    }
  })

  it("renders user-provided notes containing scripts or HTML as inert escaped text", () => {
    const maliciousNote =
      "Prefers booth <script>alert('xss')</script> **bold** <img src=x onerror=alert(1)>"

    const mockProfile = {
      status: "found",
      guest_name: "Security Tester",
      preferences: [
        {
          id: 99,
          kind: "note",
          value: maliciousNote,
          created_at: "2026-10-14T12:00:00Z",
        },
      ],
      consent_version: "v1.0",
      consent_granted_at: "2026-10-14T12:00:00Z",
      counts: { preference_count: 1, total_bookings: 0, total_orders: 0 },
      needs_reconfirmation: [],
    }

    const { container } = render(<ProfileCard profile={mockProfile as any} />)

    // The script tag must not exist as an HTML element in the DOM tree
    expect(container.querySelectorAll("script").length).toBe(0)
    expect(container.querySelectorAll("img").length).toBe(0)

    // The text content must contain the escaped text literally
    expect(container.textContent).toContain("<script>alert('xss')</script>")
  })

  it("explicitly labels unverified allergen data as 'Allergen information unverified'", () => {
    const { container } = render(
      <AllergenNotice
        notice="Standard operating advisory"
        isUnverified={true}
      />
    )

    expect(container.textContent).toContain("Allergen information unverified")
  })

  it("strictly phrases allergen presence as 'Listed as containing:' and absences as 'Listed as not containing:'", () => {
    const { container } = render(
      <AllergenNotice
        notice="Kitchen handles wheat and milk."
        containingAllergens={["dairy", "gluten"]}
        notContainingAllergens={["peanuts"]}
      />
    )

    expect(container.textContent).toContain("Listed as containing: dairy, gluten")
    expect(container.textContent).toContain("Listed as not containing: peanuts")
  })
})
