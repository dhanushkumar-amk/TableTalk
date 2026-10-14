# TableTalk MCP Tools Specification

**Protocol Specification**: MCP `2025-11-25`  
**Transport**: Streamable HTTP (`POST /mcp`)  
**Server Version**: `tabletalk-backend` v0.1.0  

This document serves as the canonical reference for all MCP tools exposed by the TableTalk restaurant server. Field names and schemas documented here are stable and contractually guaranteed across assistant callers, the React simulator frontend, and downstream execution tools.

---

## 1. Tool Index (All 24 Tools)

| Tool Name | Lifecycle Phase | Read-Only | Idempotent | Description |
|---|---|---|---|---|
| `ping_restaurant` | Phase 3 | Yes | Yes | Connectivity check and server metadata verification. |
| `search_menu` | Phase 6 | Yes | Yes | Search food items with dietary/allergen filters, profile memory integration, and spoken prices. |
| `get_menu_item` | Phase 6 | Yes | Yes | Retrieve individual food item details by item ID or name. |
| `check_availability` | Phase 7 | Yes | Yes | Check table availability, snap slot grid, and propose alternatives. |
| `create_booking` | Phase 8 | No | Yes (with key) | Transactionally create a table reservation with idempotency, race safety, and seating preference tie-breaking. |
| `cancel_booking` | Phase 8 | No (destructive) | Yes | Cancel an existing reservation with guest-name verification and anti-enumeration. |
| `get_booking` | Phase 8 | Yes | Yes | Read-only reservation summary lookup verified by guest name. |
| `place_order` | Phase 9 | No | Yes (with key) | Transactionally place a pickup order with integer pricing, non-bypassable profile allergen protection, and dynamic prep estimation. |
| `get_order` | Phase 9 | Yes | Yes | Read-only pickup order summary lookup verified by guest name. |
| `cancel_order` | Phase 9 | No (destructive) | Yes | Cancel an existing placed pickup order with guest-name verification and rate-limited anti-enumeration. |
| `enable_memory` | Phase 10 | No | No | Explicit opt-in creation of profile memory, issuing single-use profile token with consent recording. |
| `get_profile` | Phase 10 | Yes | Yes | Read-only profile overview and data access/export record with masked contact details. |
| `remember_preference` | Phase 10 | No | Yes | Deduplicated storage of typed preferences (allergy, dietary, seat_pref, favorite_item, note). |
| `forget_preference` | Phase 10 | No (destructive) | Yes | Targeted removal of stored preferences with explicit confirmation requirement for allergy removal. |
| `get_usual_order` | Phase 10 | Yes | Yes | Read-only deterministic calculation of usual order from history, re-validated against current menu. |
| `rotate_profile_token` | Phase 10 | No | No | Atomic issuance of new bearer profile token with instant revocation of previous token. |
| `forget_me` | Phase 10 | No (destructive) | Yes | Complete hard delete of profile and preferences with unlinking and scrubbing of historical business records. |
| `negotiate_booking` | Phase 11 | Yes | Yes | Multi-criteria slot exploration, scoring trade-offs across time, area, and dates. |
| `accept_offer` | Phase 11 | No | Yes | Confirm a held slot reservation with token verification and transaction rollback. |
| `release_offer` | Phase 11 | No (destructive) | Yes | Release a held slot reservation ahead of its automatic expiration. |
| `join_waitlist` | Phase 11 | No | Yes | Queue a party onto the FIFO waitlist when all matching tables are fully booked. |
| `get_waitlist_status` | Phase 11 | Yes | Yes | Read-only waitlist status lookup verified by guest name and waitlist code. |
| `leave_waitlist` | Phase 11 | No (destructive) | Yes | Remove a party from the waitlist prior to fulfillment. |
| `get_deposit_status` | Phase 12 | Yes | Yes | Read-only deposit payment requirement and ledger status lookup for a booking. |
| `refresh_payment_session` | Phase 12 | No | Yes | Regenerate an active mock checkout URL and token for bookings requiring deposits. |

---

## 1.5 Shared Validation & Error Architecture (Phase 14)

All 24 tools follow a unified validation and error architecture:

### 1. Unknown Extra Fields Policy
* **Ignore Policy**: All tools silently ignore unrecognized extra keys in arguments payloads.
* **Compatibility**: Ensures forward and backward compatibility when LLMs include auxiliary fields (e.g. `context`, `thought`, or optional attributes).

### 2. Input Sanitization
* **Whitespace**: Leading and trailing whitespace is stripped across all string fields.
* **Control Characters**: Null bytes (`\x00`), ASCII control characters (`\x01`..`\x1F`), and ANSI escape sequences are completely stripped before processing.
* **Dates & Times**: Dates require strict ISO-8601 `YYYY-MM-DD`. Times require strict 24-hour `HH:MM`.
* **String & Array Caps**: String fields are capped (names: 100, codes: 32, tokens: 128, notes/reasons: 500-1000). Array fields (items, preferences, allergens) are bounded between 1 and 20 elements.

### 3. Error Categories & Envelope Format
* **Tool Argument Errors (`isError: true`)**:
  When input arguments violate constraints or types, tools return an execution error envelope specifying the field and valid constraint:
  ```json
  {
    "content": [
      {
        "type": "text",
        "text": "Invalid argument for 'date': must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received '17-10-2026'."
      }
    ],
    "isError": true
  }
  ```
