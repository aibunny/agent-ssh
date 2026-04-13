## Why

The current secure baseline rejects `user = "root"` outright. Some environments still need temporary direct-root access for tightly controlled legacy systems. The user wants that path to be possible, but clearly discouraged.

We should not silently relax the baseline. If root login is allowed at all, it should be:

- explicit in config
- clearly documented as discouraged
- reviewable in OpenSpec and git history

## What Changes

- Add an explicit acknowledgment flag that is required whenever `user = "root"`.
- Preserve the current default behavior for all non-root server entries.
- Update validation, docs, and examples so root login is possible only through an opt-in exception path.
- Add tests for accepted and rejected root-login configurations.

## Impact

- Makes room for legacy environments that still need root login.
- Keeps root usage visible and intentional instead of normalizing it as the default.
