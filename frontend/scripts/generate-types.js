import { compile } from "json-schema-to-typescript"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, "..")
const schemasPath = path.resolve(rootDir, "src/cards/schemas/output_schemas.json")
const outputPath = path.resolve(rootDir, "src/cards/types.ts")

async function main() {
  if (!fs.existsSync(schemasPath)) {
    console.error(`Error: Schemas file not found at ${schemasPath}`)
    process.exit(1)
  }

  const rawSchemas = JSON.parse(fs.readFileSync(schemasPath, "utf-8"))

  // Build a unified schema bundle
  const unifiedDefs = {}
  const properties = {}
  const required = []

  for (const [toolName, schema] of Object.entries(rawSchemas)) {
    if (!schema || typeof schema !== "object") continue

    if (schema.$defs) {
      for (const [defKey, defVal] of Object.entries(schema.$defs)) {
        if (!unifiedDefs[defKey]) {
          unifiedDefs[defKey] = defVal
        }
      }
    }

    const toolSchemaCopy = { ...schema }
    delete toolSchemaCopy.$defs
    delete toolSchemaCopy.$schema

    properties[toolName] = toolSchemaCopy
    required.push(toolName)
  }

  const bundledSchema = {
    title: "TableTalkAllTools",
    type: "object",
    properties,
    required,
    $defs: unifiedDefs,
    additionalProperties: false,
  }

  const compiled = await compile(bundledSchema, "TableTalkAllTools", {
    bannerComment: `/**
 * AUTO-GENERATED FILE - DO NOT EDIT MANUALLY.
 * Generated from TableTalk MCP Server output schemas.
 * Run 'npm run gen:types' to regenerate.
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
`,
    style: { semi: true, singleQuote: false },
    unknownAny: false,
    additionalProperties: false,
  })

  // Extract individual tool output types and create the contract mapping
  let extraTypes = `
/* ==================== Derived Tool Output Types & Registry ==================== */

export type PingRestaurantOutput = TableTalkAllTools["ping_restaurant"];
export type SearchMenuOutput = TableTalkAllTools["search_menu"];
export type CheckAvailabilityOutput = TableTalkAllTools["check_availability"];
export type CreateBookingOutput = TableTalkAllTools["create_booking"];
export type CancelBookingOutput = TableTalkAllTools["cancel_booking"];
export type GetBookingOutput = TableTalkAllTools["get_booking"];
export type PlaceOrderOutput = TableTalkAllTools["place_order"];
export type GetOrderOutput = TableTalkAllTools["get_order"];
export type CancelOrderOutput = TableTalkAllTools["cancel_order"];
export type EnableMemoryOutput = TableTalkAllTools["enable_memory"];
export type GetProfileOutput = TableTalkAllTools["get_profile"];
export type RememberPreferenceOutput = TableTalkAllTools["remember_preference"];
export type ForgetPreferenceOutput = TableTalkAllTools["forget_preference"];
export type GetUsualOrderOutput = TableTalkAllTools["get_usual_order"];
export type RotateProfileTokenOutput = TableTalkAllTools["rotate_profile_token"];
export type ForgetMeOutput = TableTalkAllTools["forget_me"];
export type NegotiateBookingOutput = TableTalkAllTools["negotiate_booking"];
export type AcceptOfferOutput = TableTalkAllTools["accept_offer"];
export type ReleaseOfferOutput = TableTalkAllTools["release_offer"];
export type JoinWaitlistOutput = TableTalkAllTools["join_waitlist"];
export type GetWaitlistStatusOutput = TableTalkAllTools["get_waitlist_status"];
export type LeaveWaitlistOutput = TableTalkAllTools["leave_waitlist"];
export type GetDepositStatusOutput = TableTalkAllTools["get_deposit_status"];
export type RefreshPaymentSessionOutput = TableTalkAllTools["refresh_payment_session"];

export interface ToolStructuredPayloadMap {
  ping_restaurant: PingRestaurantOutput;
  search_menu: SearchMenuOutput;
  check_availability: CheckAvailabilityOutput;
  create_booking: CreateBookingOutput;
  cancel_booking: CancelBookingOutput;
  get_booking: GetBookingOutput;
  place_order: PlaceOrderOutput;
  get_order: GetOrderOutput;
  cancel_order: CancelOrderOutput;
  enable_memory: EnableMemoryOutput;
  get_profile: GetProfileOutput;
  remember_preference: RememberPreferenceOutput;
  forget_preference: ForgetPreferenceOutput;
  get_usual_order: GetUsualOrderOutput;
  rotate_profile_token: RotateProfileTokenOutput;
  forget_me: ForgetMeOutput;
  negotiate_booking: NegotiateBookingOutput;
  accept_offer: AcceptOfferOutput;
  release_offer: ReleaseOfferOutput;
  join_waitlist: JoinWaitlistOutput;
  get_waitlist_status: GetWaitlistStatusOutput;
  leave_waitlist: LeaveWaitlistOutput;
  get_deposit_status: GetDepositStatusOutput;
  refresh_payment_session: RefreshPaymentSessionOutput;
}

export type TableTalkToolName = keyof ToolStructuredPayloadMap;

export interface ToolResponseEnvelope<T = unknown> {
  content?: Array<{ type: string; text?: string; [key: string]: unknown }>;
  structuredContent?: T;
  isError?: boolean;
}
`

  fs.writeFileSync(outputPath, compiled + "\n" + extraTypes, "utf-8")
  console.log(`Successfully generated TypeScript types at ${outputPath}`)
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
