# TableTalk Cards and UI Design Specification

**Phase**: Phase 13 of 20  
**Status**: APPROVED & IMPLEMENTED  
**Protocol Version**: MCP `2025-11-25`  
**Transport**: Streamable HTTP  
**Date**: 2026-10-14  

---

## 1. Research Findings: MCP Apps, UI Extensions, & Host Support

### 1.1 Primary Sources & Spec Analysis
Research was conducted using primary documentation on 2026-10-14:

1. **MCP Specification Index & Extensions**:
   - URL: `https://modelcontextprotocol.io/specification` and `https://github.com/modelcontextprotocol/ext-apps` (SEP-1865)
   - Current Spec Status: **DRAFT / EXPERIMENTAL EXTENSION**. The core MCP specification (version `2025-11-25`) defines `tools`, `resources`, `prompts`, and `structuredContent`. It does *not* standardize an interactive UI iframe execution sandbox in the core standard. The proposed `ext-apps` extension defines `ui://` URI schemes, `_meta.ui.resourceUri` linkage, CSP headers, and bidirectional host-to-iframe JSON-RPC messaging, but remains an unstable vendor draft.

2. **Rust MCP SDK (`rmcp 3.5.1`)**:
   - URL: `https://crates.io/crates/rmcp` and `https://github.com/modelcontextprotocol/rust-sdk`
   - Current SDK Capabilities: Supports MCP `2025-11-25` protocols, resources (`ListResourcesResult`, `ReadResourceResult`), and the `_meta` key in `CallToolResult`. However, `rmcp` provides *no* application packaging, single-file HTML bundler, asset inliner, or sandbox execution runtime for `ext-apps`.

3. **Amazon Alexa+ Developer Documentation & Echo Show Runtimes**:
   - URL: Amazon Developer Hackathon Portal & Public Alexa+ documentation
   - Echo Show / Alexa+ Rendering Status: **UNVERIFIED**. Public developer materials confirm that Alexa+ acts as an MCP client invoking tools over Streamable HTTP to gather spoken responses and structured JSON. Public documentation does *not* document support for executing third-party `ui://` iframes, MCP Apps, or interactive arbitrary HTML on Echo Show devices. Claiming Alexa+ renders third-party MCP Apps would be unsupported.

4. **Common Host Ecosystem**:
   - Standard MCP hosts (MCP Inspector, Claude Desktop, Cursor, Goose, VS Code MCP extensions) render standard `structuredContent` and text blocks. None have standardized native support for arbitrary `ui://` interactive app execution without dedicated client-side plugins.

---

## 2. Architectural Decision: Option B

### Decision
**Option B**: Skip server-side UI resources. Ship the standalone, accessible React renderer driven by `structuredContent` (with optional namespaced `_meta` display hints). Document the MCP Apps path as future work.

### Justification
1. **Spec Stability & Safety**: The `ext-apps` standard is draft and experimental. Embedding bundled HTML assets into the Rust binary adds fragile compilation overhead and risks CSP/sandbox vulnerabilities.
2. **Ecosystem Fidelity**: `structuredContent` is the official, stable, contractually guaranteed MCP mechanism for returning rich, typed data from tools.
3. **No Breaking Changes**: Keeps all 24 existing tools byte-for-byte identical in text and `structuredContent` output for all existing clients.
4. **Clean Decoupling**: The React frontend (`frontend/src/cards/`) consumes validated `structuredContent` directly, ensuring accessibility (WCAG 2.2 AA), full keyboard control, and safety-critical sanitization without iframe sandbox boundaries.

---

## 3. Card Catalog & Registry

The frontend registry maps `(tool_name, status)` pairs to specialized card components. Any unknown status or unexpected payload falls back safely to `FallbackCard`.

