# agent-ssh Project Context

## Mission

Build a highly secure Rust-based SSH broker that lets agentic AI systems access remote servers without ever receiving passwords or long-lived reusable SSH private keys.

## Product Constraints

- Broker implementation MUST be Rust only.
- Configuration MUST be TOML only.
- The first secure release MUST use named server aliases and named command profiles instead of arbitrary shell execution.
- The broker MUST default to deny on ambiguous, missing, or unsafe input.
- The broker MUST never rely on password authentication. Root login MAY be used only as an explicit break-glass exception, remains strongly discouraged, and must not change the default certificate-first policy.
- The broker MUST be designed around short-lived SSH certificates and broker-owned execution.
- System OpenSSH is the initial transport target.

## Repository Conventions

- Long-lived capability truth lives in `openspec/specs/`.
- Proposed changes live in `openspec/changes/<change-id>/`.
- Meaningful implementation work starts from an OpenSpec proposal, then design/spec deltas/tasks, then code.
- Security-sensitive decisions belong in OpenSpec design docs and repo docs, not only in chat history.
- Durable collaboration notes for specialist reviews should live under the active OpenSpec change folder.
- Each active change that completes tasks MUST keep `records/task-journal.md`, and completed tasks in `tasks.md` MUST be recorded there before they are considered done.

## Architecture Conventions

- `crates/common`: shared domain types, configuration parsing, validation, and errors.
- `crates/broker`: policy evaluation, alias resolution, profile rendering, signer abstraction, approvals, and audit orchestration.
- `crates/cli`: end-user CLI surface.
- `crates/mcp`: future agent-facing interface layer.

## Security Conventions

- Exact alias matching only; no fuzzy lookup.
- No silent fallback from protected behavior to less secure behavior.
- Reject unsafe command composition before execution.
- Minimize lifetime and filesystem exposure of credential material.
- Audit decisions, rendered commands, and blocked actions.
- Keep security reviews tied to OpenSpec changes so they remain reviewable in git history.

## Collaboration Model

- Codex is the managing agent and execution lead.
- Claude Code CLI is a scoped specialist collaborator for design review, threat review, validation, and bounded implementation or verification tasks.
- Delegated Claude tasks MUST reference the active OpenSpec change ID and acceptance criteria.
- Codex, Codex subagents, and Claude Code CLI MUST record completed change tasks with files touched and verification context in the active change journal.
