## 1. OpenSpec And Review Grounding

- [x] 1.1 Author the proposal, design, tasks, task journal, and spec deltas for the legacy password compatibility lane before code changes
- [x] 1.2 Record Codex and Codex subagent design findings for this change in the task journal

## 2. Config And Policy

- [x] 2.1 Extend broker config parsing and validation for explicit `legacy_password` auth, opaque secret-reference env vars, and fail2ban acknowledgment flags
- [x] 2.2 Update planning, approval, and audit metadata so legacy password runs are approval-gated and redacted

## 3. Runtime Secret Resolution And Execution

- [x] 3.1 Add runtime secret-reference loading from process env plus sibling `.env` without accepting plaintext password values
- [x] 3.2 Implement the broker-managed askpass execution path for Linux and macOS and keep dry-run output secret-free

## 4. Docs And Tests

- [x] 4.1 Add tests covering config validation, approval behavior, `.env` secret-reference loading, and redacted execution planning
- [x] 4.2 Update README, configuration docs, threat model, and example config to document legacy password compatibility as an opt-in migration path

## 5. Specialist Review And Verification

- [ ] 5.1 Assign Claude Code CLI a bounded review of the new legacy-password design/implementation and record the outcome under this change
- [x] 5.2 Run formatting, linting, tests, OpenSpec validation, and task-journal validation, then record the verification results
