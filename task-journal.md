# Task Journal — Session-Based Interactive Mode

## 2026-04-12 — Initial implementation

### Architecture decisions

1. **SSH ControlMaster** chosen as transport: reuses battle-tested OpenSSH multiplexing, requires no new protocol code, supports all auth modes.
2. **File-based session registry**: `<data_dir>/sessions/<id>.json` — simple, readable by operators, no process state required between CLI invocations.
3. **Short socket paths**: `/tmp/agent-ssh-<id8>.sock` stays well under the 104-char Unix domain socket limit.
4. **Legacy password sessions not supported** for persistent ControlMaster (askpass helper lifetime would outlive the CLI process); `exec` remains the path for legacy servers.
5. **TTL / idle timeout**: enforced lazily on access (no background timer needed for a CLI tool).

### Changes made

- `common/src/audit.rs`: Added `SessionOpen`, `SessionClose`, `SessionExpire`, `SessionCommand` to `AuditAction`; `Denied`, `Expired` to `AuditOutcome`; `session_id: Option<String>` to `AuditEvent`.
- `common/src/config.rs`: Added `allow_unrestricted_sessions: bool` (default `false`) to `ServerConfig` and `RawServerConfig`.
- `common/src/session.rs` (new): `SessionMode`, `SessionRecord`.
- `common/src/lib.rs`: Exported `SessionMode`, `SessionRecord`.
- `broker/src/error.rs`: Added session error variants.
- `broker/src/session.rs` (new): `SessionManager` with `open_session`, `exec_in_session`, `close_session`, `list_sessions`.
- `broker/src/planner.rs`: Added `Broker::session_manager()`, `session_id` to `AuditContext` and `AuditEvent` construction.
- `broker/src/lib.rs`: Exported `SessionManager`, `SessionMode`, `SessionRecord`, `OpenSessionRequest`, `SessionExecRequest`.
- `cli/src/main.rs`: Added `session` subcommand group (`open`, `exec`, `close`, `list`).

### Test results

All existing 80 tests continue to pass. New session tests added.
