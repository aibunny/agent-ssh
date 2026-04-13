#!/usr/bin/env bash
set -euo pipefail

for change_dir in openspec/changes/*; do
  [ -d "${change_dir}" ] || continue

  change_id="$(basename "${change_dir}")"
  if [ "${change_id}" = "archive" ]; then
    continue
  fi

  if [ ! -f "${change_dir}/tasks.md" ]; then
    continue
  fi

  echo "Validating OpenSpec change: ${change_id}"
  openspec validate "${change_id}"

  echo "Checking task journal: ${change_id}"
  ./scripts/check-task-journal.sh "${change_id}"
done
