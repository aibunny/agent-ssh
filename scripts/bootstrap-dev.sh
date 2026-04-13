#!/usr/bin/env bash
set -euo pipefail

echo "Formatting workspace"
cargo fmt --all

echo "Running clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "Running tests"
cargo test --workspace --all-features

echo "OpenSpec validation"
openspec validate secure-broker-foundation
openspec validate installability-task-recording-and-fail2ban-safety

echo "Task journal validation"
./scripts/check-task-journal.sh installability-task-recording-and-fail2ban-safety
