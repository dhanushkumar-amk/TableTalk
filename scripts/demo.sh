#!/usr/bin/env bash
# scripts/demo.sh
# Starts the TableTalk Axum backend (port 8080) and Vite frontend (port 5173) with LLM_PROVIDER=groq.

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BACKEND_PORT="${PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-5173}"
export LLM_PROVIDER="groq"

BACKEND_PID=""
FRONTEND_PID=""

cleanup() {
  echo ""
  echo "==> Shutting down TableTalk demo services..."
  if [[ -n "${FRONTEND_PID:-}" ]]; then
    kill "${FRONTEND_PID}" 2>/dev/null || true
  fi
  if [[ -n "${BACKEND_PID:-}" ]]; then
    kill "${BACKEND_PID}" 2>/dev/null || true
  fi
  echo "==> TableTalk demo services stopped."
}
trap cleanup EXIT INT TERM

echo "=========================================================="
echo " TableTalk Demo Launcher (Groq Orchestration Mode)"
echo " LLM_PROVIDER: ${LLM_PROVIDER}"
echo " Backend:     http://127.0.0.1:${BACKEND_PORT}"
echo " Frontend:    http://127.0.0.1:${FRONTEND_PORT}/assistant"
echo "=========================================================="

# 1. Start Axum Backend
echo "--> Starting TableTalk Axum backend on port ${BACKEND_PORT}..."
(
  cd "${ROOT_DIR}/backend"
  PORT="${BACKEND_PORT}" LLM_PROVIDER="groq" RUST_LOG="info" cargo run --bin mcp_tabletalk_backend
) &
BACKEND_PID=$!

# Wait for backend health
echo "--> Waiting for backend /health at http://127.0.0.1:${BACKEND_PORT}/health..."
MAX_WAIT=30
WAITED=0
READY=0
while [[ ${WAITED} -lt ${MAX_WAIT} ]]; do
  if curl -sf "http://127.0.0.1:${BACKEND_PORT}/health" >/dev/null 2>&1; then
    READY=1
    break
  fi
  sleep 1
  WAITED=$((WAITED + 1))
done

if [[ ${READY} -ne 1 ]]; then
  echo "ERROR: Backend failed to start within ${MAX_WAIT} seconds."
  exit 1
fi
echo "==> Backend is healthy and ready!"

# 2. Start Vite Frontend
echo "--> Starting TableTalk Vite frontend on port ${FRONTEND_PORT}..."
(
  cd "${ROOT_DIR}/frontend"
  npm run dev -- --port "${FRONTEND_PORT}" --host
) &
FRONTEND_PID=$!

echo ""
echo "=========================================================="
echo " TableTalk Live Demo is Running!"
echo " Open in your browser: http://127.0.0.1:${FRONTEND_PORT}/assistant"
echo " Press Ctrl+C to terminate both servers."
echo "=========================================================="

wait
