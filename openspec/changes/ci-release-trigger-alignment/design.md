## Context

The repo’s current automation only defines [release.yml](/Users/aibunny/agent-ssh/.github/workflows/release.yml), and that workflow listens only for tag pushes. There is no CI workflow for `main` or pull requests, so the user sees “nothing ran” after pushing to `main`. The release tag filter is also written like a regex (`v[0-9]+.[0-9]+.[0-9]+`) even though GitHub Actions `tags` filters use glob matching.

The repo already uses `0.1.0` as the workspace and formula version, so the automation/docs should align around `v0.1.0` as the first release-tag example.

This design is grounded in GitHub’s official workflow syntax:

- `on.push.branches` and `on.pull_request` are the right triggers for CI.
- `on.push.tags` uses glob patterns, not regex.

## Goals / Non-Goals

**Goals:**

- Run CI on pushes to `main`.
- Run CI on pull requests.
- Trigger releases on semver-style tags such as `v0.1.0`.
- Validate all current OpenSpec changes in CI instead of a stale hard-coded subset.
- Keep `0.1.0` as the canonical initial version across release-facing examples.

**Non-Goals:**

- Changing the actual package version away from `0.1.0`.
- Building a complex multi-stage release promotion system.
- Adding every possible GitHub Actions optimization in this change.

## Decisions

### 1. Split CI and release responsibilities

The repo will have:

- `ci.yml` for `main` pushes, pull requests, and manual runs
- `release.yml` for semver tag pushes only

This avoids overloading release automation as general CI.

### 2. Use a GitHub-compatible semver tag glob

The release workflow will use:

- `v*.*.*`

This matches tags such as `v0.1.0`, `v1.2.3`, and similar semver-style tags.

### 3. Validate all active OpenSpec changes dynamically

Instead of hard-coding a partial list of change IDs, the repo will add a validation script that iterates active change directories, runs `openspec validate`, and checks the task journal for each change with `tasks.md`.

This keeps CI aligned with the actual repo state as new changes are added.

### 4. Keep `0.1.0` as the initial release baseline

The repo already declares version `0.1.0` in Cargo metadata and the Homebrew formula. The automation/docs in this change will use `v0.1.0` as the example first release tag so the user experience matches the checked-in version.

## Risks / Trade-offs

- [CI may take slightly longer because it validates all active changes] → The repo is still small, and correctness is worth the extra time.
- [A broad `v*.*.*` glob can match non-semver edge cases] → It is still much closer to the intended release behavior than the current regex-like string, and it matches the documented `v0.1.0` flow.

## Migration Plan

1. Author proposal, design, tasks, and spec delta.
2. Add dynamic OpenSpec validation scripting and update bootstrap-dev.
3. Add `ci.yml` for `main`/PR validation.
4. Correct the release tag trigger and align docs/comments around `v0.1.0`.
