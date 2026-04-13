#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <change-id>" >&2
  exit 1
}

if [ "$#" -ne 1 ]; then
  usage
fi

CHANGE_ID="$1"
CHANGE_DIR="openspec/changes/${CHANGE_ID}"
TASKS_FILE="${CHANGE_DIR}/tasks.md"
JOURNAL_FILE="${CHANGE_DIR}/records/task-journal.md"

if [ ! -d "${CHANGE_DIR}" ]; then
  echo "error: change '${CHANGE_ID}' does not exist at ${CHANGE_DIR}" >&2
  exit 1
fi

if [ ! -f "${TASKS_FILE}" ]; then
  echo "error: ${TASKS_FILE} does not exist" >&2
  exit 1
fi

COMPLETED_TASKS="$(sed -nE 's/^- \[[xX]\] ([0-9]+\.[0-9]+)( .*)?$/\1/p' "${TASKS_FILE}")"

if [ -z "${COMPLETED_TASKS}" ]; then
  echo "task journal check passed: ${CHANGE_ID} has no completed tasks"
  exit 0
fi

if [ ! -f "${JOURNAL_FILE}" ]; then
  echo "error: ${JOURNAL_FILE} is missing but ${CHANGE_ID} has completed tasks" >&2
  exit 1
fi

missing=0
while IFS= read -r task_id; do
  [ -n "${task_id}" ] || continue
  if ! grep -Fq "| ${task_id} |" "${JOURNAL_FILE}"; then
    echo "error: completed task ${task_id} is missing from ${JOURNAL_FILE}" >&2
    missing=1
  fi
done <<EOF
${COMPLETED_TASKS}
EOF

if [ "${missing}" -ne 0 ]; then
  exit 1
fi

echo "task journal check passed: ${CHANGE_ID}"
