## ADDED Requirements

### Requirement: Completed change tasks are durably recorded
The collaboration workflow SHALL keep an append-only task journal at `openspec/changes/<change-id>/records/task-journal.md` for every active change with completed tasks.

#### Scenario: Checked task is covered by the journal
- **GIVEN** a task is marked complete in `openspec/changes/<change-id>/tasks.md`
- **WHEN** the change artifacts are reviewed
- **THEN** `openspec/changes/<change-id>/records/task-journal.md` exists
- **AND** the journal contains at least one entry for that completed task ID

### Requirement: Journal entries identify agent work and verification context
The task journal SHALL record who performed the work, what changed, and how it was verified for Codex, Codex subagents, and Claude Code CLI contributions.

#### Scenario: Claude specialist task is recorded
- **GIVEN** Claude Code CLI completes a bounded review or implementation task for an active change
- **WHEN** the task is recorded in the journal
- **THEN** the entry includes the task ID, UTC timestamp, agent identity, files or modules touched, and verification summary
- **AND** the entry references any review note or change artifact produced by that task

### Requirement: Journal coverage is machine-checkable
The repository SHALL provide a validation step that fails when completed tasks are missing journal coverage.

#### Scenario: Completed task has no journal entry
- **GIVEN** a task checkbox is marked complete in `tasks.md`
- **WHEN** task-journal validation runs for that change
- **THEN** validation fails if the task journal lacks a corresponding entry for the completed task ID
- **AND** the validation output identifies the missing task ID