* **Business Outcomes (`isError: false`)**:
  Standard business events (e.g., table fully booked, restaurant closed, booking not found) are returned as valid tool execution results with a structured payload:
  ```json
  {
    "content": [
      {
        "type": "text",
        "text": "The restaurant is closed on Tuesdays. Would you like to check Wednesday instead?"
      }
    ],
    "structuredContent": {
      "status": "closed",
      "reason": "regular_closure"
    },
    "isError": false
  }
  ```
* **Internal Server Errors**:
  Internal database, transactional, or system failures return a generic message containing a unique UUID Request ID (`Request ID: <uuid>`). SQL queries, database drivers, file paths, and stack traces are **never** leaked to callers. The full context is logged securely on the server via `tracing::error!`.
* **Protocol Violations**:
  Transport and protocol errors (invalid JSON, unknown JSON-RPC methods, uninitialized sessions) return standard JSON-RPC protocol error structures.

---

## 2. Tool: `check_availability`

### 2.1 Purpose & Role in Agentic Workflow
`check_availability` is a **pure read-only** capacity calculation engine. It evaluates whether a party can be accommodated at a specific date, time, and dining area at "The Saffron Nebula Bistro" without holding or mutating reservation records.

If the requested slot is unavailable, the engine performs outward grid and multi-day searches to return up to 3 (or configured `alternatives`) nearest viable alternatives, including different dining areas at the same time.

> [!IMPORTANT]
> **Availability Snapshot Caveat**: Availability returned by this tool represents a real-time point-in-time calculation and does **NOT** constitute a table hold or reservation. Confirmed reservations are created exclusively via `book_table` (Phase 8).

### 2.2 Caller Responsibilities (LLM Prompting Guidance)
- **Relative Date Resolution**: The caller (voice assistant or agent) must resolve relative phrases like *"tomorrow"*, *"this Friday"*, or *"next Saturday"* to a concrete ISO-8601 calendar date (`YYYY-MM-DD`) using the restaurant's local timezone (`Asia/Kolkata`).
- **Echo Verification**: The tool output includes `restaurant_now` (current local timestamp in the restaurant timezone) and `opening_hours_that_day` so the caller can verify the time context.

### 2.3 Annotations
```json
{
  "readOnlyHint": true,
  "idempotentHint": true,
  "destructiveHint": false,
  "openWorldHint": false
}
```

### 2.4 Input Schema
Dialect: JSON Schema Draft 2020-12

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "date": {
      "type": "string",
      "description": "Local restaurant date in ISO format YYYY-MM-DD (e.g. '2026-10-17'). The caller must resolve relative phrases like 'tomorrow' or 'Friday' to a concrete date itself, using the restaurant timezone. The tool result includes the restaurant's current local date/time for verification."
    },
    "time": {
      "type": "string",
      "description": "Local 24-hour time in HH:MM format (e.g. '19:30' or '13:00'). Off-grid times will be reported and snapped to the nearest slot granularity (15 minutes)."
    },
    "party_size": {
      "type": "integer",
      "format": "int64",
      "description": "Party size (number of guests). Must be at least 1 and cannot exceed restaurant maximum capacity (12)."
    },
    "area": {
      "type": ["string", "null"],
      "enum": ["any", "indoor", "patio", "bar"],
      "description": "Seating area preference: 'indoor', 'patio', 'bar', or 'any'. Defaults to 'any'."
    },
    "duration_minutes": {
      "type": ["integer", "null"],
      "format": "int64",
      "description": "Estimated dining duration in minutes. Optional; if omitted, defaults based on party size (75m for 1-2 guests, 90m for 3-4 guests, 105m for 5-6 guests, 120m for 7+ guests). Must be between 30 and 240 minutes if specified."
    },
    "alternatives": {
      "type": ["integer", "null"],
      "format": "uint",
      "minimum": 0,
      "description": "Number of alternative slots to return if the requested time is unavailable (0-6, default 3)."
    }
  },
  "required": ["date", "time", "party_size"]
}
```

### 2.5 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "requested": {
      "type": "object",
      "properties": {
        "date": { "type": "string" },
        "time": { "type": "string" },
        "party_size": { "type": "integer", "format": "int64" },
        "area": { "type": "string" },
        "duration_minutes": { "type": "integer", "format": "int64" },
        "timezone": { "type": "string" }
      },
      "required": ["date", "time", "party_size", "area", "duration_minutes", "timezone"]
    },
    "restaurant_now": { "type": "string" },
    "status": {
      "type": "string",
      "enum": ["available", "unavailable", "closed", "invalid_request"]
    },
    "reason": {
      "type": ["string", "null"],
      "enum": [
        "closed_day",
        "special_closure",
        "outside_hours",
        "too_soon",
        "beyond_horizon",
        "party_too_large",
        "party_size_invalid",
        "no_single_table",
        "fully_booked",
        "dst_gap",
        null
      ]
    },
    "reason_detail": { "type": ["string", "null"] },
    "snapped_from": { "type": ["string", "null"] },
    "slot": {
      "type": ["object", "null"],
      "properties": {
        "start_local": { "type": "string" },
        "end_local": { "type": "string" },
        "table_label": { "type": "string" },
        "area": { "type": "string" },
        "capacity": { "type": "integer", "format": "int64" }
      },
      "required": ["start_local", "end_local", "table_label", "area", "capacity"]
    },
    "alternatives": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "start_local": { "type": "string" },
          "end_local": { "type": "string" },
          "area": { "type": "string" },
          "table_label": { "type": "string" },
          "minutes_from_requested": { "type": "integer", "format": "int64" },
          "different_day": { "type": "boolean" }
        },
        "required": ["start_local", "end_local", "area", "table_label", "minutes_from_requested", "different_day"]
      }
    },
    "opening_hours_that_day": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "open": { "type": "string" },
          "close": { "type": "string" }
        },
        "required": ["open", "close"]
      }
    },
    "note": { "type": "string" }
  },
  "required": [
    "requested",
    "restaurant_now",
    "status",
    "reason",
    "reason_detail",
    "snapped_from",
    "slot",
    "alternatives",
    "opening_hours_that_day",
    "note"
  ]
}
```

