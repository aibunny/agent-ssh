## Why

`agent-ssh` is starting as a security-sensitive greenfield project, so we need a durable spec and design backbone before implementation hardens the wrong interfaces. The foundation change establishes the broker’s source-of-truth capabilities, secure configuration model, and initial Rust workspace so Codex and Claude Code CLI can coordinate against the same reviewable artifacts.

## What Changes

- Introduce source-of-truth OpenSpec capability specs for configuration, alias resolution, profile execution, signing, auditing, approvals, and the CLI surface.
- Define the secure broker foundation in proposal, design, tasks, and spec deltas before implementation.
- Scaffold the Rust workspace and repository structure for `common`, `broker`, `cli`, and `mcp` crates.
- Implement alias-aware TOML configuration parsing and validation with default-deny security checks.
- Implement exact alias resolution, safe named-profile rendering, and append-only JSONL audit logging for broker decisions.
- Add the minimum CLI surface for config validation, host listing, profile listing, and run request planning.
- Record one scoped Claude Code CLI review task under this change and verify its findings against acceptance criteria.

## Capabilities

### New Capabilities
- `broker-config`: Secure TOML configuration loading and validation for broker settings, signers, servers, and profiles.
- `server-alias-resolution`: Exact alias lookup and multi-server resolution behavior.
- `command-profile-execution`: Named profile policy enforcement and safe command rendering.
- `signer-abstraction`: Broker-facing abstraction for short-lived SSH certificate issuance.
- `audit-logging`: Structured append-only audit records for broker actions.
- `approval-flow`: Explicit approval gating for protected actions.
- `cli-surface`: Minimum command-line interface for validation, inspection, and run requests.

### Modified Capabilities
- None.

## Impact

- Affects repository planning state under `openspec/`.
- Adds foundational docs under `docs/` and user-facing examples under `configs/`, `examples/`, and `scripts/`.
- Introduces the Rust workspace and initial code under `crates/common`, `crates/broker`, `crates/cli`, and `crates/mcp`.
- Adds build, lint, and test entry points for the first milestone.
