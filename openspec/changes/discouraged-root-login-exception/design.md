## Context

`agent-ssh` began with a strict “no root login” rule, which is still the safest default. The user now wants root to be possible, but only if the product says plainly that it is discouraged.

That means the implementation should:

- keep root rejected by default
- allow root only through an explicit opt-in acknowledgment
- document root as a discouraged compatibility exception

## Goals / Non-Goals

**Goals:**

- Permit `user = "root"` only when the operator explicitly acknowledges it.
- Preserve the current non-root defaults.
- Keep the warning language visible in docs and specs.

**Non-Goals:**

- Making root login the default or recommended path.
- Adding a broad new policy framework for Unix account classes.

## Decisions

### 1. Root login requires `root_login_acknowledged = true`

If a server entry sets `user = "root"`, it must also set:

- `root_login_acknowledged = true`

Without that flag, validation fails. For non-root users, the flag is not allowed.

### 2. Documentation says “discouraged,” not “recommended”

The wording across config docs, README, and examples will describe root login as:

- discouraged
- compatibility-oriented
- something operators should avoid unless required by their environment

### 3. Approval behavior stays unchanged in this change

This change is intentionally narrow. It modifies config validation and documentation only; it does not add a new automatic approval rule tied to root users.

## Risks / Trade-offs

- [Allowing root weakens the security posture] → Keep it opt-in and prominently discouraged.
- [Operators may cargo-cult the example] → Avoid making root the main example path.

## Migration Plan

1. Author proposal, design, tasks, and spec deltas.
2. Extend config validation for `root_login_acknowledged`.
3. Update docs/tests/examples to reflect the discouraged exception path.
