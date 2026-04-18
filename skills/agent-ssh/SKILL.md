---
name: agent-ssh
description: Use agent-ssh to inspect or operate remote servers through the agent-ssh broker without exposing SSH hosts, certificate material, private keys, or password secrets to the agent. Trigger when a user wants a configured server accessed through agent-ssh, asks to validate agent-ssh config, list available hosts or profiles, run approved remote actions, inspect logs or system state on a brokered server, or open and reuse an approval-gated unrestricted session for multi-step remote work.
---

# Agent SSH

## Overview

Use the brokered `agent-ssh` CLI instead of raw `ssh` whenever the user or repo indicates that remote access is managed through this tool. Treat the broker as the trust boundary: the agent should receive actions and command results, not credentials or raw host secrets.

See [references/cli-cheatsheet.md](references/cli-cheatsheet.md) for the short command reference.

## Preferred workflow

1. Validate the config before assuming any alias or profile exists.
2. Discover available hosts and server-approved profiles.
3. Prefer profile-based execution.
4. Use unrestricted sessions only when the task needs arbitrary commands or multi-step iteration.
5. Close unrestricted sessions when the work is complete.

## Discovery

Run these commands first when you need context:

```sh
agent-ssh config validate
agent-ssh hosts list
agent-ssh profiles list --server <alias>
```

Use the alias the broker exposes. Do not switch to raw hostnames, ports, or usernames unless the user specifically asks for lower-level debugging.

## Execution rules

- Prefer `agent-ssh exec --server <alias> --profile <name>` for normal operations.
- Pass `--arg key=value` only for placeholders the selected profile expects.
- Include `--approval <ref>` whenever the server or profile requires approval.
- If you do not have a required approval reference, stop and ask for it instead of guessing.
- Surface `stdout`, `stderr`, and exit code clearly in your response.

## Unrestricted sessions

Use unrestricted mode only when profile execution is insufficient or when the user explicitly wants arbitrary commands.

Before opening one:

- Confirm the task genuinely needs raw commands.
- Expect server policy to require `requires_approval = true`.
- Expect unrestricted mode to require `allow_unrestricted_sessions = true`.

Use this pattern:

```sh
agent-ssh session open --server <alias> --mode unrestricted --approval CAB-1234
agent-ssh session exec --session <session-id> --cmd "<command>"
agent-ssh session close <session-id>
```

Reuse the same session for related commands. Close it when finished.

## Secret handling

- Never ask for or reveal raw SSH passwords, private keys, certificate blobs, or exact host details if an alias is enough.
- Treat `legacy_password` as a compatibility lane, not the default path.
- Never print or request the resolved password secret itself.
- If the broker is configured for legacy password mode, let the broker handle the opaque secret reference.

## User-facing communication

- Keep the server alias visible in your response so the user knows where the action ran.
- Call out whether you used a profile or an unrestricted session.
- Mention approval requirements when they block progress.
- If you leave a session open intentionally for follow-up work, say so explicitly.