---

## 3. Reason Codes Reference Table

| Reason Code | Status | Meaning | Recovery / Voice Action |
|---|---|---|---|
| `null` | `available` | Table is available for the requested time. | Confirm date, time, party size and invite booking ("Want me to book it?"). |
| `fully_booked` | `unavailable` | All fitting tables are reserved during the requested interval. | Inspect `alternatives` array and present nearest options. |
| `too_soon` | `unavailable` | Reservation is in the past or within the minimum lead time (30 mins). | Advise diner of the earliest acceptable reservation time today. |
| `beyond_horizon` | `unavailable` | Requested date exceeds the booking window (30 days). | Advise diner of maximum reservation horizon date. |
| `party_too_large` | `unavailable` | Party size exceeds establishment maximum capacity (12). | Suggest splitting party or contacting banquet management. |
| `no_single_table` | `unavailable` | Party size exceeds the largest single physical table (8). | Explain table combining is unassisted and suggest phone booking. |
| `closed_day` | `closed` | Restaurant is closed on that day of week (e.g. Tuesdays). | Present regular operating days and offer next open day. |
| `special_closure` | `closed` | Scheduled holiday or maintenance event closure. | Inform diner of closure reason and reopen date. |
| `outside_hours` | `closed` | Requested time is before open, after close, in service gap, or past last seating. | State operating hours from `opening_hours_that_day`. |
| `dst_gap` | `invalid_request` | Requested time falls into a Daylight Saving spring-forward gap. | Request alternative time outside the gap window. |

---

## 4. Spoken Voice Text Phrasing Specifications

The assistant voice prompt is returned in the text `content` block accompanying the structured JSON:

1. **Available Slot**:
   - `"I have a table for 2 guests on Friday the sixteenth at seven thirty in the evening. Want me to book it?"`
2. **Unavailable with Different Area**:
   - `"I don't have indoor seating at seven thirty in the evening, but patio is free at seven thirty in the evening, or I have nine in indoor. Would either work for you?"`
3. **Unavailable with Alternative Times**:
   - `"I don't have a table available at seven thirty in the evening, but I have seven or eight fifteen in the evening. Would either of those work for you?"`
4. **Closed Day**:
   - `"The restaurant is closed on Tuesday the twentieth. Would you like to check availability for tomorrow?"`
5. **Outside Hours**:
   - `"The restaurant is not open at four in the afternoon. Opening hours that day are noon to three thirty in the afternoon and seven to eleven in the evening."`

---

## 5. Sample Requests and Responses

### 5.1 Sample: Available Slot
**Request**:
```json
{
  "name": "check_availability",
  "arguments": {
    "date": "2026-10-16",
    "time": "19:30",
    "party_size": 2,
    "area": "indoor"
  }
}
```
**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I have a table for 2 guests on Friday the sixteenth at seven thirty in the evening. Want me to book it?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-16",
      "time": "19:30",
      "party_size": 2,
      "area": "indoor",
      "duration_minutes": 75,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:10:58+05:30",
    "status": "available",
    "reason": null,
    "reason_detail": null,
    "snapped_from": null,
    "slot": {
      "start_local": "2026-10-16T19:30:00+05:30",
      "end_local": "2026-10-16T20:45:00+05:30",
      "table_label": "T01",
      "area": "indoor",
      "capacity": 2
    },
    "alternatives": [],
    "opening_hours_that_day": [
      { "open": "12:00", "close": "15:30" },
      { "open": "19:00", "close": "23:30" }
    ],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

