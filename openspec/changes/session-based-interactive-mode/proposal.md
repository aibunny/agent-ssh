## Why

The current `exec` command issues a new SSH connection per command. AI agents performing multi-step workflows — diagnosing an incident, following a runbook, iterating on a deploy — must pay the authentication overhead for every command and cannot preserve shell state across steps. A broker-controlled session layer fixes this while maintaining full audit coverage and zero credential exposure.

## What Changes

- Add `allow_unrestricted_sessions` flag to `ServerConfig` (opt-in per server).
- Extend `AuditAction` with session lifecycle and command events.
- Extend `AuditOutcome` with `denied` and `expired` outcomes.
- Add `session_id` field to `AuditEvent`.
- Add `SessionManager` to the broker crate with open / exec / close / list operations.
- Add `agent-ssh session` subcommand group to the CLI.

## Capabilities

### New Capabilities
- `session-management`: Broker-held SSH ControlMaster sessions with TTL, idle timeout, and per-session audit trail.
- `unrestricted-session-mode`: Explicit opt-in for agents to run arbitrary commands inside a policy-gated, approval-required session.

### Modified Capabilities
- `broker-config`: ServerConfig gains `allow_unrestricted_sessions` flag.
- `audit-logging`: New session-lifecycle and session-command event types.
- `cli-surface`: New `session` subcommand group.

## Impact

- Backwards-compatible config change (flag defaults to `false`).
- New CLI surface; existing `exec` command unchanged.
- SSH ControlMaster sockets placed in `/tmp/agent-ssh-<id8>.sock` (short path, under Unix socket limit).
- Session records persisted as JSON in `<data_dir>/sessions/<id>.json`.