| Tool Name | Server Statuses | Rendered Component | Key Visual Elements |
|---|---|---|---|
| `search_menu` | `found`, `no_matches` | `MenuCarousel` + `MenuItemCard` | Horizontal snap carousel, image/placeholder, name, price, dietary chips, spice level, allergen notice |
| `get_usual_order` | `found` | `OrderSummaryCard` (with menu line items) | Order lines, pricing math (subtotal, tax, total), dietary tags, allergen notice |
| `get_usual_order` | `no_history`, `not_found` | `StatusCard` | Neutral info card explaining no order history is available |
| `check_availability` | `available`, `unavailable`, `closed`, `invalid_request` | `AvailabilityCard` | Snapshot disclaimer, slot allocation, operating hours, alternative slot chips |
| `negotiate_booking` | `options_found`, `no_viable_options` | `NegotiationOptionsCard` | Ranked options, human-friendly reason codes, hold countdown timer, Accept / Hold actions |
| `create_booking` | `confirmed` | `BookingConfirmationCard` | Monospace confirmation code with copy button, slot time, party, area, cancellation policy |
| `create_booking` | `unavailable`, `closed`, `limit_exceeded`, `idempotency_conflict` | `StatusCard` / `AvailabilityCard` | Clear error or alternatives, next action recommendation |
| `get_booking` | `found` | `BookingConfirmationCard` | Monospace confirmation code with copy button, guest name, party, area, status badge |
| `get_booking` | `not_found` | `StatusCard` | Verification failure or missing booking notice |
| `cancel_booking` | `cancelled`, `already_cancelled` | `StatusCard` | Confirmation of cancellation, policy note, cancellation timestamp |
| `cancel_booking` | `not_found`, `not_cancellable` | `StatusCard` | Anti-enumeration safe message or cancellation deadline passed |
| `place_order` | `placed` | `OrderSummaryCard` | Monospace order code, pickup time, line items, pricing breakdown, pay at pickup badge |
| `place_order` | `allergen_conflict`, `item_unavailable`, etc. | `StatusCard` | Conflict explanation, blocked allergens, suggested replacements |
| `get_order` | `found` | `OrderSummaryCard` | Line items, subtotal, tax, total, pickup time, status |
| `get_order` | `not_found` | `StatusCard` | Generic not-found message |
| `cancel_order` | `cancelled`, `already_cancelled`, `not_cancellable` | `StatusCard` | Status outcome, policy note |
| `get_deposit_status` | `open`, `success`, `declined`, `expired`, `refunded` | `PaymentCard` | "SIMULATED PAYMENT" banner, amount, countdown timer, mock checkout button |
| `refresh_payment_session` | `refreshed` | `PaymentCard` | New payment link, renewed countdown |
| `accept_offer` | `confirmed` | `BookingConfirmationCard` | Confirmed booking code, table details |
| `accept_offer` | `deposit_required` | `PaymentCard` | Deposit amount, hold expiration, checkout button |
| `accept_offer` | `hold_expired`, `already_booked` | `StatusCard` | Hold expiration notice |
| `release_offer` | `released`, `not_found` | `StatusCard` | Offer release confirmation |
| `join_waitlist` | `joined`, `duplicate`, `limit_exceeded` | `WaitlistCard` | Approximate position, requested slot, check status action |
| `get_waitlist_status` | `waiting`, `offered`, `expired`, `cancelled` | `WaitlistCard` | Status badge, approximate position, offer details with countdown timer |
| `leave_waitlist` | `left`, `already_left` | `StatusCard` | Waitlist cancellation confirmation |
| `get_profile` | `found` | `ProfileCard` | Stored preferences, escaped plain-text user note, no token display |
| `enable_memory` | `enabled` | `ProfileCard` | Confirmation of opt-in, masked note, memory status |
| `remember_preference` | `remembered`, `duplicate` | `StatusCard` | Stored preference confirmation |
| `forget_preference` | `forgotten`, `confirmation_required` | `StatusCard` | Preference removal confirmation |
| `forget_me` | `forgotten` | `StatusCard` | Complete erasure notice |
| `ping_restaurant` | *all* | `StatusCard` | System health and connectivity |
| `rotate_profile_token` | *all* | `StatusCard` | Token rotation status (token itself never rendered in DOM) |
| *Fallback* | *any unknown / invalid* | `FallbackCard` | Spoken text caption, safe notification, zero raw JSON |

---

## 4. Reason Code to Human Phrase Mapping

Negotiation and availability reason codes are translated to user-friendly phrases:

| Server Reason Code | User-Facing Phrase | Description / Context |
|---|---|---|
| `requested_exact_match` | Exact match for your request | The slot matches requested time and area perfectly |
| `closest_time_match` | Nearest available time | Closest slot to requested time |
| `adjacent_time_earlier` | Earlier dining time | Available slot 15–45 minutes earlier |
| `adjacent_time_later` | Later dining time | Available slot 15–45 minutes later |
| `alternative_area` | Different seating area | Available in patio or bar instead of indoor |
| `alternative_day` | Alternative date | Available on the next open day |
| `larger_table_allocated` | Generous seating | Slightly larger table assigned to fit party |
| `split_party_recommendation` | Large party accommodation | Requires banquet assistance or multiple tables |
| `fully_booked` | Fully booked at this time | All tables in the requested area are reserved |
| `closed_day` | Restaurant is closed on this day | Regular weekly closure |
| `special_closure` | Closed for special event | Scheduled holiday or maintenance |
| `outside_hours` | Outside service hours | Time is outside regular lunch/dinner shifts |
| `too_soon` | Requires more advance notice | Minimum 30-minute booking lead time |
| `beyond_horizon` | Outside 30-day booking window | Reservations open 30 days in advance |
| `party_too_large` | Exceeds online party size | Parties over 12 require direct contact |
| `no_single_table` | No single table large enough | Party exceeds physical table capacity (8) |
| `active_bookings_limit` | Maximum active reservations reached | Limit of 3 active bookings per guest |
| `deposit_required` | Table deposit required | Large parties or peak hours require deposit |
| `hold_expired` | Hold expired | The 15-minute table hold period has elapsed |

---

## 5. Event Contract