### 5.2 Sample: Fully Booked with Alternatives
**Request**:
```json
{
  "name": "check_availability",
  "arguments": {
    "date": "2026-10-17",
    "time": "19:30",
    "party_size": 4,
    "area": "indoor"
  }
}
```
**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "I don't have indoor seating at seven thirty in the evening, but patio is free at seven thirty in the evening, or I have nine in indoor. Would either work for you?"
    }
  ],
  "structuredContent": {
    "requested": {
      "date": "2026-10-17",
      "time": "19:30",
      "party_size": 4,
      "area": "indoor",
      "duration_minutes": 90,
      "timezone": "Asia/Kolkata"
    },
    "restaurant_now": "2026-10-13T14:10:58+05:30",
    "status": "unavailable",
    "reason": "fully_booked",
    "reason_detail": "No tables available for party of 4 at 19:30 in indoor area. Found 3 alternative slot(s).",
    "snapped_from": null,
    "slot": null,
    "alternatives": [
      {
        "start_local": "2026-10-17T19:30:00+05:30",
        "end_local": "2026-10-17T21:00:00+05:30",
        "area": "patio",
        "table_label": "P02",
        "minutes_from_requested": 0,
        "different_day": false
      },
      {
        "start_local": "2026-10-17T21:00:00+05:30",
        "end_local": "2026-10-17T22:30:00+05:30",
        "area": "indoor",
        "table_label": "T03",
        "minutes_from_requested": 90,
        "different_day": false
      },
      {
        "start_local": "2026-10-17T21:15:00+05:30",
        "end_local": "2026-10-17T22:45:00+05:30",
        "area": "indoor",
        "table_label": "T03",
        "minutes_from_requested": 105,
        "different_day": false
      }
    ],
    "opening_hours_that_day": [
      { "open": "12:00", "close": "16:00" },
      { "open": "19:00", "close": "23:30" }
    ],
    "note": "availability is a snapshot, not a reservation"
  },
  "isError": false
}
```

### 5.3 Sample: Tool Execution Error (Self-Correction Format)
**Request**:
```json
{
  "name": "check_availability",
  "arguments": {
    "date": "tomorrow",
    "time": "19:30",
    "party_size": 2
  }
}
```
**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Invalid input: 'date' must be in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Received 'tomorrow'."
    }
  ],
  "isError": true
}
```

---

## 6. Tool: `create_booking`

### 6.1 Purpose & Role in Agentic Workflow
`create_booking` executes the transactional write path for dining reservations. It takes validated reservation parameters, re-evaluates capacity *inside a `BEGIN IMMEDIATE` write transaction* (never trusting a prior availability snapshot), selects the optimal best-fit table, and inserts the confirmed booking atomically with audit trails and idempotency tracking.

### 6.2 Caller Responsibilities & Idempotency Guidance
- **Idempotency Key**: Callers are strongly advised to pass an `idempotency_key` (8–64 alphanumeric characters, hyphens, or underscores). On network retries or assistant loops, reusing the same key guarantees that identical parameters return the original booking (`idempotent_replay: true`) without creating duplicate bookings.
- **Conflict Handling**: Sending the same `idempotency_key` with modified parameters results in status `"idempotency_conflict"` and creates no rows.
- **Business Outcomes**: Slot unavailability (`"unavailable"`, `"closed"`, `"limit_exceeded"`) returns a standard successful tool response (`isError: false`) with structured alternatives and clear spoken text. Tool execution errors (`isError: true`) are reserved strictly for malformed inputs.

### 6.3 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 6.4 Input Schema
Dialect: JSON Schema Draft 2020-12
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "date": {
      "type": "string",
      "description": "Local restaurant date in ISO format YYYY-MM-DD (e.g. '2026-10-17'). Must be a concrete calendar date within the 30-day reservation window."
    },
    "time": {
      "type": "string",
      "description": "Local 24-hour time in HH:MM format (e.g. '19:30'). Off-grid times are automatically snapped to 15-minute slot intervals."
    },
    "party_size": {
      "type": "integer",
      "format": "int64",
      "description": "Number of guests (1-12)."
    },
    "guest_name": {
      "type": "string",
      "description": "Full name of the reservation holder (1-80 characters, trimmed)."
    },
    "phone": {
      "type": ["string", "null"],
      "description": "Optional contact phone number. Validated loosely; never logged in plaintext."
    },
    "email": {
      "type": ["string", "null"],
      "description": "Optional contact email address. Validated loosely; never logged in plaintext."
    },
    "area": {
      "type": ["string", "null"],
      "enum": ["any", "indoor", "patio", "bar"],
      "description": "Preferred dining area ('indoor', 'patio', 'bar', 'any'). Defaults to 'any'."
    },
    "duration_minutes": {
      "type": ["integer", "null"],
      "format": "int64",
      "description": "Optional custom dining duration in minutes (30-240). Defaults based on party size."
    },
    "special_requests": {
      "type": ["string", "null"],
      "description": "Optional dietary or seating requests (max 300 characters, stored as plain text)."
    },
    "idempotency_key": {
      "type": ["string", "null"],
      "description": "Optional 8-64 character string [a-zA-Z0-9_-]. Reusing the same key upon retries returns the existing booking safely."
    },
    "customer_id": {
      "type": ["string", "null"],
      "description": "Optional customer identifier for future CRM phases."
    }
  },
  "required": ["date", "time", "party_size", "guest_name"]
}
```

### 6.5 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": {
      "type": "string",
      "enum": ["confirmed", "unavailable", "closed", "invalid_request", "limit_exceeded", "idempotency_conflict"]
    },
    "reason": { "type": ["string", "null"] },
    "reason_detail": { "type": ["string", "null"] },
    "booking": {
      "type": ["object", "null"],
      "properties": {
        "confirmation_code": { "type": "string" },
        "guest_name": { "type": "string" },
        "party_size": { "type": "integer" },
        "start_local": { "type": "string" },
        "end_local": { "type": "string" },
        "area": { "type": "string" },
        "table_label": { "type": "string" },
        "status": { "type": "string" },
        "special_requests": { "type": ["string", "null"] },
        "created_at": { "type": "string" }
      }
    },
    "idempotent_replay": { "type": "boolean" },
    "alternatives": { "type": "array" },
    "cancellation_policy": { "type": ["string", "null"] },
    "note": { "type": "string" }
  },
  "required": ["status", "idempotent_replay", "alternatives", "note"]
}
```

