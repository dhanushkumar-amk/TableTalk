# TableTalk — 3-Minute Local Demo Video Runbook

This runbook provides an exact, second-by-second click-and-say script for recording a 3-minute demo video of TableTalk using Groq LLM tool orchestration (`llama-3.3-70b-versatile`) and the interactive React frontend cards.

---

## Pre-Recording Checklist

1. **Environment Setup**:
   - Ensure `.env` contains:
     ```bash
     LLM_PROVIDER=groq
     GROQ_API_KEY=gsk_your_api_key_here
     GROQ_MODEL=llama-3.3-70b-versatile
     ```
   - If recording offline without an external API key, the built-in deterministic fallback engine will handle all 5 turns automatically (`fallback_mode: true` with `"Demo mode"` badge).
2. **Screen & Privacy Protection**:
   - Keep `.env` and terminal windows displaying environment variables off screen during recording.
   - Do NOT display real personal phone numbers, addresses, or actual payment credentials on screen.
   - Rotate any demo Groq or test API keys immediately after the video recording concludes.
3. **Start Services**:
   ```bash
   # Terminal 1: Backend
   cargo run --manifest-path backend/Cargo.toml --bin mcp_tabletalk_backend

   # Terminal 2: Frontend
   cd frontend && npm run dev
   ```
   - Backend runs on `http://127.0.0.1:8080`
   - Frontend runs on `http://127.0.0.1:5173/assistant`
4. **Browser Setup**:
   - Open Chrome or Edge to `http://127.0.0.1:5173/assistant` (ensure microphone permissions are granted).
   - Set zoom level to 100% or 110% for crisp UI readability.
   - Confirm audio input/output devices are tested.

---

## 3-Minute Video Script (Total: 180s)

```
+--------------------------------------------------------------------------------------------------+
| TIMELINE OVERVIEW                                                                                |
| 0:00 - 0:30 | Hook: Voice Negotiation on Fully Booked Slot & Ranked Alternatives                 |
| 0:30 - 1:00 | Table Confirmation & Interactive Booking Card                                     |
| 1:00 - 1:45 | Allergy-Aware Menu Search (Vegan & Peanut Exclusion with Transparent Disclosures)  |
| 1:45 - 2:25 | Pickup Order Placement & Live Prep Tracking                                       |
| 2:25 - 2:50 | Privacy & "Forget Me" Hard Memory Scrubbing                                        |
| 2:50 - 3:00 | Architecture & Wrap-Up (Groq + Axum Rust MCP Server)                              |
+--------------------------------------------------------------------------------------------------+
```

---

### [0:00 – 0:30] Act 1: The Hook — Fully Booked Conflict & Ranked Alternatives (30s)

*Goal: Hook the viewer immediately with a realistic dining conflict handled smoothly through voice and visual cards.*

- **Action**:
  - Open `/assistant`. The clean greeting "How can I help you dine today?" is visible with 4 suggestion chips.
  - Click the **Microphone** button (or click the suggestion chip `"Table for 4 Friday at 7:30"`).
- **Spoken Voice (User)**:
  > *"Hey, can I get a table for 4 this Friday at 7:30 PM?"*
- **Visuals on Screen**:
  - Mic pulses red with animated waveform ring (`Listening...`).
  - Assistant enters thinking state (`Thinking...`).
  - Tool activity chip appears: `[✨ Checking availability...]`.
  - Assistant responds with natural speech:
    > *"That exact slot is booked, but I found three ranked alternatives nearby. I can hold any of these for 5 minutes."*
  - The **NegotiationOptionsCard** renders dynamically below the assistant bubble showing:
    - Ranked option 1: 7:00 PM Patio (Score: 92)
    - Ranked option 2: 8:00 PM Indoor (Score: 88)
    - Ranked option 3: 8:15 PM Patio (Score: 78)
    - Action buttons: `[Hold 5 min]`, `[Book 8:00 PM Indoor]`

---

### [0:30 – 1:00] Act 2: Table Confirmation & Booking Card (30s)

*Goal: Demonstrate card-emitted events turning into chat turns and rendering confirmed booking data.*

- **Action**:
  - Click `[Book 8:00 PM Indoor]` on the card (or speak into the mic: *"Let's take the 8:00 PM indoor table for Aarav Sharma"*).
