/**
 * Runtime schema validation for incoming structuredContent.
 *
 * Ensures that unexpected or corrupted payloads render a safe fallback card
 * showing the plain spoken text caption, never crashing and never dumping raw JSON.
 */

import type { TableTalkToolName } from "./types"

export interface ValidationResult {
  valid: boolean
  error?: string
}

export function validateCardPayload(
  toolName: TableTalkToolName | string,
  payload: unknown,
): ValidationResult {
  if (payload === null || payload === undefined) {
    return { valid: false, error: "Payload is null or undefined" }
  }

  if (typeof payload !== "object" || Array.isArray(payload)) {
    return { valid: false, error: "Payload must be a JSON object" }
  }

  const obj = payload as Record<string, unknown>

  switch (toolName) {
    case "search_menu":
      if (!Array.isArray(obj.items)) {
        return { valid: false, error: "search_menu requires 'items' array" }
      }
      return { valid: true }

    case "check_availability":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "check_availability requires 'status' string" }
      }
      return { valid: true }

    case "create_booking":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "create_booking requires 'status' string" }
      }
      return { valid: true }

    case "get_booking":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "get_booking requires 'status' string" }
      }
      return { valid: true }

    case "cancel_booking":
      if (typeof obj.status !== "string" || typeof obj.already_cancelled !== "boolean") {
        return { valid: false, error: "cancel_booking requires status and already_cancelled" }
      }
      return { valid: true }

    case "place_order":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "place_order requires 'status' string" }
      }
      return { valid: true }

    case "get_order":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "get_order requires 'status' string" }
      }
      return { valid: true }

    case "cancel_order":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "cancel_order requires 'status' string" }
      }
      return { valid: true }

    case "negotiate_booking":
      if (typeof obj.status !== "string" || !Array.isArray(obj.options)) {
        return { valid: false, error: "negotiate_booking requires status and options array" }
      }
      return { valid: true }

    case "accept_offer":
      if (typeof obj.status !== "string") {
        return { valid: false, error: "accept_offer requires 'status' string" }
      }
      return { valid: true }

    case "get_deposit_status":
    case "refresh_payment_session":
      if (typeof obj.status !== "string") {
        return { valid: false, error: `${toolName} requires 'status' string` }
      }
      return { valid: true }

    case "join_waitlist":
    case "get_waitlist_status":
    case "leave_waitlist":
      if (typeof obj.status !== "string") {
        return { valid: false, error: `${toolName} requires 'status' string` }
      }
      return { valid: true }

    case "get_profile":
    case "enable_memory":
    case "remember_preference":
    case "forget_preference":
    case "forget_me":
    case "get_usual_order":
      if (typeof obj.status !== "string") {
        return { valid: false, error: `${toolName} requires 'status' string` }
      }
      return { valid: true }

    case "ping_restaurant":
      return { valid: true }

    default:
      // Unknown tool: treat as invalid to trigger safe fallback
      return { valid: false, error: `Unrecognized tool name: ${toolName}` }
  }
}
