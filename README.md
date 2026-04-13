# agent-ssh

A security-first SSH broker written in Rust. Define named servers and named
command profiles once in a TOML file; then any caller — human or AI agent —
can run those exact commands and **always see the full output**, without ever
touching a raw password or long-lived SSH key. The secure default is
certificate-oriented; a separate legacy compatibility lane can use broker-owned
secret references for older hosts without exposing plaintext to the caller.

```
$ agent-ssh exec --server staging-api --profile logs \
    --arg service=api --arg "since=5 min ago"

--- agent-ssh exec: staging-api  deploy@10.0.1.10:22  profile=logs ---
Apr 12 10:00:01 staging systemd[1]: Started api.service.
Apr 12 10:00:02 staging api[1234]: listening on :8080
--- exit 0 ---
```

## Contents

- [Install](#install)
- [Quick start (5 minutes)](#quick-start)
- [Configuration reference](#configuration-reference)
- [CLI reference](#cli-reference)
- [Legacy password compatibility](#legacy-password-compatibility)
- [Approval-gated servers](#approval-gated-servers)
- [Fail2ban guidance](#fail2ban-guidance)
- [Audit log](#audit-log)
- [Security model](#security-model)

---

## Install

### Homebrew (macOS / Linux)

```sh
brew tap aibunny/agent-ssh https://github.com/aibunny/agent-ssh
brew install agent-ssh
```

### apt (Debian / Ubuntu)

Download the `.deb` for your architecture from the
[Releases](https://github.com/aibunny/agent-ssh/releases/latest) page, then:

```sh
# Intel / AMD
sudo dpkg -i agent-ssh_*_amd64.deb

# ARM (Raspberry Pi, AWS Graviton, Apple Silicon Linux VMs)
sudo dpkg -i agent-ssh_*_arm64.deb
```

### One-line installer (all platforms)

```sh
curl -fsSL https://raw.githubusercontent.com/aibunny/agent-ssh/main/scripts/install.sh | sh
```

### Cargo

```sh
cargo install --git https://github.com/aibunny/agent-ssh agent-ssh-cli
```

### Build from source

```sh
git clone https://github.com/aibunny/agent-ssh
cd agent-ssh
cargo build --release -p agent-ssh-cli
# Binary is at: target/release/agent-ssh
```

---

## Quick start

**Step 1 — create a starter config**

```sh
agent-ssh init
```

This writes a fully commented `agent-ssh.toml` in the current directory.
Open it and fill in the values marked `<CHANGE ME>`.

**Step 2 — validate the config**

```sh
agent-ssh config validate
```

**Step 3 — list your servers**

```sh
agent-ssh hosts list
```

```
staging-api    environment=staging     user=deploy    requires_approval=false
prod-web-1     environment=production  user=deploy    requires_approval=true
```

**Step 4 — run a command and see the output**

```sh
agent-ssh exec --server staging-api --profile disk
```

```
--- agent-ssh exec: staging-api  deploy@10.0.1.10:22  profile=disk ---
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        20G  8.1G   11G  44% /
--- exit 0 ---
```

Every execution is recorded in the audit log (`./data/audit.jsonl` by default).

---

## Configuration reference

The broker looks for a config file in this order:

1. `--config <path>` CLI flag
2. `$AGENT_SSH_CONFIG` environment variable
3. `agent-ssh.toml` in the current directory

Run `agent-ssh init` to generate a starter file with all options annotated.

### Full example

```toml
[broker]
cert_ttl_seconds = 120          # 1–3600 seconds; keep short
audit_log_path   = "./data/audit.jsonl"
default_signer   = "step_ca"   # must match a [signers.*] key

# ── Signers ──────────────────────────────────────────────────────────────────
# A signer issues short-lived SSH certificates (step-ca / Vault / etc.).

[signers.step_ca]
kind        = "step-ca"
ca_url      = "https://ca.internal.example"
provisioner = "agent-ssh"
subject     = "agent-ssh-broker"

# ── Servers ───────────────────────────────────────────────────────────────────
# Each entry is a named SSH target.  Callers use the alias, never the host.

[servers.staging-api]
host             = "10.0.1.10"
port             = 22             # optional; defaults to 22
user             = "deploy"       # 'root' is discouraged and requires root_login_acknowledged = true
environment      = "staging"
allowed_profiles = ["logs", "disk"]
requires_approval = false         # optional; defaults to false
# auth_method    = "certificate"  # optional; "certificate" is the default

[servers.prod-web-1]
host             = "10.0.10.21"
user             = "deploy"
environment      = "production"
allowed_profiles = ["logs"]
requires_approval = true          # every exec needs --approval <ref>

# Optional legacy password compatibility server.
# The password itself must not be stored in TOML or .env.
# Instead, the .env file next to the config may contain:
#   AGENT_SSH_LEGACY_WEB_PASSWORD_REF=os_keychain:agent-ssh:legacy-web
#
# [servers.legacy-web]
# host             = "10.0.20.5"
# user             = "deploy"
# environment      = "migration"
# allowed_profiles = ["logs"]
# auth_method      = "legacy_password"
# password_secret_ref_env_var = "AGENT_SSH_LEGACY_WEB_PASSWORD_REF"
# legacy_password_acknowledged = true
# fail2ban_allowlist_confirmed = true

# ── Profiles ──────────────────────────────────────────────────────────────────
# Templates use {{placeholder}} tokens.  Shell metacharacters are FORBIDDEN.

[profiles.logs]
description = "Tail systemd service logs"
template    = "journalctl -u {{service}} --since {{since}} --no-pager"

[profiles.disk]
description = "Show disk usage"
template    = "df -h"
```

### Validation rules

| Field | Constraint |
|-------|-----------|
| Identifiers (aliases, profile names, signer names) | `[a-z0-9][a-z0-9_-]*`, max 64 chars |
| `host` | No whitespace, max 253 chars |
| `user` | `[A-Za-z_][A-Za-z0-9._-]*`, max 32 chars; `root` is discouraged and requires `root_login_acknowledged = true` |
| `port` | 1–65535, defaults to 22 |
| `cert_ttl_seconds` | 1–3600 |
| `auth_method` | Optional; `certificate` is the secure default, `legacy_password` is compatibility-only |
| `password_secret_ref_env_var` | Required only for `legacy_password`; must name an env var whose value is an opaque reference like `os_keychain:agent-ssh:legacy-web` |
| `template` | Safe literal tokens + `{{placeholder}}` only, max 4096 chars |
| Argument values at runtime | No control characters, max 4096 chars |

### Template grammar

Template tokens must be either:

- A **safe literal** — characters `a-z A-Z 0-9 _ . / : = @ + - , %` only
- A **placeholder** — `{{name}}` where `name` matches `[a-z][a-z0-9_-]*`

Tokens containing `|`, `;`, `>`, `` ` ``, `$`, `&`, or quotes cause the
broker to refuse to load the profile. Placeholder values are always
single-quoted when rendered.

---

## CLI reference

### `agent-ssh init`

Create a starter `agent-ssh.toml` in the current directory.

```sh
agent-ssh init                         # writes agent-ssh.toml
agent-ssh init --output /etc/agent-ssh.toml   # custom path
agent-ssh init --force                 # overwrite existing file
```

---

### `agent-ssh config validate`

Parse and validate the configuration file. Exits 0 if valid.

```sh
agent-ssh config validate --config agent-ssh.toml
```

---

### `agent-ssh hosts list`

List all configured server aliases.

```sh
agent-ssh hosts list
```

```
staging-api    environment=staging     user=deploy    requires_approval=false
prod-web-1     environment=production  user=deploy    requires_approval=true
```

---

### `agent-ssh profiles list`

List the command profiles allowed for a server.

```sh
agent-ssh profiles list --server staging-api
```

```
logs    requires_approval=false    description=Tail systemd service logs
disk    requires_approval=false    description=Show disk usage
```

---

### `agent-ssh run`

**Plan only** — validate the request and show what would be executed, without
making any SSH connection. Useful for scripting and dry-run checks.

```sh
agent-ssh run \
  --server staging-api \
  --profile logs \
  --arg service=api \
  --arg "since=10 min ago"
```

```
server:           staging-api
target:           deploy@10.0.1.10:22
environment:      staging
profile:          logs
auth_method:      certificate
signer:           step_ca
requires_approval:false
approval_provided:false
rendered_command: journalctl -u 'api' --since '10 min ago' --no-pager
execution_mode:   PlanOnly
audit_log:        ./data/audit.jsonl

(Use `agent-ssh exec` to plan and run this command.)
```

---

### `agent-ssh exec`

**Plan and execute** — the primary command for agents and operators. Connects
via SSH, runs the command, and always returns the full captured output.

```sh
agent-ssh exec \
  --server staging-api \
  --profile logs \
  --arg service=api \
  --arg "since=5 min ago"
```

```
--- agent-ssh exec: staging-api  deploy@10.0.1.10:22  profile=logs ---
Apr 12 10:00:01 staging systemd[1]: Started api.service.
Apr 12 10:00:02 staging api[1234]: listening on :8080
--- exit 0 ---
```

The stdout of the remote command is printed to stdout; the header, stderr,
and exit-code lines are printed to stderr. This makes it easy for agents to
capture stdout cleanly while still seeing status information on stderr.

Exit code of `agent-ssh exec` mirrors the remote command's exit code:
- `0` — command succeeded
- non-zero — command failed (the exit code is the remote command's exit code)

#### `--dry-run`

Show the exact SSH invocation without executing it.

```sh
agent-ssh exec --server staging-api --profile disk --dry-run
```

```
dry-run: would execute the following SSH command:

  ssh -o BatchMode=yes -o PreferredAuthentications=publickey -o PubkeyAuthentication=yes -o PasswordAuthentication=no -o KbdInteractiveAuthentication=no -o NumberOfPasswordPrompts=0 -o IdentitiesOnly=yes -o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p 22 deploy@10.0.1.10 df -h

target:  deploy@10.0.1.10:22
command: df -h
```

---

## Legacy password compatibility

`legacy_password` exists only as a migration bridge for servers that cannot yet use the certificate path.

- It is off by default and must be enabled per server.
- It always requires approval at plan/exec time.
- `.env` may hold only opaque secret references, not the password itself.
- Initial reference format is `os_keychain:<service>:<account>`.
- macOS lookups use `security find-generic-password`.
- Linux lookups use `secret-tool lookup`.

Example `.env` file next to `agent-ssh.toml`:

```dotenv
AGENT_SSH_LEGACY_WEB_PASSWORD_REF=os_keychain:agent-ssh:legacy-web
```

The broker uses a one-shot askpass helper, so dry-run output, audit logs, and ssh argv stay free of plaintext password material.

---

## Fail2ban guidance

The secure default uses system OpenSSH in a publickey-only, non-interactive mode:

- `BatchMode=yes`
- `PreferredAuthentications=publickey`
- `PubkeyAuthentication=yes`
- `PasswordAuthentication=no`
- `KbdInteractiveAuthentication=no`
- `NumberOfPasswordPrompts=0`
- `IdentitiesOnly=yes`

This removes password and keyboard-interactive retries, which are common
fail2ban triggers. The broker cannot directly control remote fail2ban policy,
so if your broker egresses from fixed IPs or CIDRs, allowlist those addresses
in fail2ban's `ignoreip` setting on the remote hosts when you need an explicit
no-ban guarantee.

If you enable `legacy_password`, the broker switches to a single askpass-driven
password attempt for that server. That still depends on remote fail2ban policy,
so `fail2ban_allowlist_confirmed = true` is required and operator allowlisting
is still recommended.

---

## Approval-gated servers

Set `requires_approval = true` on a server or profile to require an opaque
approval reference (ticket ID, change record number, etc.) before execution:

```sh
agent-ssh exec \
  --server prod-web-1 \
  --profile logs \
  --arg service=nginx \
  --arg "since=5 min ago" \
  --approval CAB-1234
```

Without `--approval` the broker blocks the request and records a `blocked`
audit event. The approval reference is stored in the audit log but is never
validated cryptographically in this release — that is a planned future
capability.

---

## Audit log

Every broker decision is appended to the JSONL file at `broker.audit_log_path`.
Each line is a complete JSON record:

```json
{
  "event_id": "a3b5…",
  "occurred_at": "2026-04-12T10:00:01Z",
  "actor": "cli",
  "action": "run_execute",
  "outcome": "executed",
  "message": "command completed with exit code 0",
  "server_alias": "staging-api",
  "environment": "staging",
  "profile": "logs",
  "args": { "service": "api", "since": "5 min ago" },
  "rendered_command": "journalctl -u 'api' --since '5 min ago' --no-pager",
  "requires_approval": false,
  "approval_reference": null,
  "signer": "step_ca",
  "transport": "system_ssh",
  "auth_method_kind": "certificate",
  "exit_code": 0
}
```

Possible `action` values: `config_validate`, `hosts_list`, `profiles_list`,
`run_plan`, `run_execute`.

Possible `outcome` values: `succeeded`, `blocked`, `invalid`, `planned`,
`executed`, `failed`.

Note: `run_execute` always follows a `run_plan` event for the same request —
you get two audit events per `exec` invocation.

---

## Security model

| Property | How it's enforced |
|----------|-------------------|
| Discouraged root login | `user = "root"` requires `root_login_acknowledged = true` so the exception stays explicit and reviewable |
| No free-form shell | Only profiles from `allowed_profiles` can run; no arbitrary commands |
| No shell injection | Template literals are whitelisted; placeholder values are single-quoted |
| No null/control chars | Argument values containing control characters are rejected |
| No overlong inputs | Identifiers ≤64, hosts ≤253, usernames ≤32, templates/values ≤4096 chars |
| No raw password exposure | Plaintext passwords are rejected in TOML and `.env`; legacy password mode accepts only opaque secret references |
| Secure default transport | Certificate mode disables password and keyboard-interactive SSH auth |
| Exact alias matching | Partial or fuzzy server names are rejected; `staging` ≠ `staging-api` |
| Allowlisted profiles | Each server declares exactly which profiles it may run |
| Approval gating | `requires_approval = true` blocks execution without `--approval`, and `legacy_password` is always approval-gated |
| Audit trail | Every allowed and blocked decision is written to JSONL before returning |
| Short-lived certs | Certificate TTL is bounded to 1–3600 seconds |
