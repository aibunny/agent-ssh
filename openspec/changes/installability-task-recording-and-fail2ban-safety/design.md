## Context

`agent-ssh` already has an OpenSpec foundation, but the active implementation drifted from that foundation in ways that weaken trust. The repository still contains `<owner>` placeholders across install metadata, the release automation and Homebrew formula are inconsistent, and the broker supports `auth_method = "password"` plus `sshpass`, which violates the mission of never handing agents reusable passwords and increases the chance of remote fail2ban bans.

At the same time, collaboration memory is only partially durable: review notes live in change folders, but there is no per-task journal that proves which agent completed which task, what changed, or how it was verified. The user also wants Codex and Claude work to be recorded every time, not only when a specialist review happens to be written down.

This design is informed by the current OpenSSH and fail2ban guidance:

- OpenSSH `BatchMode` disables interactive prompting, `PreferredAuthentications` controls auth ordering, and `PasswordAuthentication`, `KbdInteractiveAuthentication`, and `NumberOfPasswordPrompts` can explicitly disable password-style fallbacks.
- OpenSSH `IdentitiesOnly` limits which identities the client offers, reducing accidental auth spray from a crowded local agent.
- Fail2ban's `ignoreip` setting is a server-side allowlist; the broker can reduce ban-triggering behavior, but it cannot unilaterally reconfigure remote fail2ban instances.

## Goals / Non-Goals

**Goals:**

- Make completed OpenSpec tasks durably recorded for Codex, Codex subagents, and Claude Code CLI work.
- Add a lightweight, machine-checkable repository rule that fails when checked tasks are missing journal coverage.
- Standardize install/distribution metadata and docs under the `aibunny/agent-ssh` identity for Linux and macOS users.
- Remove password-auth and `sshpass` support from the secure release path.
- Harden the system OpenSSH invocation so execution is publickey-only, non-interactive, and fail-closed.
- Add docs and tests that make the new task-recording and fail2ban-safe behavior explicit.

**Non-Goals:**

- Implementing the full signer-backed short-lived certificate transport in this change.
- Managing remote fail2ban configuration directly from the broker.
- Renaming every crate package in the workspace just to make Cargo install syntax prettier.
- Expanding the product surface beyond task journaling, installability cleanup, and transport hardening.

## Decisions

### 1. Use a per-change append-only task journal plus validation scripts

Each active change will keep `records/task-journal.md` as the durable ledger for task execution. Every completed task ID in `tasks.md` must have at least one journal entry recording:

- task ID
- UTC timestamp
- agent identity (`Codex`, a named Codex subagent, or `Claude Code CLI`)
- summary of work
- files/modules touched
- verification command or review summary
- linked artifact or review note when one exists

Two repository scripts will support this flow:

- `scripts/record-task.sh` to append standardized entries
- `scripts/check-task-journal.sh` to fail if checked tasks are missing journal coverage

Alternatives considered:

- Keep relying on chat history and ad hoc review notes. Rejected because the user asked for always-recorded work and chat is not durable project memory.
- Store task history only inside `tasks.md`. Rejected because it overloads checkbox state and makes verification/provenance hard to scan.

### 2. Keep repository identity canonical in workspace metadata and distribution surfaces

The canonical public identity for this project will be `aibunny/agent-ssh`. Root Cargo workspace metadata will carry `authors`, `homepage`, and `repository`, and install/distribution files will reuse that identity instead of hard-coded placeholders.

The CLI crate package name will remain `agent-ssh-cli` for now. Install docs and release notes will be corrected to use the real package name rather than renaming the crate mid-change. This keeps the change focused on correctness and security instead of package-churn.

Alternatives considered:

- Rename the CLI package to `agent-ssh` in the same change. Rejected because it broadens the blast radius without being required to make installs correct.
- Leave metadata duplicated file by file. Rejected because it makes future drift more likely.

### 3. Fix release automation by making the formula update deterministic

The Homebrew formula will use explicit `version` metadata and stable placeholder names that match the release workflow exactly. The formula-update job will check out the writable branch before committing so checksum/version updates can be pushed reliably after a tagged release.

