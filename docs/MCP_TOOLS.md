# TableTalk MCP Tools Specification

**Protocol Specification**: MCP `2025-11-25`  
**Transport**: Streamable HTTP (`POST /mcp`)  
**Server Version**: `tabletalk-backend` v0.1.0  

This document serves as the canonical reference for all MCP tools exposed by the TableTalk restaurant server. Field names and schemas documented here are stable and contractually guaranteed across assistant callers, the React simulator frontend, and downstream execution tools.

---

## 1. Tool Index

| Tool Name | Lifecycle Phase | Read-Only | Idempotent | Description |
|---|---|---|---|---|
| `ping_restaurant` | Phase 3 | Yes | Yes | Connectivity check and server metadata verification. |
| `search_menu` | Phase 6 | Yes | Yes | Search food items with dietary and allergen filters and spoken prices. |
| `check_availability` | Phase 7 | Yes | Yes | Check table availability, snap slot grid, and propose alternatives. |

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
