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

echo "Starting backend via cargo run --manifest-path backend/Cargo.toml --bin mcp_tabletalk_backend..."
PORT="${PORT}" RUST_LOG="warn" cargo run --manifest-path backend/Cargo.toml --bin mcp_tabletalk_backend &
SERVER_PID=$!

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
  if [[ "${INSPECTOR_OUT}" =~ "ping_restaurant" && "${INSPECTOR_OUT}" =~ "search_menu" && "${INSPECTOR_OUT}" =~ "check_availability" && "${INSPECTOR_OUT}" =~ "create_booking" && "${INSPECTOR_OUT}" =~ "cancel_booking" && "${INSPECTOR_OUT}" =~ "get_booking" && "${INSPECTOR_OUT}" =~ "place_order" && "${INSPECTOR_OUT}" =~ "get_order" && "${INSPECTOR_OUT}" =~ "cancel_order" ]]; then
    log_pass "Inspector CLI tools/list discovered all nine tools"
  else
    log_fail "Inspector CLI tools/list output did not contain all nine tools: ${INSPECTOR_OUT}"
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

# Check 8: Inspector CLI tools/call search_menu with query
echo "Testing Inspector CLI tools/call search_menu with query=biryani..."
if SEARCH_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name search_menu --tool-arg query=biryani 2>&1); then
  if [[ "${SEARCH_OUT}" =~ "Biryani" || "${SEARCH_OUT}" =~ "biryani" || "${SEARCH_OUT}" =~ "rupees" ]]; then
    log_pass "Inspector CLI tools/call search_menu succeeded with rich response"
  else
    log_fail "Inspector CLI tools/call search_menu missing expected text: ${SEARCH_OUT}"
  fi
else
  log_fail "Inspector CLI tools/call search_menu failed execution: ${SEARCH_OUT}"
fi

# Check 9: Inspector CLI tools/call check_availability
echo "Testing Inspector CLI tools/call check_availability..."
AVAIL_DATE=$(node -e 'const d=new Date(); d.setDate(d.getDate()+3); if (d.getDay() === 2) d.setDate(d.getDate()+1); console.log(d.toISOString().slice(0,10));')
if AVAIL_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name check_availability --tool-arg date="${AVAIL_DATE}" --tool-arg time="19:30" --tool-arg party_size="2" 2>&1); then
  if [[ "${AVAIL_OUT}" =~ "available" || "${AVAIL_OUT}" =~ "book" || "${AVAIL_OUT}" =~ "table" ]]; then
    log_pass "Inspector CLI tools/call check_availability succeeded: ${AVAIL_DATE} 19:30"
  else
    log_fail "Inspector CLI tools/call check_availability missing expected response: ${AVAIL_OUT}"
  fi
else
  log_fail "Inspector CLI tools/call check_availability failed execution: ${AVAIL_OUT}"
fi

# Check 10: Create -> Get -> Cancel booking lifecycle on a throwaway slot
echo "Testing booking lifecycle: create_booking -> get_booking -> cancel_booking..."
THROW_DATE=$(node -e 'const d=new Date(); d.setDate(d.getDate()+14); if (d.getDay() === 2) d.setDate(d.getDate()+1); console.log(d.toISOString().slice(0,10));')
IDEMP_KEY="smoke-key-$(node -e 'console.log(Date.now());')"

CREATE_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name create_booking \
  --tool-arg date="${THROW_DATE}" \
  --tool-arg time="19:30" \
  --tool-arg party_size="2" \
  --tool-arg guest_name="Smoke Tester" \
  --tool-arg idempotency_key="${IDEMP_KEY}" 2>&1 || true)

