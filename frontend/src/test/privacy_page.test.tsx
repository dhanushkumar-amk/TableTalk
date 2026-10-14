import { describe, it, expect, vi, beforeEach } from "vitest"
import { render, screen, within } from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import { MemoryRouter } from "react-router-dom"
import { PrivacyPage } from "../pages/PrivacyPage"

describe("Privacy Page (/privacy)", () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it("renders page header and privacy controls with memory switch off by default", () => {
    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    expect(screen.getByRole("heading", { name: "Your data, your control" })).toBeTruthy()
    expect(screen.getByText("Dining Memory Switch")).toBeTruthy()
    expect(screen.getByRole("switch").getAttribute("aria-checked")).toBe("false")
    expect(screen.getByText("Demo project. Do not enter real personal data.")).toBeTruthy()
  })

  it("opens consent dialog requiring explicit checkbox to enable memory", async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    const switchBtn = screen.getByRole("switch")
    await user.click(switchBtn)

    // Consent modal should be visible
    const dialog = screen.getByRole("dialog", { name: /Explicit Memory Consent/i })
    expect(dialog).toBeTruthy()

    const confirmBtn = within(dialog).getByRole("button", { name: "Enable Memory" }) as HTMLButtonElement
    expect(confirmBtn.disabled).toBe(true)

    // Check the consent checkbox
    const checkbox = within(dialog).getByRole("checkbox")
    await user.click(checkbox)
    expect(confirmBtn.disabled).toBe(false)

    // Confirm consent
    await user.click(confirmBtn)
    expect(switchBtn.getAttribute("aria-checked")).toBe("true")
    expect(screen.getByText("Memory Opt-in Active")).toBeTruthy()
    expect(screen.getByRole("button", { name: /rotate token/i })).toBeTruthy()
  })

  it("rotates profile token without ever displaying raw token text on screen", async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    // Enable memory first
    await user.click(screen.getByRole("switch"))
    const dialog = screen.getByRole("dialog", { name: /Explicit Memory Consent/i })
    await user.click(within(dialog).getByRole("checkbox"))
    await user.click(within(dialog).getByRole("button", { name: "Enable Memory" }))

    const rotateBtn = screen.getByRole("button", { name: /rotate token/i })
    await user.click(rotateBtn)

    // Token text should never be rendered in DOM
    expect(screen.queryByText(/tk_/i)).toBeNull()
  })

  it("triggers critical safety reminder when removing an allergy", async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    // Enable memory
    await user.click(screen.getByRole("switch"))
    const dialog = screen.getByRole("dialog", { name: /Explicit Memory Consent/i })
    await user.click(within(dialog).getByRole("checkbox"))
    await user.click(within(dialog).getByRole("button", { name: "Enable Memory" }))

    // Remove allergy button
    const removePeanutsBtn = screen.getByRole("button", { name: "Remove allergy peanuts" })
    await user.click(removePeanutsBtn)

    // Allergy confirmation dialog should appear with warning
    const allergyDialog = screen.getByRole("dialog", { name: /Remove Allergy/i })
    expect(allergyDialog).toBeTruthy()
    expect(
      within(allergyDialog).getByText(/restaurant staff must still be verbally informed/i)
    ).toBeTruthy()

    // Confirm removal
    await user.click(within(allergyDialog).getByRole("button", { name: "Confirm Removal" }))
    expect(screen.queryByRole("button", { name: "Remove allergy peanuts" })).toBeNull()
  })

  it("triggers data export download as JSON", async () => {
    const user = userEvent.setup()
    const createObjectURLMock = vi.fn().mockReturnValue("blob:mock-url")
    const revokeObjectURLMock = vi.fn()
    window.URL.createObjectURL = createObjectURLMock
    window.URL.revokeObjectURL = revokeObjectURLMock

    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    const downloadBtn = screen.getByRole("button", { name: /download json/i })
    await user.click(downloadBtn)

    expect(createObjectURLMock).toHaveBeenCalled()
    expect(revokeObjectURLMock).toHaveBeenCalled()
  })

  it("requires typing DELETE in danger zone to permanently erase profile memory", async () => {
    const user = userEvent.setup()
    render(
      <MemoryRouter>
        <PrivacyPage />
      </MemoryRouter>
    )

    // Enable memory first
    await user.click(screen.getByRole("switch"))
    const dialog = screen.getByRole("dialog", { name: /Explicit Memory Consent/i })
    await user.click(within(dialog).getByRole("checkbox"))
    await user.click(within(dialog).getByRole("button", { name: "Enable Memory" }))

    // Click forget me button
    const forgetMeBtn = screen.getByRole("button", { name: "Delete All My Data" })
    await user.click(forgetMeBtn)

    const forgetDialog = screen.getByRole("dialog", { name: /Permanently Forget Me/i })
    expect(forgetDialog).toBeTruthy()

    const confirmPurgeBtn = within(forgetDialog).getByRole("button", {
      name: "Confirm Permanent Deletion",
    }) as HTMLButtonElement
    expect(confirmPurgeBtn.disabled).toBe(true)

    // Type partial or wrong text
    const input = within(forgetDialog).getByPlaceholderText("DELETE")
    await user.type(input, "NO")
    expect(confirmPurgeBtn.disabled).toBe(true)

    // Clear and type exact DELETE
    await user.clear(input)
    await user.type(input, "DELETE")
    expect(confirmPurgeBtn.disabled).toBe(false)

    await user.click(confirmPurgeBtn)

    // Memory should now be disabled and summary displayed
    expect(screen.getByText("Data Destruction Confirmed")).toBeTruthy()
    expect(screen.getByRole("switch").getAttribute("aria-checked")).toBe("false")
  })
})
