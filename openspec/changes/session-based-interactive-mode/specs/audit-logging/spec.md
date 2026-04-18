## MODIFIED Requirements

### Requirement: Append-only audit records
The system SHALL write structured audit records for broker actions.

#### Scenario: Session lifecycle event processed
- **GIVEN** a caller opens, closes, or loses access to a broker-controlled session
- **WHEN** the broker processes that lifecycle event
- **THEN** the broker appends an audit record
- **AND** the record includes the session action type, target alias, and outcome
- **AND** the record can be correlated by `session_id`

### Requirement: Rendered command auditability
The system SHALL record the rendered command and request metadata used for policy evaluation.

#### Scenario: Command executes inside a broker-held session
- **GIVEN** a command is executed within an open broker-controlled session
- **WHEN** the broker emits the session command audit event
- **THEN** the event includes the `session_id`
- **AND** it records the command outcome
- **AND** it includes the remote exit code when the command reached the remote host
