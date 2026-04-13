## 1. OpenSpec And Task Recording

- [x] 1.1 Author the proposal, design, and spec deltas for task recording, installability, and fail2ban-safe SSH transport
- [x] 1.2 Add a change-local task journal and document the rule that Codex and Claude work must be recorded before a task is marked complete

## 2. Collaboration Tooling

- [x] 2.1 Implement repository scripts to append task-journal entries and validate journal coverage for completed OpenSpec tasks
- [x] 2.2 Update project and collaboration docs to require journal-backed task completion for Codex, Codex subagents, and Claude Code CLI

## 3. SSH Transport Hardening

- [x] 3.1 Remove password-auth configuration and execution support from the secure release path, including docs and starter config
- [x] 3.2 Harden system OpenSSH invocation to publickey-only non-interactive execution and add regression tests for the new flags and fail-closed behavior

## 4. Installability And Packaging

- [x] 4.1 Standardize repository/package metadata and install docs under `aibunny/agent-ssh`
- [x] 4.2 Fix installer behavior, Homebrew formula metadata, and release automation so Linux/macOS distribution artifacts stay aligned

## 5. Specialist Review And Verification

- [ ] 5.1 Assign Claude Code CLI a bounded review of this change and record the findings under the change artifacts
- [x] 5.2 Run formatting, linting, tests, OpenSpec validation, and task-journal validation, then record the verification results in the journal
