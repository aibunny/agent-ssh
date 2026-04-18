## ADDED Requirements

### Requirement: Session management commands
The system SHALL provide CLI commands to open, use, inspect, and close broker-controlled SSH sessions.

#### Scenario: Open a restricted session
- **GIVEN** a configured server alias
- **WHEN** the operator runs `agent-ssh session open --server <alias>`
- **THEN** the CLI asks the broker to open a restricted session
- **AND** the CLI prints the resulting session ID

#### Scenario: Execute a command in a session
- **GIVEN** an open session ID
- **WHEN** the operator runs `agent-ssh session exec --session <id> ...`
- **THEN** the CLI routes the request through the broker session manager
- **AND** the CLI prints the captured command output

#### Scenario: List open sessions
- **GIVEN** the broker has open sessions
- **WHEN** the operator runs `agent-ssh session list`
- **THEN** the CLI lists the currently open session IDs and their server metadata

#### Scenario: Close a session
- **GIVEN** an open session ID
- **WHEN** the operator runs `agent-ssh session close <id>`
- **THEN** the CLI asks the broker to close the session
- **AND** the CLI reports that the session was closed
