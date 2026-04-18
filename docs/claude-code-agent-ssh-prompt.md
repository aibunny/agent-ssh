# Claude Code Prompt for `agent-ssh`

Paste this into `CLAUDE.md` or your Claude Code project instructions when you want Claude to use `agent-ssh` as the remote access path.

```md
This project uses `agent-ssh` for remote server access.

When a user asks to inspect or operate a remote server that is managed through this repo, use `agent-ssh` instead of raw `ssh`.

Follow these rules:

1. Treat the broker as the trust boundary.
   The agent should receive actions and command results, not raw SSH secrets, private keys, certificate blobs, passwords, or exact host details when a server alias is enough.

2. Start with discovery.
   Before assuming a host or profile exists, run:
   - `agent-ssh config validate`
   - `agent-ssh hosts list`
   - `agent-ssh profiles list --server <alias>`

3. Prefer approved profiles over arbitrary commands.
   Default execution should use:
   - `agent-ssh exec --server <alias> --profile <name>`
   - Add `--arg key=value` only for placeholders the selected profile expects.
   - Add `--approval <ref>` whenever the server or profile requires approval.

4. Use unrestricted sessions only when necessary.
   Open an unrestricted session only if:
   - the task genuinely requires arbitrary commands or multi-step remote iteration, or
   - the user explicitly asks for raw commands.

   When using unrestricted mode:
   - expect `requires_approval = true`
   - expect `allow_unrestricted_sessions = true`
   - reuse the same session for related commands
   - close the session when finished

   Pattern:
   - `agent-ssh session open --server <alias> --mode unrestricted --approval CAB-1234`
   - `agent-ssh session exec --session <session-id> --cmd "<command>"`
   - `agent-ssh session close <session-id>`

5. Handle legacy password mode safely.
   If the server uses `legacy_password`, treat it as a broker-managed compatibility lane. Never ask for, print, or expose the resolved password secret. Let the broker handle opaque secret references.

6. Communicate clearly to the user.
   In responses:
   - mention the server alias you used
   - say whether you used a profile or an unrestricted session
   - summarize stdout, stderr, and exit code clearly
   - mention approval blockers if present
   - say if you intentionally left a session open for follow-up work

7. Do not silently fall back to raw SSH.
   If `agent-ssh` is expected but unavailable, explain the blocker instead of switching to direct `ssh`.
```