### 6.6 Status and Reason Codes
| Status | Reason Code | Description |
|---|---|---|
| `confirmed` | `null` | Table booked successfully. Confirmation code assigned. |
| `unavailable` | `fully_booked` | No matching table capacity at requested time. Alternatives provided. |
| `unavailable` | `beyond_horizon` | Requested date is further in advance than the 30-day booking horizon. |
| `unavailable` | `in_the_past` | Requested slot is earlier than the restaurant's current local time. |
| `closed` | `closed_day` | Restaurant is closed on this day of week (e.g., Tuesdays). |
| `closed` | `outside_hours` | Slot falls outside regular meal service hours or past last seating. |
| `limit_exceeded` | `active_bookings_limit` | Guest has reached the maximum permitted active future bookings (default 3). |
| `idempotency_conflict` | `idempotency_conflict` | Idempotency key reused with mismatched booking parameters. |

---

## 7. Tool: `cancel_booking`

### 7.1 Purpose & Anti-Enumeration Design
`cancel_booking` cancels an existing confirmed reservation.
- **Verification Guarantee**: Cancellation requires both the `confirmation_code` and the `guest_name`. The name match is case-insensitive and trimmed.
- **Anti-Enumeration Protection**: If the confirmation code does not exist or the guest name does not match, the tool returns the **identical generic error response** (`"No matching booking found for the provided confirmation code and guest name."`) to prevent attackers from probing valid confirmation codes.
- **Rate Limiting**: An in-memory rate limiter tracks failed attempts per confirmation code. More than 5 failed attempts in 15 minutes triggers a temporary lock.
- **State Transition**: Rows are **never deleted**. Status changes to `"cancelled"` and `cancelled_at` is stamped.

### 7.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": true,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 7.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "confirmation_code": {
      "type": "string",
      "description": "Confirmation code assigned during booking (e.g. 'TT-8K4MZ'). Case-insensitive; hyphens and spaces tolerated."
    },
    "guest_name": {
      "type": "string",
      "description": "Full name of the reservation holder for verification. Must match the booking record."
    },
    "reason": {
      "type": ["string", "null"],
      "description": "Optional reason for cancellation (max 200 characters)."
    }
  },
  "required": ["confirmation_code", "guest_name"]
}
```

### 7.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": {
      "type": "string",
      "enum": ["cancelled", "already_cancelled", "not_found", "not_cancellable"]
    },
    "already_cancelled": { "type": "boolean" },
    "booking": { "type": ["object", "null"] },
    "policy_note": { "type": ["string", "null"] },
    "reason": { "type": ["string", "null"] },
    "reason_detail": { "type": ["string", "null"] }
  },
  "required": ["status", "already_cancelled"]
}
```

---

## 8. Tool: `get_booking`

### 8.1 Purpose & Verification
`get_booking` is a **read-only** tool that retrieves the booking summary for a guest. Like `cancel_booking`, it requires verification via `guest_name` alongside `confirmation_code` and returns a generic not-found response on any mismatch. Internal row IDs and private contact information are stripped.

### 8.2 Annotations
```json
{
  "readOnlyHint": true,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 8.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "confirmation_code": {
      "type": "string",
      "description": "Confirmation code assigned during booking (e.g. 'TT-8K4MZ'). Case-insensitive; hyphens and spaces tolerated."
    },
    "guest_name": {
      "type": "string",
      "description": "Full name of the reservation holder for verification."
    }
  },
  "required": ["confirmation_code", "guest_name"]
}
```

### 8.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": {
      "type": "string",
      "enum": ["found", "not_found"]
    },
    "booking": { "type": ["object", "null"] },
    "message": { "type": "string" }
  },
  "required": ["status", "message"]
}
```

---

## 9. Tool: `place_order`

### 9.1 Purpose & Role in Agentic Workflow
`place_order` transactionally creates a takeout/pickup food order at "The Saffron Nebula Bistro". It enforces server-side pricing integrity, strict allergen conflict blocking with unverified data protection, dynamic kitchen preparation time estimation, and idempotent retries.

All orders in Phase 9 are pickup-only with payment status fixed to `"unpaid_pay_at_pickup"`. No payments, deposits, session memory, or kitchen display systems are enabled in this phase.

### 9.2 Caller Responsibilities (LLM Prompting Guidance)
- **Do Not Invent Prices**: The caller must never pass prices. Prices are computed purely server-side from active menu records.
- **Do Not Invent Pickup Times**: If the guest has no specific time preference, omit `pickup_time`; the engine automatically computes and assigns the earliest feasible pickup time based on kitchen load.
- **Item Resolution**: Callers should prefer passing `item_id` obtained from `search_menu`. If passing `item_name`, provide exact names. Ambiguous names return candidate dishes so the caller can ask the guest for clarification.
- **Allergen Rules**: If the guest expresses allergen restrictions, pass them in `allergen_acknowledgement.allergens_to_avoid`.

### 9.3 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 9.4 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "items": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "item_id": { "type": ["integer", "null"] },
          "item_name": { "type": ["string", "null"] },
          "quantity": { "type": "integer", "minimum": 1, "maximum": 20 },
          "notes": { "type": ["string", "null"], "maxLength": 140 }
        },
        "required": ["quantity"]
      },
      "minItems": 1,
      "maxItems": 20,
      "description": "1-20 order line items. Provide either item_id or item_name for each line."
    },
    "pickup_time": {
      "type": ["string", "null"],
      "description": "Optional local HH:MM or ISO timestamp for pickup today. Do not invent times; defaults to earliest feasible pickup time."
    },
    "guest_name": {
      "type": "string",
      "minLength": 1,
      "maxLength": 80,
      "description": "Name of the guest picking up the order."
    },
    "phone": { "type": ["string", "null"] },
    "email": { "type": ["string", "null"] },
    "allergen_acknowledgement": {
      "type": ["object", "null"],
      "properties": {
        "allergens_to_avoid": {
          "type": "array",
          "items": {
            "type": "string",
            "enum": ["dairy", "egg", "fish", "shellfish", "tree_nuts", "peanuts", "wheat", "soy", "sesame", "gluten", "mustard", "celery", "lupin", "molluscs", "sulphites"]
          }
        },
        "acknowledged": { "type": "boolean" }
      },
      "required": ["allergens_to_avoid", "acknowledged"]
    },
    "order_notes": { "type": ["string", "null"], "maxLength": 300 },
    "idempotency_key": { "type": ["string", "null"], "maxLength": 128 }
  },
  "required": ["items", "guest_name"]
}
```

