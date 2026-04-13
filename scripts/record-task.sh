#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <change-id> <task-id> <agent> <summary> [files] [verification] [artifacts]" >&2
  exit 1
}

if [ "$#" -lt 4 ]; then
  usage
fi

CHANGE_ID="$1"
TASK_ID="$2"
AGENT="$3"
SUMMARY="$4"
FILES="${5:--}"
VERIFICATION="${6:--}"
ARTIFACTS="${7:--}"

CHANGE_DIR="openspec/changes/${CHANGE_ID}"
TASKS_FILE="${CHANGE_DIR}/tasks.md"
JOURNAL_DIR="${CHANGE_DIR}/records"
JOURNAL_FILE="${JOURNAL_DIR}/task-journal.md"

if [ ! -d "${CHANGE_DIR}" ]; then
  echo "error: change '${CHANGE_ID}' does not exist at ${CHANGE_DIR}" >&2
  exit 1
fi

if [ ! -f "${TASKS_FILE}" ]; then
  echo "error: ${TASKS_FILE} does not exist" >&2
  exit 1
fi

if ! grep -Eq "^- \[[ xX]\] ${TASK_ID}\b" "${TASKS_FILE}"; then
  echo "error: task '${TASK_ID}' was not found in ${TASKS_FILE}" >&2
  exit 1
fi

sanitize_cell() {
  printf '%s' "$1" \
    | tr '\n' ' ' \
    | sed -E 's/[[:space:]]+/ /g; s/^ //; s/ $//; s/\|/\\|/g'
}

mkdir -p "${JOURNAL_DIR}"

if [ ! -f "${JOURNAL_FILE}" ]; then
  {
    echo "# Task Journal"
    echo
    echo "Record one append-only row for each completed task. Do not delete or rewrite prior rows; add a new row if follow-up work is needed for the same task."
    echo
    echo "| Timestamp (UTC) | Task | Agent | Summary | Files | Verification | Artifacts |"
    echo "| --- | --- | --- | --- | --- | --- | --- |"
  } > "${JOURNAL_FILE}"
fi

TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

printf '| %s | %s | %s | %s | %s | %s | %s |\n' \
  "$(sanitize_cell "${TIMESTAMP}")" \
  "$(sanitize_cell "${TASK_ID}")" \
  "$(sanitize_cell "${AGENT}")" \
  "$(sanitize_cell "${SUMMARY}")" \
  "$(sanitize_cell "${FILES}")" \
  "$(sanitize_cell "${VERIFICATION}")" \
  "$(sanitize_cell "${ARTIFACTS}")" \
  >> "${JOURNAL_FILE}"

echo "recorded ${CHANGE_ID} task ${TASK_ID} in ${JOURNAL_FILE}"