if [[ "${CREATE_OUT}" =~ "confirmed" && "${CREATE_OUT}" =~ "TT-" ]]; then
  log_pass "Inspector CLI create_booking succeeded on ${THROW_DATE} 19:30"
  # Extract code (e.g. TT-XXXXX)
  CONF_CODE=$(echo "${CREATE_OUT}" | grep -oE "TT-[A-Z0-9]{5}" | head -n 1)
  if [[ -n "${CONF_CODE}" ]]; then
    # Get booking
    GET_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name get_booking \
      --tool-arg confirmation_code="${CONF_CODE}" \
      --tool-arg guest_name="Smoke Tester" 2>&1 || true)
    if [[ "${GET_OUT}" =~ "found" && "${GET_OUT}" =~ "${CONF_CODE}" ]]; then
      log_pass "Inspector CLI get_booking found booking ${CONF_CODE}"
    else
      log_fail "Inspector CLI get_booking failed for ${CONF_CODE}: ${GET_OUT}"
    fi

    # Cancel booking
    CANCEL_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name cancel_booking \
      --tool-arg confirmation_code="${CONF_CODE}" \
      --tool-arg guest_name="Smoke Tester" \
      --tool-arg reason="Smoke test cleanup" 2>&1 || true)
    if [[ "${CANCEL_OUT}" =~ "cancelled" ]]; then
      log_pass "Inspector CLI cancel_booking successfully cancelled ${CONF_CODE}"
    else
      log_fail "Inspector CLI cancel_booking failed for ${CONF_CODE}: ${CANCEL_OUT}"
    fi
  else
    log_fail "Could not extract confirmation code from create_booking output"
  fi
else
  log_fail "Inspector CLI create_booking failed: ${CREATE_OUT}"
fi

# Check 11: Place -> Get -> Cancel order lifecycle
echo "Testing pickup order lifecycle: place_order -> get_order -> cancel_order..."
ORDER_IDEMP_KEY="smoke-ord-$(node -e 'console.log(Date.now());')"

ORDER_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name place_order \
  --tool-arg items='[{"item_id":1,"quantity":1,"notes":"Extra crisp"}]' \
  --tool-arg guest_name="Smoke Orderer" \
  --tool-arg idempotency_key="${ORDER_IDEMP_KEY}" 2>&1 || true)

if [[ "${ORDER_OUT}" =~ "placed" && "${ORDER_OUT}" =~ "TO-" ]]; then
  log_pass "Inspector CLI place_order succeeded"
  ORDER_CODE=$(echo "${ORDER_OUT}" | grep -oE "TO-[A-Z0-9]{5}" | head -n 1)
  if [[ -n "${ORDER_CODE}" ]]; then
    # Get order
    GET_ORD_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name get_order \
      --tool-arg order_code="${ORDER_CODE}" \
      --tool-arg guest_name="Smoke Orderer" 2>&1 || true)
    if [[ "${GET_ORD_OUT}" =~ "found" && "${GET_ORD_OUT}" =~ "${ORDER_CODE}" ]]; then
      log_pass "Inspector CLI get_order found order ${ORDER_CODE}"
    else
      log_fail "Inspector CLI get_order failed for ${ORDER_CODE}: ${GET_ORD_OUT}"
    fi

    # Cancel order
    CANCEL_ORD_OUT=$(npx --yes @modelcontextprotocol/inspector --cli "${MCP_URL}" --method tools/call --tool-name cancel_order \
      --tool-arg order_code="${ORDER_CODE}" \
      --tool-arg guest_name="Smoke Orderer" \
      --tool-arg reason="Smoke test cancel" 2>&1 || true)
    if [[ "${CANCEL_ORD_OUT}" =~ "cancelled" ]]; then
      log_pass "Inspector CLI cancel_order successfully cancelled ${ORDER_CODE}"
    else
      log_fail "Inspector CLI cancel_order failed for ${ORDER_CODE}: ${CANCEL_ORD_OUT}"
    fi
  else
    log_fail "Could not extract order code from place_order output"
  fi
else
  # If closed because of current time/day in test environment, verify truthful outcome
  if [[ "${ORDER_OUT}" =~ "closed" || "${ORDER_OUT}" =~ "pickup_too_soon" ]]; then
    log_pass "Inspector CLI place_order returned expected business outcome (${ORDER_OUT})"
  else
    log_fail "Inspector CLI place_order failed: ${ORDER_OUT}"
  fi
fi

echo "--------------------------------------------------"
echo "Summary: ${PASS_COUNT} passed, ${FAIL_COUNT} failed."
echo "=================================================="

if [[ ${FAIL_COUNT} -gt 0 ]]; then
  exit 1
fi

exit 0