### 9.5 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": {
      "type": "string",
      "enum": [
        "placed",
        "item_unavailable",
        "unknown_item",
        "ambiguous_item",
        "pickup_too_soon",
        "closed",
        "allergen_conflict",
        "limit_exceeded",
        "idempotency_conflict",
        "invalid_request"
      ]
    },
    "reason": { "type": ["string", "null"] },
    "reason_detail": { "type": ["string", "null"] },
    "order": {
      "type": ["object", "null"],
      "properties": {
        "order_code": { "type": "string" },
        "guest_name": { "type": "string" },
        "status": { "type": "string" },
        "pickup_time_local": { "type": "string" },
        "lines": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "item_id": { "type": "integer" },
              "name": { "type": "string" },
              "quantity": { "type": "integer" },
              "unit_price": {
                "type": "object",
                "properties": {
                  "amount_minor": { "type": "integer" },
                  "currency": { "type": "string" },
                  "display": { "type": "string" }
                },
                "required": ["amount_minor", "currency", "display"]
              },
              "line_total": {
                "type": "object",
                "properties": {
                  "amount_minor": { "type": "integer" },
                  "currency": { "type": "string" },
                  "display": { "type": "string" }
                },
                "required": ["amount_minor", "currency", "display"]
              },
              "notes": { "type": ["string", "null"] },
              "allergens": { "type": "array", "items": { "type": "string" } },
              "dietary_tags": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["item_id", "name", "quantity", "unit_price", "line_total", "allergens", "dietary_tags"]
          }
        },
        "subtotal": {
          "type": "object",
          "properties": {
            "amount_minor": { "type": "integer" },
            "currency": { "type": "string" },
            "display": { "type": "string" }
          },
          "required": ["amount_minor", "currency", "display"]
        },
        "tax": {
          "type": "object",
          "properties": {
            "amount_minor": { "type": "integer" },
            "currency": { "type": "string" },
            "display": { "type": "string" }
          },
          "required": ["amount_minor", "currency", "display"]
        },
        "total": {
          "type": "object",
          "properties": {
            "amount_minor": { "type": "integer" },
            "currency": { "type": "string" },
            "display": { "type": "string" }
          },
          "required": ["amount_minor", "currency", "display"]
        },
        "payment_status": { "type": "string" },
        "stated_allergens": { "type": ["string", "null"] },
        "created_at": { "type": "string" }
      },
      "required": ["order_code", "guest_name", "status", "pickup_time_local", "lines", "subtotal", "tax", "total", "payment_status", "created_at"]
    },
    "price_changes": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "item_id": { "type": "integer" },
          "item_name": { "type": "string" },
          "current_price": {
            "type": "object",
            "properties": {
              "amount_minor": { "type": "integer" },
              "currency": { "type": "string" },
              "display": { "type": "string" }
            },
            "required": ["amount_minor", "currency", "display"]
          }
        },
        "required": ["item_id", "item_name", "current_price"]
      }
    },
    "candidates": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "item_id": { "type": "integer" },
          "name": { "type": "string" },
          "price": {
            "type": "object",
            "properties": {
              "amount_minor": { "type": "integer" },
              "currency": { "type": "string" },
              "display": { "type": "string" }
            },
            "required": ["amount_minor", "currency", "display"]
          }
        },
        "required": ["item_id", "name", "price"]
      }
    },
    "alternatives": { "type": "array", "items": { "type": "string" } },
    "earliest_pickup_local": { "type": ["string", "null"] },
    "allergen_notice": { "type": "string" },
    "idempotent_replay": { "type": "boolean" }
  },
  "required": [
    "status",
    "price_changes",
    "candidates",
    "alternatives",
    "allergen_notice",
    "idempotent_replay"
  ]
}
```

### 9.6 Status and Reason Codes
| Status | Reason | Meaning / Next Caller Action |
|---|---|---|
| `placed` | `null` | Order successfully written inside `BEGIN IMMEDIATE`. Read confirmation speech to guest. |
| `allergen_conflict` | `allergen_conflict` | At least one item contains or may contain avoided allergen, or allergen data is unverified. Suggest returned alternatives. |
| `ambiguous_item` | `ambiguous_item` | An `item_name` matched multiple menu items. Ask guest to clarify using returned `candidates`. |
| `unknown_item` | `unknown_item` | An `item_id` or `item_name` does not exist on the active menu. |
| `item_unavailable` | `item_unavailable` | Item is out of stock (`available = false`). |
| `pickup_too_soon` | `pickup_too_soon` | Requested time is earlier than prep time + queue allowance. Suggest `earliest_pickup_local`. |
| `closed` | `closed_day` / `outside_hours` | Restaurant is closed or order cutoff passed. |
| `limit_exceeded` | `too_many_active_orders` / `order_total_exceeded` | Guest exceeded 3 active orders or ₹25,000 minor unit total. |
| `idempotency_conflict` | `idempotency_conflict` | Idempotency key was replayed with altered request parameters. |
| `invalid_request` | `empty_order` / `too_many_items` / etc. | Structural or business validation error. |

---

## 10. Tool: `get_order`

### 10.1 Purpose & Role in Agentic Workflow
Read-only order status inspection. Looks up an existing pickup order by its human-friendly code (`TO-XXXXX`) and guest name.

### 10.2 Annotations
```json
{
  "readOnlyHint": true,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 10.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "order_code": { "type": "string", "description": "Human-friendly code (e.g. 'TO-GWT4K')" },
    "guest_name": { "type": "string", "description": "Full name of the guest who placed the order for verification." }
  },
  "required": ["order_code", "guest_name"]
}
```

### 10.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["found", "not_found"] },
    "order": { "type": ["object", "null"] },
    "message": { "type": "string" }
  },
  "required": ["status", "message"]
}
```

