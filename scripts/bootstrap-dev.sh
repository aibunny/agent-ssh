#!/usr/bin/env bash
set -euo pipefail

echo "Checking formatting"
cargo fmt --all --check

echo "Running clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "Running tests"
cargo test --workspace --all-features

echo "OpenSpec validation"
./scripts/validate-openspec.sh
