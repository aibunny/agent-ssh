# Configuration

## Format

`agent-ssh` uses TOML only. A configuration document contains broker-wide settings plus named signers, named servers, and named profiles.

## Top-Level Sections

- `[broker]`: broker-wide settings such as certificate TTL, audit log path, and default signer
- `[signers.<name>]`: signer definitions referenced by the broker or by individual servers
- `[servers.<alias>]`: remote targets keyed by exact alias
- `[profiles.<name>]`: named command profiles keyed by exact profile name

## Server Rules

- Server aliases are exact identifiers, not fuzzy labels.
- `user = "root"` is allowed only as an explicit break-glass exception and is strongly discouraged.
- This exception changes only the SSH username; it does not permit password fallback, arbitrary shell, or any silent relaxation of other policy checks.
- `allowed_profiles` must reference defined profiles.
- `auth_method` may be omitted or set to `"certificate"` explicitly.
- `auth_method = "legacy_password"` is compatibility-only and requires `password_secret_ref_env_var`, `legacy_password_acknowledged = true`, and `fail2ban_allowlist_confirmed = true`.
- Plaintext password settings are rejected.
- `requires_approval = true` blocks protected runs unless an approval reference is provided.

## Profile Rules

- Profiles are globally defined and referenced by name from server allowlists.
- `requires_approval = true` may also be set on a profile to protect that action across every server that allows it.
- Approval is required when either the server or the selected profile marks the request as protected.
- Legacy password servers always require approval, even if `requires_approval` is not set.

## Profile Template Rules

Profile templates intentionally use a narrow grammar in the foundation milestone:

- Tokens are whitespace-separated.
- Literal tokens must stay within a conservative shell-safe character set.
- Placeholders must be standalone tokens like `{{service}}`.
- Placeholders may only appear in fixed option-value positions, immediately after a literal token that starts with `-`.
- Shell operators such as `|`, `;`, `&&`, `||`, redirections, command substitution, backticks, and inline quoting are rejected.

This keeps the user-facing TOML shape familiar while preventing free-form shell composition.

## Example

See [`configs/agent-ssh.example.toml`](/Users/aibunny/agent-ssh/configs/agent-ssh.example.toml) for a complete example.

## Legacy Password Compatibility

Legacy password auth exists only as a migration bridge for hosts that cannot yet consume certificate-based access. It is not the secure default.

- The password itself must not appear in TOML, `.env`, CLI args, or audit output.
- The configured `password_secret_ref_env_var` must resolve to an opaque reference such as `os_keychain:agent-ssh:legacy-web`.
- `agent-ssh` loads that env var from the current process env or from a sibling `.env` file next to the resolved config path.

## Operational Note

By default, `agent-ssh` executes system OpenSSH in a publickey-only, non-interactive mode. Legacy password compatibility instead uses a broker-managed askpass helper with a single password attempt. In both modes, remote fail2ban policy still belongs to the server operator. If your broker uses fixed egress IPs or NAT ranges, allowlist those IPs/CIDRs in fail2ban's `ignoreip` setting on the remote hosts when you need an explicit no-ban guarantee.