---

## 11. Tool: `cancel_order`

### 11.1 Purpose & Role in Agentic Workflow
Cancels a previously placed pickup order. Cancellation is strictly allowed only while the order is in `placed` status (before kitchen preparation begins). Double cancellations are idempotent.

### 11.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": true,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 11.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "order_code": { "type": "string", "description": "Human-friendly code (e.g. 'TO-GWT4K')" },
    "guest_name": { "type": "string", "description": "Full name of the guest for verification." },
    "reason": { "type": ["string", "null"], "maxLength": 200, "description": "Optional cancellation reason." }
  },
  "required": ["order_code", "guest_name"]
}
```

### 11.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["cancelled", "already_cancelled", "not_found", "not_cancellable", "limit_exceeded"] },
    "already_cancelled": { "type": "boolean" },
    "order": { "type": ["object", "null"] },
    "policy_note": { "type": ["string", "null"] },
    "reason": { "type": ["string", "null"] },
    "reason_detail": { "type": ["string", "null"] }
  },
  "required": ["status", "already_cancelled"]
}
```

---

## 12. Tool: `enable_memory`

### 12.1 Purpose & Role in Agentic Workflow
Enables persistent profile memory for a dining guest. **Memory is strictly OFF by default.** Callers must explicitly obtain guest consent (`consent_acknowledged: true`). Generates and returns a 256-bit cryptographically secure profile token (`tt_prof_...`) exactly once.

### 12.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": false,
  "idempotentHint": false,
  "openWorldHint": false
}
```

### 12.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "guest_name": { "type": "string", "minLength": 1, "maxLength": 80 },
    "phone": { "type": ["string", "null"] },
    "email": { "type": ["string", "null"] },
    "consent_acknowledged": { "type": "boolean" },
    "initial_preferences": {
      "type": ["array", "null"],
      "items": {
        "type": "object",
        "properties": {
          "kind": { "type": "string", "enum": ["allergy", "dietary", "seat_pref", "favorite_item", "note"] },
          "value": { "type": "string" },
          "severity": { "type": ["string", "null"], "enum": ["mild", "severe", "unknown"] }
        },
        "required": ["kind", "value"]
      }
    }
  },
  "required": ["guest_name", "consent_acknowledged"]
}
```

### 12.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["enabled", "consent_required"] },
    "profile_token": { "type": ["string", "null"] },
    "guest_name": { "type": "string" },
    "consent_version": { "type": "string" },
    "preferences_saved": { "type": "integer" },
    "message": { "type": "string" }
  },
  "required": ["status", "guest_name", "consent_version", "preferences_saved", "message"]
}
```

---

## 13. Tool: `get_profile`

### 13.1 Purpose & Role in Agentic Workflow
Read-only profile inspection tool. Returns profile display name, masked contact details, typed preferences, re-confirmation notices, and a full `data_export` object that functions as the GDPR Art. 15 / DPDP data access feature.

### 13.2 Annotations
```json
{
  "readOnlyHint": true,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 13.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" }
  },
  "required": ["profile_token"]
}
```

### 13.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["found", "not_found"] },
    "guest_name": { "type": "string" },
    "phone_masked": { "type": ["string", "null"] },
    "email_masked": { "type": ["string", "null"] },
    "consent_version": { "type": "string" },
    "consent_granted_at": { "type": "string" },
    "preferences": { "type": "array" },
    "counts": {
      "type": "object",
      "properties": {
        "total_orders": { "type": "integer" },
        "total_bookings": { "type": "integer" },
        "preference_count": { "type": "integer" }
      },
      "required": ["total_orders", "total_bookings", "preference_count"]
    },
    "needs_reconfirmation": { "type": "array", "items": { "type": "string" } },
    "data_export": { "type": ["object", "null"] }
  },
  "required": ["status", "guest_name", "consent_version", "consent_granted_at", "preferences", "counts", "needs_reconfirmation"]
}
```