- **Spoken Voice (User)**:
  > *"Let's take the 8:00 PM indoor table under Aarav Sharma."*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Reserving table...]`.
  - Assistant replies aloud:
    > *"You're all set! I've booked Table T03 for 4 guests on Friday at 8:00 PM under Aarav Sharma. Your confirmation code is TT-784920."*
  - The **BookingConfirmationCard** appears in real time:
    - Confirmed green badge `CONFIRMED`.
    - Date & Time: `Friday, 8:00 PM – 9:30 PM (Indoor Table T03)`.
    - Monospace confirmation code block with copy button `[Copy TT-784920]`.
    - Cancellation policy notice (`Free cancellation up to 6 hours prior`).

---

### [1:00 – 1:45] Act 3: Allergy-Aware Menu Search (45s)

*Goal: Highlight uncompromising allergen safety — zero false safety promises and transparent ingredient disclosures.*

- **Action**:
  - Click the microphone button or type into the auto-growing composer:
- **Spoken Voice (User)**:
  > *"What vegan options do you have that don't contain peanuts?"*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Searching menu...]`.
  - Assistant voice response:
    > *"Here are our vegan dishes prepared without peanut ingredients. We note cross-contact risk for all items since peanuts are used in our kitchen."*
  - The **MenuCarousel** card animates in:
    - **Tandoori Roti** (₹60) — Vegan chip, Gluten disclosure.
    - **Dal Tadka** (₹280) — Vegan chip, Garlic/Onion disclosure.
    - **Yellow Lentil Curry** (₹310) — Vegan chip, Verified allergen row.
  - Click **Arrow Right** or drag carousel to show smooth keyboard accessibility and swipe navigation.
  - Notice the permanent disclaimer in the card: *"Cross-contact risk is actively monitored; no item is guaranteed 100% allergen-free."*

---

### [1:45 – 2:25] Act 4: Pickup Order Placement (40s)

*Goal: Show conversational ordering with price totals and live prep tracking without client payment execution.*

- **Action**:
  - Speak into the mic or click the suggestion chip `"Order pickup"`:
- **Spoken Voice (User)**:
  > *"I'd like to place a pickup order for 2 Dal Tadka and 4 Tandoori Roti under Aarav."*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Placing pickup order...]`.
  - Assistant voice response:
    > *"Your pickup order has been placed for Aarav. Estimated prep time is 25 minutes. Total is eight hundred and forty rupees, payable at pickup."*
  - The **OrderSummaryCard** renders:
    - Order Code: `ORD-92841` with one-tap copy button.
    - Item breakdown with unit prices, quantity multipliers, and line totals.
    - Subtotal, Tax, and Grand Total (`₹840.00`).
    - Status badge: `Placed — Ready in 25 mins`.
    - `Pay at Pickup` confirmed (cards never process credit card transactions directly).

---

### [2:25 – 2:50] Act 5: Privacy & "Forget Me" Data Erasure (25s)

*Goal: Demonstrate server-side profile token protection and hard PII scrubbing with confirmation modal.*

- **Action**:
  - Type or say: *"Please forget all my data and preferences."*
- **Spoken Voice (User)**:
  > *"Please forget me and erase my profile."*
- **Visuals on Screen**:
  - Destructive action confirmation modal appears:
    - Title: `Erase Guest Profile?`
    - Description: *"This will permanently delete your stored preferences, profile token, and personal notes from the server database."*
  - Click the red button `[Forget Me]`.
  - Tool activity chip: `[✨ Erasing guest profile...]`.
  - Assistant confirms:
    > *"All saved preferences, contact information, and personal data have been permanently erased from our systems."*
  - The **StatusCard** displays a blue privacy shield badge: `PRIVACY: DATA ERASED`.

---

### [2:50 – 3:00] Act 6: Architecture & Closing (10s)

*Goal: Concise engineering sign-off highlighting the dual-provider architecture.*

- **Action**:
  - Briefly click the top right `[Live Activity]` button to show the full stream of executed MCP tool calls and latencies (12ms - 45ms).
- **Spoken Voice (Narrator)**:
  > *"TableTalk combines high-speed Rust Axum MCP tools with Groq Llama 3.3 orchestration and accessible React cards for a fast, reliable, voice-first dining assistant. Thank you!"*

---

## Troubleshooting & Demo Fallback Tips

1. **Groq Rate Limit (HTTP 429) or Network Outage**:
   - The backend automatically catches 429 or network errors, engages exponential backoff, and switches to the embedded rule-based fallback router (`fallback_mode: true`).
   - The UI displays an amber `"Demo mode"` badge in the header, and all flows (availability check, booking, menu search, pickup orders) continue to respond deterministically.
2. **Browser Microphone Permissions**:
   - If the browser blocks microphone access, click the Composer textarea to type queries. The mic button displays an informational toast indicating speech recognition fallback.
3. **Audio Narration Mute**:
   - Click the Speaker icon in the top header at any time to toggle voice synthesis on/off during recording.
