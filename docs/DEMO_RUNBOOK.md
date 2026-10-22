# TableTalk — 3-Minute Local Demo Video Runbook

This runbook provides the exact, second-by-second click-and-say script for recording a 3-minute demo video of TableTalk using the deterministic **Scripted Demo Router** (`Scripted demo` badge) and the interactive React frontend cards.

> **DEMO MODE DECLARATION**: This demo video operates via TableTalk's deterministic **Scripted Demo Router**, not an external live LLM. Underneath, **every scenario calls the real, underlying Rust MCP tools** (`check_availability`, `create_booking`, `search_menu`, `place_order`, `join_waitlist`, `forget_me`, etc.) against SQLite with full database transaction integrity and live card UI generation.

---

## Pre-Recording Checklist

1. **Start Services**:
   ```bash
   # Terminal 1: Backend Server (Must be running on port 8080)
   cargo run --bin mcp_tabletalk_backend

   # Terminal 2: Frontend Dev Server (Port 5173)
   cd frontend && npm run dev
   ```
2. **Verify Backend Connection**:
   - Run `bash scripts/demo-transcript.sh` to confirm all 15 scenarios respond with real MCP tools.
3. **Browser Setup**:
   - Open Chrome or Edge to `http://127.0.0.1:5173/assistant`.
   - Verify the top badge displays `"Scripted demo"` with the sparkles icon.
   - Set zoom level to 100% or 110% for crisp card readability.

---

## 3-Minute Video Script (Total: 180s)

```
+--------------------------------------------------------------------------------------------------+
| TIMELINE OVERVIEW                                                                                |
| 0:00 - 1:00 | Act 1: THE HERO FLOW — Conflict Negotiation, Hold, & Confirmed Booking (60s)       |
| 1:00 - 1:35 | Act 2: Allergy-Aware Menu Search (Never "Safe" + Staff Confirmation) (35s)         |
| 1:35 - 2:10 | Act 3: Pickup Order & Stated Allergen Conflict Warning (35s)                       |
| 2:10 - 2:35 | Act 4: Large Party Deposit (Simulated Payment Banner) & Waitlist (25s)             |
| 2:35 - 2:55 | Act 5: Consent-Driven Memory & Hard "Forget Me" Data Erasure (20s)                 |
| 2:55 - 3:00 | Act 6: Architecture Sign-Off & Closing (5s)                                        |
+--------------------------------------------------------------------------------------------------+
```

---

### [0:00 – 1:00] Act 1: THE HERO FLOW (Multi-Turn Table Negotiation) (60s)

*Goal: Hook the viewer immediately with a real dining capacity conflict handled smoothly through spoken turns and interactive UI cards.*

#### Turn 1: Check Availability (Fully Booked Conflict)
- **Spoken Voice (User)**:
  > *"Table for 4 Friday at 7:30"*
- **Tool Executed**: `check_availability`
- **Assistant Spoken Response**:
  > *"I don't have indoor seating at seven thirty in the evening, but patio is free at seven thirty in the evening, or I have nine in indoor. Would either work for you?"*
- **Visuals on Screen**:
  - Tool activity chip appears: `[✨ Checking availability...]`.
  - The **Table Availability Card** renders with status `UNAVAILABLE` and shows ranked alternative slots (Patio 7:30 PM, Indoor 9:00 PM).
  - The "Try saying" chip row above the composer updates to: `Try saying: "the patio one"`.

#### Turn 2: Pick Alternative
- **Spoken Voice (User)**:
  > *"The patio one"* (or click the `"the patio one"` chip)
- **Tool Executed**: None (Resolved from session state)
- **Assistant Spoken Response**:
  > *"I have selected the Patio table at 19:30 for 4 guests. Would you like me to hold this table for 5 minutes, or confirm the booking?"*
- **Visuals on Screen**:
  - The "Try saying" chip row advances to: `Try saying: "hold it"`.

#### Turn 3: Temporary Hold
- **Spoken Voice (User)**:
  > *"Hold it"*
- **Tool Executed**: None (Session state hold)
- **Assistant Spoken Response**:
  > *"I've placed a temporary hold on the patio table at 7:30 PM for 5 minutes. Would you like to confirm the booking under your name?"*
- **Visuals on Screen**:
  - The "Try saying" chip row advances to: `Try saying: "yes book it"`.

#### Turn 4: Initiate Booking
- **Spoken Voice (User)**:
  > *"Yes book it"*
- **Tool Executed**: None (Prompts for missing name)
- **Assistant Spoken Response**:
  > *"May I have your name to complete the reservation?"*
- **Visuals on Screen**:
  - The "Try saying" chip row advances to: `Try saying: "my name is Ravi"`.

#### Turn 5: Confirm Booking
- **Spoken Voice (User)**:
  > *"My name is Ravi"*
