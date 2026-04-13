# Codex Subagent Findings

## Helmholtz

- Keep root login as a tiny separate OpenSpec change, not part of the password-compatibility lane.
- Frame it as a break-glass exception that does not relax password or transport rules.
- Mirror the wording in `openspec/project.md`, `openspec/specs/broker-config/spec.md`, `docs/configuration.md`, and `docs/threat-model.md`.

## Franklin

- The smallest safe implementation is a single per-server opt-in, not a CLI switch or runtime override.
- Reject the opt-in flag when the configured user is not `root` so the exception stays explicit.
- No planner or executor changes are needed for the minimal implementation because the validated username already flows through existing code.

## Integration Decision

This change uses `root_login_acknowledged = true` as the explicit break-glass acknowledgment flag. The validator still rejects root by default, allows it only when acknowledged, rejects the acknowledgment flag for non-root users, and leaves transport/approval behavior unchanged.