---

## 14. Tool: `remember_preference`

### 14.1 Purpose & Role in Agentic Workflow
Stores a validated, typed dining preference. Deduplicates repeated entries idempotently. Allergies and dietary values must resolve against database registers. Notes are capped at 140 chars with a hard limit of 20 notes per profile.

### 14.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 14.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" },
    "kind": { "type": "string", "enum": ["allergy", "dietary", "seat_pref", "favorite_item", "note"] },
    "value": { "type": "string" },
    "severity": { "type": ["string", "null"], "enum": ["mild", "severe", "unknown"] }
  },
  "required": ["profile_token", "kind", "value"]
}
```

### 14.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["remembered", "not_found", "validation_failed"] },
    "preference": { "type": ["object", "null"] },
    "deduplicated": { "type": "boolean" },
    "message": { "type": "string" }
  },
  "required": ["status", "deduplicated", "message"]
}
```

---

## 15. Tool: `forget_preference`

### 15.1 Purpose & Role in Agentic Workflow
Removes a specific preference from the guest's profile. Removing an **allergy** strictly requires `confirm_remove_allergy: true`. The voice response explicitly reminds guests that deletion applies to the digital profile only and they must continue informing waitstaff.

### 15.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": true,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 15.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" },
    "kind": { "type": "string" },
    "value": { "type": "string" },
    "confirm_remove_allergy": { "type": ["boolean", "null"], "default": false }
  },
  "required": ["profile_token", "kind", "value"]
}
```

### 15.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["forgotten", "confirmation_required", "not_found"] },
    "kind": { "type": "string" },
    "value": { "type": "string" },
    "removed": { "type": "boolean" },
    "policy_note": { "type": ["string", "null"] },
    "message": { "type": "string" }
  },
  "required": ["status", "kind", "value", "removed", "message"]
}
```

---

## 16. Tool: `get_usual_order`

### 16.1 Purpose & Role in Agentic Workflow
Computes a "usual order" suggestion from historical placed or completed orders. Re-evaluates every item against the **live menu state**: checks current stock availability, flags price shifts, and filters against stored profile allergies. Does **not** place an order.

### 16.2 Annotations
```json
{
  "readOnlyHint": true,
  "destructiveHint": false,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 16.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" }
  },
  "required": ["profile_token"]
}
```

### 16.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["suggested", "no_history", "not_found"] },
    "guest_name": { "type": ["string", "null"] },
    "suggested_lines": { "type": "array" },
    "subtotal": { "type": ["object", "null"] },
    "tax": { "type": ["object", "null"] },
    "total": { "type": ["object", "null"] },
    "price_changes": { "type": "array" },
    "unavailable_items": { "type": "array", "items": { "type": "string" } },
    "conflicting_items": { "type": "array" },
    "needs_allergy_reconfirmation": { "type": "array", "items": { "type": "string" } },
    "allergen_notice": { "type": "string" }
  },
  "required": ["status", "suggested_lines", "price_changes", "unavailable_items", "conflicting_items", "needs_allergy_reconfirmation", "allergen_notice"]
}
```

---

## 17. Tool: `rotate_profile_token`

### 17.1 Purpose & Role in Agentic Workflow
Rotates the bearer profile token atomically. Issues a new token and permanently revokes the existing token in a single transaction.

### 17.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": false,
  "idempotentHint": false,
  "openWorldHint": false
}
```

### 17.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" }
  },
  "required": ["profile_token"]
}
```

### 17.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["rotated", "not_found"] },
    "new_profile_token": { "type": ["string", "null"] },
    "message": { "type": "string" }
  },
  "required": ["status", "message"]
}
```

---

## 18. Tool: `forget_me`

### 18.1 Purpose & Role in Agentic Workflow
Enforces complete hard-deletion of the guest profile, preferences, tokens, and consent records. Past orders and bookings are unlinked and scrubbed to non-identifying aggregates (`[deleted]` name, NULL contacts/notes). Deletes cached idempotency response records.

### 18.2 Annotations
```json
{
  "readOnlyHint": false,
  "destructiveHint": true,
  "idempotentHint": true,
  "openWorldHint": false
}
```

### 18.3 Input Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "profile_token": { "type": "string" },
    "confirm": { "type": "boolean" }
  },
  "required": ["profile_token", "confirm"]
}
```

### 18.4 Output Schema
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "status": { "type": "string", "enum": ["forgotten", "confirmation_required", "not_found"] },
    "removed_components": { "type": "array", "items": { "type": "string" } },
    "scrubbed_business_records": {
      "type": "object",
      "properties": {
        "scrubbed_orders": { "type": "integer" },
        "scrubbed_bookings": { "type": "integer" }
      },
      "required": ["scrubbed_orders", "scrubbed_bookings"]
    },
    "message": { "type": "string" }
  },
  "required": ["status", "removed_components", "scrubbed_business_records", "message"]
}
```


