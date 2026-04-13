## ADDED Requirements

### Requirement: CI runs on primary development events
The repository SHALL run automated validation on pushes to `main` and on pull requests.

#### Scenario: Push to main
- **GIVEN** a commit is pushed to `main`
- **WHEN** GitHub Actions evaluates the repository workflows
- **THEN** a CI workflow runs repository validation

#### Scenario: Pull request
- **GIVEN** a pull request targets the repository
- **WHEN** GitHub Actions evaluates the repository workflows
- **THEN** a CI workflow runs repository validation

### Requirement: Release workflow responds to semver tags
The repository SHALL trigger release automation on semver-style tags such as `v0.1.0`.

#### Scenario: Initial release tag
- **GIVEN** a tag named `v0.1.0`
- **WHEN** it is pushed to the repository
- **THEN** the release workflow starts

### Requirement: Repository validation covers active OpenSpec changes
The repository SHALL validate all active OpenSpec changes during its scripted verification flow.

#### Scenario: New active change exists
- **GIVEN** an active change folder exists under `openspec/changes/`
- **WHEN** repository validation runs
- **THEN** that change is included in OpenSpec validation
- **AND** its task journal coverage is checked when tasks are completed
