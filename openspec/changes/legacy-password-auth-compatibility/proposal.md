## Why

`agent-ssh` now enforces a strong certificate-first baseline, which is the right default for the secure release. Some real deployments still need a temporary bridge for legacy hosts that cannot yet consume short-lived SSH certificates. The user is willing to support that bridge only if the agent never receives the password in raw format.

That means we cannot simply restore `sshpass`, inline passwords, or plaintext `.env` values. If password compatibility returns at all, it needs to be an explicit legacy mode with a separate OpenSpec change, an auditable opt-in, and a secret model that keeps plaintext outside the agent-facing API surface.

## What Changes

- Add a new `legacy_password` auth mode that is disabled by default and must be explicitly enabled per server.
- Allow only opaque secret references, supplied through a configured environment variable name, for legacy password auth.
- Auto-load a sibling `.env` file only to resolve secret reference variables, not raw passwords.
- Require legacy password servers to acknowledge fail2ban allowlisting risk and to pass approval gates before execution.
- Execute legacy password sessions through system OpenSSH with a broker-managed `SSH_ASKPASS` helper so plaintext is not stored in TOML, CLI args, or audit logs.
- Add tests and docs that prove the compatibility path is non-default, redacted, and fail-closed.

## Impact

- Preserves the secure certificate path as the default and preferred mode.
- Creates a bounded migration lane for legacy hosts on macOS and Linux.
- Adds a small amount of runtime complexity in exchange for keeping password material out of the agent-visible surface.