- **Tool Executed**: `create_booking` (Real database booking created!)
- **Assistant Spoken Response**:
  > *"Your reservation is confirmed! Confirmation code is T T hyphen [CODE] under Ravi."*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Reserving table...]`.
  - The **Booking Confirmation Card** appears with green badge `CONFIRMED`, table `P02`, and the spaced confirmation code.

---

### [1:00 – 1:35] Act 2: Allergy-Aware Menu Search (35s)

*Goal: Highlight uncompromising allergen safety — zero false safety promises and transparent ingredient disclosures.*

- **Spoken Voice (User)**:
  > *"I'm allergic to peanuts, what can I eat"*
- **Tool Executed**: `search_menu`
- **Assistant Spoken Response**:
  > *"These dishes are listed as not containing peanuts. Please confirm with our staff before dining if you have severe sensitivities."*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Searching menu...]`.
  - The **Menu Carousel Card** displays peanut-free items (e.g. *Tandoori Roti*, *Crisp Palak Chaat*).
  - Point out that the assistant **never** claims food is "safe" or "guaranteed", always stating "listed as not containing peanuts" and directing guests to verify with staff.

---

### [1:35 – 2:10] Act 3: Pickup Order & Allergen Conflict (35s)

*Goal: Demonstrate pickup order placement, item calculation, and active allergen conflict blocking.*

#### Part A: Allergen Conflict Warning
- **Spoken Voice (User)**:
  > *"Order the satay, I'm allergic to peanuts"*
- **Tool Executed**: `place_order` (detects allergen conflict)
- **Assistant Spoken Response**:
  > *"The Banarasi Baingan Bharta is listed as containing peanuts, which you asked to avoid. Would you like to try the Crisp Palak Chaat instead?"*
- **Visuals on Screen**:
  - The assistant refuses to silently place an unsafe dish, transparently naming the conflict and offering an alternative.

#### Part B: Successful Takeout Placement
- **Spoken Voice (User)**:
  > *"Order paneer tikka and two garlic naan for pickup at 8, name Ravi"*
- **Tool Executed**: `place_order`
- **Assistant Spoken Response**:
  > *"I've placed your pickup order for 3 items totaling eight hundred eighty-two rupees, ready at eight in the evening. Your order code is [CODE]. Pay at pickup."*
- **Visuals on Screen**:
  - Tool activity chip: `[✨ Placing pickup order...]`.
  - The **Pickup Order Card** displays item quantities, prices in integer rupees, and pickup time.

---

### [2:10 – 2:35] Act 4: Large Party Deposit & Waitlist (25s)

*Goal: Show high-capacity deposit rules (The Human-Confirms Payment Rule) and waitlist queuing.*

#### Part A: Large Party Deposit
- **Spoken Voice (User)**:
  > *"Table for 8 Saturday at 8"*
- **Tool Executed**: `create_booking` (returns `deposit_required`)
- **Assistant Spoken Response**:
  > *"For parties of 8 on Saturday at 8:00 PM, a refundable deposit of 2000 rupees is required to confirm your reservation. Please use the simulated checkout card below to complete payment. I cannot process payments directly."*
- **Visuals on Screen**:
  - The **Payment Required Card** appears with the prominent `SIMULATED` banner and a button pointing to mock checkout. Assistant never approves payments directly.

#### Part B: Waitlist
- **Spoken Voice (User)**:
  > *"Put me on the waitlist"*
- **Tool Executed**: `join_waitlist`
- **Assistant Spoken Response**:
  > *"You're on the waitlist at position 1 for Friday, October 23 between 7:00 pm and 8:30 pm. Your code is W L - [CODE]."*
- **Visuals on Screen**:
  - The **Waitlist Card** shows queue position 1 and estimated wait time.

---

### [2:35 – 2:55] Act 5: Consent Memory & "Forget Me" Hard Erasure (20s)

*Goal: Prove privacy-first guest memory: explicit opt-in and permanent data deletion.*

#### Part A: Explicit Consent
- **Spoken Voice (User)**:
  > *"Remember I'm allergic to peanuts"*
- **Assistant**: *"Would you like me to remember your peanut allergy for future visits and recommendations? Please say yes to confirm."*
- **User**: *"Yes"*
- **Tool Executed**: `remember_preference` (Server stores hash; profile token never logged in plaintext)

#### Part B: Destructive "Forget Me" Confirmation
- **Spoken Voice (User)**:
  > *"Forget me"*
- **Assistant**: *"Are you sure you want me to forget all your saved preferences and profile data? Please say or type yes to confirm."*
- **User**: *"Yes"*
- **Tool Executed**: `forget_me`
- **Assistant**:
  > *"I have completely removed your profile and preferences. Your historical orders and bookings have been anonymized for business records."*

---

### [2:55 – 3:00] Act 6: Architecture Sign-Off & Closing (5s)

- **Action**:
  - Click the **Live Activity** drawer at top right to reveal the full audit trail of executed MCP tools.
- **Spoken Voice (Narrator)**:
  > *"TableTalk combines deterministic Rust Axum MCP tools with rich React cards for an uncompromising, voice-first dining assistant. Thank you!"*

---

## What to Say If a Reply Is Off (Recovery Guidance)

1. **If Speech Recognition Mishears a Word**:
   - Click the **"Try saying"** chip pill directly above the input bar! It immediately dispatches the exact canonical phrase into the chat.
2. **If You Want to Reset the Conversation**:
   - Refresh the page (`F5`) or say *"Hi"* to restart cleanly.
3. **If You Want to Cancel an Action**:
   - Say *"No"* or *"Never mind"* during any confirmation turn (`cancel booking`, `forget me`). The assistant will reply: *"Understood, I've cancelled that action."*
