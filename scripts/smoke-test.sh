#!/usr/bin/env bash
# scripts/smoke-test.sh
# Automated MCP Inspector CLI and protocol smoke tests for TableTalk backend.

set -euo pipefail

PORT="${PORT:-8088}"
HOST="${HOST:-127.0.0.1}"
BASE_URL="http://${HOST}:${PORT}"
MCP_URL="${BASE_URL}/mcp"

PASS_COUNT=0
FAIL_COUNT=0

log_pass() {
  echo -e "\033[32m[PASS]\033[0m $1"
  PASS_COUNT=$((PASS_COUNT + 1))
}

log_fail() {
  echo -e "\033[31m[FAIL]\033[0m $1"
  FAIL_COUNT=$((FAIL_COUNT + 1))
}

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    echo "Shutting down server (PID: ${SERVER_PID})..."
    kill -TERM "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

echo "=================================================="
echo " Starting TableTalk MCP Server on port ${PORT}..."
echo "=================================================="

BIN_PATH=""
if [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/tabletalk-backend" ]]; then
  BIN_PATH="${CARGO_TARGET_DIR}/debug/tabletalk-backend"
elif [[ -n "${CARGO_TARGET_DIR:-}" && -x "${CARGO_TARGET_DIR}/debug/tabletalk-backend.exe" ]]; then
  BIN_PATH="${CARGO_TARGET_DIR}/debug/tabletalk-backend.exe"
elif [[ -x "/c/Users/dhanu/AppData/Local/cargo-target/tabletalk/debug/tabletalk-backend.exe" ]]; then
  BIN_PATH="/c/Users/dhanu/AppData/Local/cargo-target/tabletalk/debug/tabletalk-backend.exe"
elif [[ -x "target/debug/tabletalk-backend" ]]; then
  BIN_PATH="target/debug/tabletalk-backend"
elif [[ -x "target/debug/tabletalk-backend.exe" ]]; then
  BIN_PATH="target/debug/tabletalk-backend.exe"
fi

if [[ -n "${BIN_PATH}" ]]; then
  echo "Starting backend via pre-built binary: ${BIN_PATH}..."
  PORT="${PORT}" RUST_LOG="warn" "${BIN_PATH}" &
  SERVER_PID=$!
else
  echo "Starting backend via cargo run --bin tabletalk-backend..."
  PORT="${PORT}" RUST_LOG="warn" cargo run --bin tabletalk-backend &
  SERVER_PID=$!
fi

# Wait for /health
echo "Waiting for server to become healthy at ${BASE_URL}/health..."
MAX_RETRIES="${MAX_RETRIES:-30}"
RETRY_COUNT=0
READY=0

while [[ ${RETRY_COUNT} -lt ${MAX_RETRIES} ]]; do
  if curl -sf "${BASE_URL}/health" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 1
  RETRY_COUNT=$((RETRY_COUNT + 1))
done

if [[ ${READY} -ne 1 ]]; then
  echo -e "\033[31mServer failed to start within ${MAX_RETRIES} seconds.\033[0m"
  exit 1
fi
echo "Server is healthy (PID: ${SERVER_PID}). Running smoke tests..."
echo "--------------------------------------------------"

# Check 1: Health endpoint check
HEALTH_RESP=$(curl -s "${BASE_URL}/health")
if [[ "${HEALTH_RESP}" =~ "ok" ]]; then
  log_pass "Health endpoint returned 200 OK: ${HEALTH_RESP}"
else
  log_fail "Health endpoint check failed: ${HEALTH_RESP}"
fi

# Check 2: Inspector CLI tools/list
echo "Testing Inspector CLI tools/list against ${MCP_URL}..."
if INSPECTOR_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/list 2>&1); then
  if [[ "${INSPECTOR_OUT}" =~ "ping_restaurant" ]]; then
    log_pass "Inspector CLI tools/list discovered ping_restaurant"
  else
    log_fail "Inspector CLI tools/list output did not contain ping_restaurant: ${INSPECTOR_OUT}"
  fi
else
  log_fail "Inspector CLI tools/list failed execution: ${INSPECTOR_OUT}"
fi

# Check 3: Inspector CLI tools/call with argument
echo "Testing Inspector CLI tools/call ping_restaurant with name=Alice..."
if CALL_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name ping_restaurant --tool-arg name=Alice 2>&1); then
  if [[ "${CALL_OUT}" =~ "Alice" ]]; then
    log_pass "Inspector CLI tools/call with argument succeeded (contained 'Alice')"
  else
    log_fail "Inspector CLI tools/call with argument missing expected response: ${CALL_OUT}"
  fi
else
  log_fail "Inspector CLI tools/call with argument failed execution: ${CALL_OUT}"
fi

# Check 4: Inspector CLI tools/call without argument (default Guest)
echo "Testing Inspector CLI tools/call ping_restaurant without arguments..."
if CALL_DEF_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name ping_restaurant 2>&1); then
  if [[ "${CALL_DEF_OUT}" =~ "Guest" ]]; then
    log_pass "Inspector CLI tools/call without argument succeeded (contained default 'Guest')"
  else
    log_fail "Inspector CLI tools/call without argument missing 'Guest': ${CALL_DEF_OUT}"
  fi
else
  log_fail "Inspector CLI tools/call without argument failed execution: ${CALL_DEF_OUT}"
fi

# Check 5: Bad input / nonexistent tool via Inspector CLI
echo "Testing Inspector CLI tools/call with invalid tool..."
if BAD_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name nonexistent_tool 2>&1); then
  log_fail "Expected nonexistent_tool call to fail, but it exited 0: ${BAD_OUT}"
else
  log_pass "Inspector CLI correctly failed on nonexistent tool call"
fi

# Check 6: Protocol security - Disallowed Origin header rejected
echo "Testing Origin header security check via curl..."
ORIGIN_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${MCP_URL}" \
  -H "Origin: http://malicious-site.com" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "mcp-protocol-version: 2025-11-25" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

if [[ "${ORIGIN_STATUS}" == "403" ]]; then
  log_pass "Disallowed Origin returned 403 Forbidden"
else
  log_fail "Disallowed Origin expected 403, got: ${ORIGIN_STATUS}"
fi

# Check 7: Protocol version header validation via curl
echo "Testing invalid mcp-protocol-version header via curl..."
VERSION_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "${MCP_URL}" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -H "mcp-protocol-version: 1999-01-01" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}')

if [[ "${VERSION_STATUS}" == "400" ]]; then
  log_pass "Invalid mcp-protocol-version returned 400 Bad Request"
else
  log_fail "Invalid mcp-protocol-version expected 400, got: ${VERSION_STATUS}"
fi

echo "--------------------------------------------------"
echo "Summary: ${PASS_COUNT} passed, ${FAIL_COUNT} failed."
echo "=================================================="

if [[ ${FAIL_COUNT} -gt 0 ]]; then
  exit 1
fi

exit 0
