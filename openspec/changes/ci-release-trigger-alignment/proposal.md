## Why

The repository currently has only a release workflow, and it triggers only on pushed tags. That means normal pushes to `main` and pull requests do not run any GitHub Actions checks. The current tag trigger is also written like a regex even though GitHub Actions uses glob patterns, so tags like `v0.1.0` may not match at all.

The user also wants the release/version surface centered on `0.1.0` as the initial project version.

## What Changes

- Add a CI workflow that runs on pushes to `main` and on pull requests.
- Correct the release workflow tag trigger so `v0.1.0`-style tags actually fire.
- Add a repository validation script that checks all active OpenSpec changes instead of only older hard-coded ones.
- Keep the canonical repo version at `0.1.0` and use `v0.1.0` as the documented first release-tag example.

## Impact

- Regular development pushes get automated validation.
- Release automation becomes predictable for semver-style tags.
- CI stays aligned with the repo’s growing OpenSpec history.
