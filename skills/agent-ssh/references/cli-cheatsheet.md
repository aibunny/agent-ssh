# agent-ssh CLI Cheatsheet

## Default flow

```sh
agent-ssh config validate
agent-ssh hosts list
agent-ssh profiles list --server <alias>
agent-ssh exec --server <alias> --profile <name> [--arg key=value ...] [--approval CAB-1234]
```

## Unrestricted session flow

Use this only when profile execution is not enough, or the user explicitly wants arbitrary commands.

```sh
agent-ssh session open --server <alias> --mode unrestricted --approval CAB-1234
agent-ssh session exec --session <session-id> --cmd "<command>"
agent-ssh session close <session-id>
```

## Policy reminders

- Prefer named profiles over raw commands.
- Approval is required when the server or profile requires it.
- `allow_unrestricted_sessions = true` matters only with `requires_approval = true`.
- `legacy_password` is compatibility-only and should never expose the password to the agent.
- Persistent unrestricted sessions are not supported for `legacy_password`.

## Install reminders

- macOS: Homebrew is the simplest native setup on MacBook.
- Linux: Homebrew on Linux, the installer script, Cargo, or Debian packages.
- Windows: use WSL2 with Ubuntu or Debian for the current release workflow.
