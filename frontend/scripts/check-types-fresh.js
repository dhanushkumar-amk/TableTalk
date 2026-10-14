import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { execSync } from "node:child_process"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const rootDir = path.resolve(__dirname, "..")
const typesPath = path.resolve(rootDir, "src/cards/types.ts")

const before = fs.readFileSync(typesPath, "utf-8")
execSync("node scripts/generate-types.js", { cwd: rootDir, stdio: "inherit" })
const after = fs.readFileSync(typesPath, "utf-8")

if (before !== after) {
  console.error("Error: src/cards/types.ts is stale! Run 'npm run gen:types' and commit the changes.")
  process.exit(1)
} else {
  console.log("Types staleness check passed: src/cards/types.ts is fresh.")
}
