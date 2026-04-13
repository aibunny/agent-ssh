## Why

The repository has drifted in three places that matter for a security-first release: specialist work is durable but not recorded per completed task, install/distribution metadata is still inconsistent and placeholder-heavy, and the broker still contains password-auth execution paths that conflict with the project mission and can trigger fail2ban bans. We need a single change that restores the secure baseline, makes packaging reviewable, and turns Codex/Claude task tracking into an auditable repo artifact.

## What Changes

- Add a durable per-change task journal workflow for Codex, Codex subagents, and Claude Code CLI work, plus repository validation that completed tasks are recorded before they are treated as done.
- Add Linux/macOS installation and release requirements that standardize repository identity, maintainer metadata, install docs, and packaging automation under `aibunny/agent-ssh`.
- Add a transport requirement for system OpenSSH execution that is publickey-only, non-interactive, and fail-closed instead of falling back to password or keyboard-interactive auth.
- Remove password-auth configuration and `sshpass` execution support from the secure release path.
- Update tests, docs, and collaboration guidance so the new workflow and transport posture are explicit and verifiable.

## Capabilities

### New Capabilities
- `task-recording`: Durable per-change task journals and validation for Codex and Claude collaboration work.
- `installation-packaging`: Install docs, package metadata, and release automation for Linux and macOS distribution under the canonical `aibunny/agent-ssh` identity.
- `ssh-transport-execution`: Publickey-only, non-interactive system OpenSSH execution behavior for broker-managed runs.

### Modified Capabilities
- `broker-config`: Secure-release configuration validation now rejects password-auth server settings and other password-oriented SSH fields.

## Impact

- Affects repository process and durable memory under `openspec/`, `docs/`, and `scripts/`.
- Affects install/distribution surfaces in `README.md`, `Cargo.toml`, `crates/*/Cargo.toml`, `Formula/agent-ssh.rb`, `.github/workflows/release.yml`, and `scripts/install.sh`.
- Affects broker config and execution code in `crates/common`, `crates/broker`, and `crates/cli`.
- Adds or updates tests covering secure config parsing, OpenSSH invocation hardening, and task-journal validation.
