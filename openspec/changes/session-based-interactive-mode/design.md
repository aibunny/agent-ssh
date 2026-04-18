# Design: Broker-Controlled Interactive SSH Sessions

## Transport Model

Sessions use SSH ControlMaster:
- **Open**: `ssh -f -N -M -o ControlPath=<socket> -o ControlPersist=yes <target>` — forks a master connection to background.
- **Exec**: `ssh -o ControlMaster=no -o ControlPath=<socket> <target> <command>` — multiplexes over the open master.
- **Close**: `ssh -O exit -o ControlPath=<socket> <target>` — terminates the master.
- **Check**: `ssh -O check -o ControlPath=<socket> <target>` — verifies master is alive.

Socket path: `/tmp/agent-ssh-<first-8-hex-chars-of-UUID>.sock` (≤ 35 chars, well under 104-char Unix socket limit).

## Session Lifecycle

```
open → [exec* | check] → close
         ↓ TTL/idle expired → auto-cleanup on next access
```

Default TTL: 300 s (5 min). Maximum TTL: 3600 s. Default idle timeout: 60 s.

## Execution Modes

| Mode | Config requirement | Approval required | Command source |
|---|---|---|---|
| Restricted | (default) | per-server policy | profile + args (same as exec) |
| Unrestricted | `allow_unrestricted_sessions = true` + `requires_approval = true` | always | raw `--cmd` string |

## Security Properties

- No agent forwarding (`ForwardAgent=no`).
- No X11 forwarding (`ForwardX11=no`).
- No local command execution (`PermitLocalCommand=no`).
- Max command length in unrestricted mode: 4096 chars.
- Control characters rejected in unrestricted command strings.
- Legacy password auth not supported for persistent sessions (use `exec` instead).
- TTL and idle timeout enforced on every exec and close operation.
- All session events written to the audit log before any SSH action.

## Session Persistence

Session records: `<audit_log_parent>/sessions/<id>.json` (JSON, includes all metadata except credentials).

## Audit Events

| Action | Outcome | When |
|---|---|---|
| `session_open` | `succeeded` / `denied` | On open attempt |
| `session_close` | `succeeded` | On close |
| `session_expire` | `expired` | When TTL/idle detected |
| `session_command` | `executed` / `denied` / `failed` | Per command |