Cards do **not** invoke MCP tools directly. Instead, they emit strongly typed UI events to a host listener (to be connected in Phase 17).

| Event Name | Payload Type | Description |
|---|---|---|
| `onAcceptOption` | `{ optionId: string; holdId?: string }` | Diner clicked Accept on a negotiated option |
| `onHoldOption` | `{ optionId: string }` | Diner requested a 15-minute hold on an option |
| `onSelectAlternative` | `{ date: string; time: string; area: string }` | Diner clicked an alternative slot chip |
| `onCancelBooking` | `{ confirmationCode: string; guestName: string }` | Diner confirmed cancellation of a reservation |
| `onCancelOrder` | `{ orderCode: string; guestName: string }` | Diner confirmed cancellation of a pickup order |
| `onOpenCheckout` | `{ checkoutUrl: string }` | Diner clicked to pay deposit (URL used for navigation, not displayed) |
| `onCheckStatus` | `{ type: "waitlist" \| "deposit" \| "booking"; id: string }` | Diner requested refreshed status |
| `onLeaveWaitlist` | `{ waitlistId: string }` | Diner confirmed leaving the waitlist |
| `onCopyCode` | `{ code: string; label: string }` | Diner copied confirmation code or order code |

### Destructive Actions & Confirm Steps
Destructive actions (`onCancelBooking`, `onCancelOrder`, `onLeaveWaitlist`) require a two-step confirmation inside the card before firing the event callback.

---

## 6. Safety-Critical Design Rules

1. **Allergen Presentation**:
   - The card must display the verbatim `allergen_notice` provided by the server.
   - Wording is strictly restricted to "Listed as containing: ..." and "Listed as not containing: ...".
   - **PROHIBITED TERMS**: Never use the words **"safe"**, **"guaranteed"**, or **"allergy-free"** anywhere in the UI.
   - **NO GREEN CHECKMARKS**: Allergen badges use neutral amber, slate, or outline styles. Never use green icons that imply safety.
   - Unverified allergen information must be explicitly labeled: `"Allergen information unverified"`.

2. **Simulated Payment Presentation**:
   - Every payment card must display a prominent badge: `"SIMULATED PAYMENT — no real money"`.
   - Never render card input forms, CVV fields, or card number inputs.
   - The `checkout_url` is passed as a link target or event parameter, but is **never rendered as visible text** on screen.

3. **Privacy & Sensitive Tokens**:
   - Profile bearer tokens (`profile_token`) are strictly kept in memory. They are **never rendered in the DOM**, never logged to the browser console, and never saved in `localStorage` or `sessionStorage`.
   - Stored user notes (`user_provided_note`) are rendered as plain escaped text inside a dedicated `"Your note"` block. They are never interpreted as markdown or HTML.
   - Internal database IDs (e.g., `id`, `table_id`, `guest_id`) are stripped and never rendered. Only guest-facing alphanumeric confirmation codes (`TT-XXXXX`) are displayed.

---

## 7. Design System & Accessibility Tokens

### Design System Tokens
- **Font Stack**: `Geist Variable`, sans-serif; `JetBrains Mono` / `ui-monospace` for confirmation codes.
- **Color Palette (OKLCH)**:
  - Background: `oklch(1 0 0)` (light), `oklch(0.145 0 0)` (dark)
  - Card Surface: `oklch(1 0 0)` (light), `oklch(0.205 0 0)` (dark)
  - Primary Text: `oklch(0.145 0 0)` (light), `oklch(0.985 0 0)` (dark)
  - Accent / Primary: `oklch(0.205 0 0)` (light), `oklch(0.922 0 0)` (dark)
  - Warning / Simulated: `oklch(0.7 0.18 50)` (Amber)
  - Destructive: `oklch(0.577 0.245 27.325)` (Red)
- **Border Radius**: Base radius `0.625rem` (`10px`), card radius `1rem` (`16px`).
- **Touch Target**: All interactive controls are at least `44px × 44px`.

### Accessibility Checklist (WCAG 2.2 AA)
- [x] Semantic HTML (`<article>`, `<header>`, `<main>`, `<button>`).
- [x] Contrast ratio exceeds 4.5:1 for normal text and 3:1 for large text/icons.
- [x] Keyboard navigable: full arrow key and tab navigation for carousels and actions.
- [x] Color is never the sole indicator: status badges include text and distinct icons.
- [x] `prefers-reduced-motion` suppresses animations and transitions.
- [x] Live regions (`aria-live="polite"`) announce countdown milestones (at 10m, 5m, 2m, 1m, 0m), not every second.

---

## 8. Known Limitations
1. **Alexa+ Echo Show Rendering**: Status is **UNVERIFIED** in public Amazon developer documentation. The UI cards currently run in our React web simulator.
2. **Profile Tokens**: Maintained in component memory only during simulation.
3. **Simulated Payments**: Only mock payment outcomes are supported; no real payment gateway is integrated.
4. **Local Images**: Uses generated local SVG and CSS gradient placeholders. No remote or copyrighted assets are fetched.
