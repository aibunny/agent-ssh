# Codex Subagent Design Findings

## Laplace

- Recommended change id: `legacy-password-auth-compatibility`
- Keep password compatibility explicitly `legacy`, opt-in, and compatibility-only
- Preserve the secure baseline language in the spec and require that the agent never receives plaintext password material
- Update `broker-config`, `cli-surface`, `audit-logging`, and `approval-flow` with legacy-password scenarios

## James

- Introduce the new auth mode through `crates/common/src/config.rs`, not through caller-facing request fields
- Keep `RunRequest` unchanged and keep `RunPlan` limited to an auth label rather than any secret material
- Extend planner, executor, and broker errors for a separate legacy-password execution path
- Add fail-closed tests for missing secret resolution and audit/dry-run redaction

## Integration Decision

The implementation follows those findings by:

- adding a separate OpenSpec change instead of weakening the secure default in place
- keeping secret handling out of alias/profile request inputs
- using approval gating and audit redaction for the compatibility lane
- keeping plaintext password values out of TOML, `.env`, CLI args, audit logs, and review artifacts
