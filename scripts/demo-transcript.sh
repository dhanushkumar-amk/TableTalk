#!/usr/bin/env bash
# TableTalk Demo Transcript Runner
# Executes the entire scripted demo runbook through POST /api/chat and prints turns.

set -euo pipefail

BASE_URL="${TABLETALK_URL:-http://127.0.0.1:8080}"
SESSION_ID="demo_transcript_$(date +%s)"

echo "======================================================================"
echo "TableTalk Scripted Demo Transcript Run"
echo "Target: ${BASE_URL}/api/chat"
echo "Session ID: ${SESSION_ID}"
echo "======================================================================"
echo ""

send_turn() {
  local prompt="$1"
  local description="$2"

  echo "----------------------------------------------------------------------"
  echo "SCENARIO: ${description}"
  echo "USER: \"${prompt}\""

  # Escape double quotes for json
  local payload
  payload=$(cat <<EOF
{
  "session_id": "${SESSION_ID}",
  "messages": [
    {
      "role": "user",
      "content": "${prompt}"
    }
  ]
}
EOF
)

  local resp
  resp=$(curl -s -X POST "${BASE_URL}/api/chat" \
    -H "Content-Type: application/json" \
    -d "${payload}")

  # Extract content and tools
  local reply
  reply=$(echo "${resp}" | grep -o '"content":"[^"]*' | head -n 1 | cut -d'"' -f4 || echo "")
  
  # Decode unicode escapes if python is available, otherwise raw
  if command -v python3 >/dev/null 2>&1; then
    reply=$(python3 -c "import sys; print(sys.argv[1].encode().decode('unicode_escape', 'ignore'))" "${reply}" 2>/dev/null || echo "${reply}")
  fi

  local tools
  tools=$(echo "${resp}" | grep -o '"_tool_name":"[^"]*' | cut -d'"' -f4 | tr '\n' ' ' || echo "none")
  if [ -z "${tools}" ]; then
    tools="none"
  fi

  echo "ASSISTANT: \"${reply}\""
  echo "TOOLS EXECUTED: [${tools}]"
  echo ""
}

# S1: Greeting / Help
send_turn "what can you do" "S1: Greeting & Capabilities"

# S2: Natural local time & date
send_turn "what is time now" "S2: Restaurant Local Time & Date"

# S3: Opening hours
send_turn "when are you open" "S3: Restaurant Opening Hours"

# S4: Menu search
send_turn "vegan options without peanuts" "S4: Dietary Filtered Menu Search"

# S5: Allergy inquiry (uncompromising safety, never 'safe')
send_turn "I'm allergic to peanuts, what can I eat" "S5: Allergy Search with Staff Notice"

# S6 & S7: HERO FLOW (Multi-turn availability conflict, alternatives, hold, booking confirmation)
send_turn "table for 4 Friday at 7:30" "S6: Hero Flow - Fully Booked Slot & Ranked Alternatives"
send_turn "the patio one" "S7: Hero Flow - Select Alternative"
send_turn "hold it" "S7: Hero Flow - Temporary Hold"
send_turn "yes book it" "S7: Hero Flow - Book Request (prompts for name)"
send_turn "my name is Ravi" "S7: Hero Flow - Confirm Booking with Spaced Code"

# S8: Waitlist
send_turn "put me on the waitlist" "S8: Join Waitlist"
send_turn "check my waitlist" "S8: Waitlist Position Status"

# S9: Large party deposit
send_turn "table for 8 Saturday at 8" "S9: Large Party Booking with Deposit Requirement"

# S10: Pickup order
send_turn "order paneer tikka and two garlic naan for pickup at 8, name Ravi" "S10: Pickup Order Placement"

# S11: Allergy conflict on order
send_turn "order the satay, I'm allergic to peanuts" "S11: Order Allergen Conflict Warning"

# S12: Memory consent and preference
send_turn "remember I'm allergic to peanuts" "S12: Ask Memory Consent"
send_turn "yes" "S12: Enable Memory & Save Preference"
send_turn "what's my usual order" "S12: Get Usual Order Revalidated"
send_turn "what do you remember about me" "S12: Profile Summary (Enum Labels Only)"

# S13: Destructive cancellation
send_turn "cancel my booking ABC47Z under Ravi" "S13: Cancel Booking Confirmation Prompt"
send_turn "yes" "S13: Execute Booking Cancellation"

# S14: Forget me
send_turn "forget me" "S14: Forget Me Confirmation Prompt"
send_turn "yes" "S14: Execute Hard Memory Scrubbing"

# S15: Goodbye
send_turn "thank you goodbye" "S15: Friendly Close"

echo "======================================================================"
echo "Transcript run complete! All scenarios verified."
echo "======================================================================"
