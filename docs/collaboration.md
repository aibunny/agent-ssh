# Collaboration

## Roles

- Codex is the managing agent and execution lead.
- Claude Code CLI is a scoped specialist for architecture review, threat review, design validation, verification, and tightly bounded implementation tasks.

## Source Of Truth

OpenSpec artifacts in this repository are the canonical collaboration layer:

- Long-lived truth: `openspec/specs/`
- Active changes: `openspec/changes/<change-id>/`
- Project conventions: `openspec/project.md`

## Required Workflow

1. Create or update an OpenSpec change before meaningful implementation.
2. Review `proposal.md`, `design.md`, `tasks.md`, and spec deltas.
3. Record completed work in `openspec/changes/<change-id>/records/task-journal.md` before marking a task done.
4. Implement code against the approved change.
5. Validate code, specs, and task-journal coverage together.
6. Archive the change when complete.

## Task Recording Rule

- Every checked task in `openspec/changes/<change-id>/tasks.md` must have at least one matching entry in `openspec/changes/<change-id>/records/task-journal.md`.
- Journal entries must record the task ID, UTC timestamp, agent identity, summary, files or modules touched, verification context, and any linked review artifact.
- This rule applies to Codex, Codex subagents, and Claude Code CLI work.
- Use `scripts/record-task.sh <change-id> <task-id> <agent> <summary> [files] [verification] [artifacts]` to append entries consistently.
- Use `scripts/check-task-journal.sh <change-id>` during verification; Codex should not mark a task complete until this journal coverage exists.

## Delegating To Claude Code CLI

Each delegated task must include:

- the OpenSpec change ID
- the exact OpenSpec files to read first
- a tightly bounded scope
- expected files to touch
- acceptance criteria
- a verification checklist

Claude output should be written back into the repository as an OpenSpec-linked review note or other clearly referenced artifact under the active change.

## Review Rule

Codex verifies Claude output before accepting it, updates OpenSpec task state, and rejects any change that weakens security or diverges from the approved design.
