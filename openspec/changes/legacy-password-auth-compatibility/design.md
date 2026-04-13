## Context

The repository baseline is still correct: the first secure release should not normalize password authentication. At the same time, some environments need a temporary bridge while operators migrate hosts toward certificate-based access. The user’s condition is narrow and important: the agent may trigger the broker, but it must never receive the password in raw form.

That rules out:

- plaintext passwords in TOML
- plaintext passwords in `.env`
- plaintext passwords in CLI flags
- password values in audit logs, task journals, or review notes
- silent fallback from certificate auth to password auth

This design is informed by current OpenSSH behavior:

- `SSH_ASKPASS` and `SSH_ASKPASS_REQUIRE` allow non-interactive password entry without placing the password on the ssh command line.
- `PreferredAuthentications`, `PasswordAuthentication`, `PubkeyAuthentication`, `KbdInteractiveAuthentication`, and `NumberOfPasswordPrompts` let the client fail closed instead of spraying multiple auth methods.
- Fail2ban allowlisting remains a server-side responsibility; the broker can reduce risk, but it cannot guarantee remote jails will never ban an IP without operator configuration.

## Goals / Non-Goals

**Goals:**

- Preserve certificate auth as the default and recommended path.
- Add an explicit `legacy_password` mode for migration-only servers.
- Keep raw password material out of config files, CLI arguments, audit logs, and dry-run output.
- Support Linux and macOS secret lookup through OS-native secret stores.
- Allow `.env` usage only for opaque secret-reference variables.
- Fail closed on missing approvals, missing secret references, malformed secret references, missing helper tooling, or SSH auth failure.

**Non-Goals:**

- Reintroducing plaintext password support in `.env` or TOML.
- Reintroducing `sshpass` or any argv/file-based password injection.
- Guaranteeing immunity from remote fail2ban policies.
- Expanding legacy password support into the future default path.

## Decisions

### 1. Legacy password auth is a separate opt-in auth method

Servers may declare `auth_method = "legacy_password"` only when all of the following are true:

- `password_secret_ref_env_var` is set
- `legacy_password_acknowledged = true`
- `fail2ban_allowlist_confirmed = true`

The secure default remains `auth_method = "certificate"`.

This keeps the legacy lane explicit in both config and review history.

### 2. `.env` may hold only secret references, never raw passwords

The broker will accept a secret reference through an environment variable whose name is configured in TOML. The CLI will populate its runtime secret map from:

1. the current process environment
2. a sibling `.env` file next to the resolved config path, without overriding existing process env values

The variable value is an opaque reference, not the password. Initial supported format:

- `os_keychain:<service>:<account>`

The service and account identifiers are not secrets, but they still use a conservative character set so the broker can safely compose an askpass helper without shell injection risk.

Alternatives considered:

- Raw password in `.env`. Rejected because it is still plaintext-at-rest and visible to the agent/runtime.
- Raw password in process env only. Rejected because it still exposes plaintext in the local process surface.

### 3. The broker uses a one-shot askpass helper instead of argv or file-based password injection

For `legacy_password`, the executor will create a secure temporary askpass script and point OpenSSH at it using:

- `SSH_ASKPASS=<temp helper path>`
- `SSH_ASKPASS_REQUIRE=force`
- `DISPLAY=agent-ssh:0`

The helper script contains only the non-secret secret-store reference and retrieves the password from the OS-native store at execution time:

- macOS: `security find-generic-password -w -s <service> -a <account>`
- Linux: `secret-tool lookup service <service> account <account>`

This keeps the password out of:

- ssh argv
- broker logs
- `.env`
- TOML

Alternatives considered:

- `sshpass -e`, `sshpass -f`, or `sshpass -p`. Rejected because they move plaintext into env, disk, or argv.
- Having the broker read the password into Rust strings before spawning ssh. Rejected because the user asked to keep raw format out of the agent-facing runtime surface when authoritative alternatives exist.

### 4. Legacy password mode is always approval-gated

Any `legacy_password` run requires approval even if the server and profile do not otherwise require it. This treats password compatibility as a higher-risk exception path.

### 5. Audit output records only the auth kind

Audit events and run plans may record `auth_method_kind = "legacy_password"`, but they must not record:

- raw password material
- secret-reference env var names
- secret reference values

### 6. Fail2ban safety is explicit, not implied

The config must include `fail2ban_allowlist_confirmed = true` for `legacy_password` servers. This is an operator acknowledgment, not a technical guarantee. The broker still uses a single-password-attempt SSH configuration:

- `BatchMode=no`
- `PreferredAuthentications=password`
- `PasswordAuthentication=yes`
- `PubkeyAuthentication=no`
- `KbdInteractiveAuthentication=no`
- `NumberOfPasswordPrompts=1`
- `ConnectTimeout=30`
- `StrictHostKeyChecking=accept-new`

This keeps behavior predictable and avoids multi-method auth spray.

## Risks / Trade-offs

- [Legacy password auth still weakens the overall security posture] → Keep it opt-in, approval-gated, and visibly documented as compatibility-only.
- [Linux hosts may not have `secret-tool` installed] → Fail closed with a clear error and document the dependency.
- [The broker still launches a helper script] → Use secure temp files, a conservative secret-reference grammar, and no password content in the script itself.
- [Remote fail2ban thresholds remain outside broker control] → Require explicit operator acknowledgment and continue documenting allowlisting guidance.

## Migration Plan

1. Author the OpenSpec proposal, design, tasks, and spec deltas for this compatibility lane.
2. Extend config validation and approval semantics to support explicit `legacy_password`.
3. Add runtime secret-reference loading from process env plus sibling `.env`.
4. Implement the askpass-based execution path and redaction-safe dry-run output.
5. Add tests, update docs/examples, and run specialist review/verification.

## Open Questions

- Should a future change support additional secret-reference backends beyond OS-native keychain lookup?
- Should legacy password auth eventually require an explicit per-run `--legacy-password-ok` confirmation in addition to approval references?
