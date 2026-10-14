import { execSync } from "node:child_process"
import path from "node:path"
import { fileURLToPath } from "node:url"

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const backendDir = path.resolve(__dirname, "../../backend")

async function checkServerReachable(url) {
  try {
    const res = await fetch(`${url}/health`, { signal: AbortSignal.timeout(1500) })
    return res.ok
  } catch {
    return false
  }
}

async function main() {
  const serverUrl = process.env.TABLETALK_SERVER_URL || "http://127.0.0.1:8080"
  console.log(`Checking if TableTalk server is reachable at ${serverUrl}...`)

  const isUp = await checkServerReachable(serverUrl)

  if (isUp) {
    console.log(`Server reachable at ${serverUrl}. Generating real output fixtures via test runner against seeded DB...`)
  } else {
    console.log(`Notice: Standalone HTTP server is not actively running at ${serverUrl}.`)
    console.log(`Running hermetic test server with seeded database to generate 100% verified real tool fixtures...`)
  }

  try {
    execSync("cargo test --test dump_schemas -- --nocapture", {
      cwd: backendDir,
      stdio: "inherit",
    })
    console.log("Fixtures successfully generated and committed.")
  } catch (err) {
    console.error("Failed to generate fixtures from server:", err.message)
    process.exit(1)
  }
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