Alternatives considered:

- Keep inferring version from URLs and mutate whatever text happens to match. Rejected because the current workflow already demonstrates how fragile that is.
- Skip automatic formula updates. Rejected because the repository is intended to be installable directly from release artifacts.

### 4. Remove password auth from config and execution instead of trying to “safely” keep it

The secure release will no longer accept `auth_method = "password"` or `password_env_var`. The broker will keep an auth-method label model, but the only supported mode in this change is certificate/publickey-oriented execution. `sshpass` support and password-oriented execution errors/tests will be removed.

Alternatives considered:

- Keep password auth behind warnings. Rejected because the project mission and existing OpenSpec constraints explicitly forbid password authentication.
- Keep password auth only for “legacy” examples. Rejected because examples become de facto support signals for insecure behavior.

### 5. Harden system OpenSSH to publickey-only, non-interactive invocation

The executor will build ssh commands with a fail-closed option set:

- `BatchMode=yes`
- `PreferredAuthentications=publickey`
- `PubkeyAuthentication=yes`
- `PasswordAuthentication=no`
- `KbdInteractiveAuthentication=no`
- `NumberOfPasswordPrompts=0`
- `IdentitiesOnly=yes`
- `ConnectTimeout=30`
- `StrictHostKeyChecking=accept-new`

This combination eliminates password and keyboard-interactive fallback, avoids hanging on prompts, and reduces accidental multi-identity offering. We will not add `IdentityAgent=none` in this change because the broker does not yet hand explicit signer material into the executor; forcing the agent fully off now would turn every current execution path into an unconditional auth failure before the signer work lands.

Alternatives considered:

- Leave auth selection to the local ssh default stack. Rejected because it can attempt password-style fallbacks and widen fail2ban exposure.
- Disable the local agent entirely now. Rejected for this change because it would break current execution without simultaneously adding signer-backed identity injection.

### 6. Treat fail2ban safety as client-side minimization plus operator guidance

The broker can materially reduce ban-triggering behavior by removing password auth and prompt-based fallback, but it cannot guarantee that a remote fail2ban deployment will never ban a client IP. The docs will therefore make the contract explicit:

- the broker avoids password and keyboard-interactive auth attempts
- operators should allowlist known broker egress IPs/CIDRs in fail2ban `ignoreip` when their environment requires that guarantee

Alternatives considered:

- Claim that the hardened ssh flags alone “ensure” fail2ban immunity. Rejected because remote policy remains outside the broker’s control.

## Risks / Trade-offs

- [Task-journal validation checks coverage, not prose quality] → Keep the schema small and machine-checkable, then review entry quality in PRs.
- [Keeping `agent-ssh-cli` as the Cargo package name makes Cargo install commands less pretty] → Document the exact package name now and revisit a rename only if distribution strategy changes.
- [Publickey-only SSH options can break environments that were relying on password fallback] → This is intentional fail-closed behavior and will be called out in docs and config validation.
- [Remote fail2ban rules may still ban repeated publickey failures] → Keep retries minimal, remove password paths, and document `ignoreip` allowlisting for broker egress IPs.
- [Formula automation remains release-environment sensitive] → Make placeholder mapping explicit and validate the generated formula in CI/release verification.

## Migration Plan

1. Author proposal, design, tasks, and spec deltas for task recording, installability, and transport hardening.
2. Add the task-journal file plus recording/validation scripts and update collaboration guidance.
3. Update repository/package metadata, install docs, installer behavior, and release automation under `aibunny/agent-ssh`.
4. Remove password-auth config/execution support and harden the ssh invocation plus tests/docs.
5. Run a bounded Claude Code CLI review against this change, record the findings under the change, and verify the repository state.

Rollback is straightforward because the repository is still pre-release: revert the change artifacts, scripts, docs, and code changes together.

## Open Questions

- Once signer-backed session material lands, should the executor require explicit `IdentityFile`/`CertificateFile` and disable agent access entirely?
- Do we want a future CI job that smoke-tests generated release metadata (formula/install script/release notes) without waiting for a tag push?
