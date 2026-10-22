#!/usr/bin/env bash
# scripts/backup.sh
# Production SQLite database backup utility for TableTalk
# Creates atomic point-in-time snapshot with integrity verification.

set -euo pipefail

DB_PATH="${1:-data/tabletalk.db}"
BACKUP_DIR="${2:-backups}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
BACKUP_FILE="${BACKUP_DIR}/tabletalk_${TIMESTAMP}.db"

echo "=================================================="
echo " TableTalk Database Backup Utility"
echo "=================================================="
echo "Source DB:   ${DB_PATH}"
echo "Backup Dir:  ${BACKUP_DIR}"
echo "Target File: ${BACKUP_FILE}"
echo "--------------------------------------------------"

if [ ! -f "${DB_PATH}" ]; then
  echo "Error: Source database file '${DB_PATH}' not found." >&2
  exit 1
fi

mkdir -p "${BACKUP_DIR}"

if command -v sqlite3 >/dev/null 2>&1; then
  echo "Running online SQLite VACUUM INTO backup..."
  sqlite3 "${DB_PATH}" "VACUUM INTO '${BACKUP_FILE}';"
else
  echo "Notice: sqlite3 CLI not found, performing filesystem snapshot..."
  cp "${DB_PATH}" "${BACKUP_FILE}"
  if [ -f "${DB_PATH}-wal" ]; then
    cp "${DB_PATH}-wal" "${BACKUP_FILE}-wal" || true
  fi
fi

if [ ! -s "${BACKUP_FILE}" ]; then
  echo "Error: Backup file is empty or missing: ${BACKUP_FILE}" >&2
  exit 1
fi

if command -v sqlite3 >/dev/null 2>&1; then
  echo "Verifying backup integrity with PRAGMA quick_check..."
  INTEGRITY=$(sqlite3 "${BACKUP_FILE}" "PRAGMA quick_check;")
  if [ "${INTEGRITY}" != "ok" ]; then
    echo "Error: Backup integrity check failed: ${INTEGRITY}" >&2
    exit 1
  fi
  echo "Integrity check: OK"
fi

BACKUP_SIZE=$(ls -lh "${BACKUP_FILE}" | awk '{print $5}')
echo "--------------------------------------------------"
echo "Backup successfully created: ${BACKUP_FILE} (${BACKUP_SIZE})"
echo "=================================================="
