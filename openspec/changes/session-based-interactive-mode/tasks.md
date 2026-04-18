# Tasks: Session-Based Interactive Mode

## Implementation

- [x] Add `allow_unrestricted_sessions: bool` to `ServerConfig` and `RawServerConfig` in `common/src/config.rs`
- [x] Add `SessionOpen`, `SessionClose`, `SessionExpire`, `SessionCommand` to `AuditAction` in `common/src/audit.rs`
- [x] Add `Denied`, `Expired` to `AuditOutcome`
- [x] Add `session_id: Option<String>` to `AuditEvent`
- [x] Create `common/src/session.rs` with `SessionMode`, `SessionRecord`
- [x] Export session types from `common/src/lib.rs`
- [x] Add session error variants to `broker/src/error.rs`
- [x] Create `broker/src/session.rs` with `SessionManager`
- [x] Add `Broker::session_manager()` method in `broker/src/planner.rs`
- [x] Export session types from `broker/src/lib.rs`
- [x] Add `session` subcommand group to `cli/src/main.rs`

## Tests

- [x] TTL expiry detection
- [x] Idle timeout detection
- [x] Unrestricted mode blocked when `allow_unrestricted_sessions = false`
- [x] Unrestricted mode requires approval
- [x] Restricted mode validates profiles
- [x] Session record serialization round-trip
- [x] Session command length limit

## Validation

- [x] `cargo fmt --all`
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] `cargo test --workspace`
