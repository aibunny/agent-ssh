# audit-logging Specification

## Purpose

Define the security audit trail for broker policy decisions, rendered commands, and execution outcomes.

## Requirements

### Requirement: Append-only audit records
The system SHALL write structured audit records for broker actions.

#### Scenario: Run request processed
- **GIVEN** a caller submits a run request
- **WHEN** the broker processes the request
- **THEN** the broker appends an audit record
- **AND** the record includes the action type, target alias, and policy outcome

### Requirement: Rendered command auditability
The system SHALL record the rendered command and request metadata used for policy evaluation.

#### Scenario: Command rendering succeeds
- **GIVEN** a profile command is rendered successfully
- **WHEN** the broker prepares the request result
- **THEN** the audit record includes the rendered command, profile name, and arguments
- **AND** the record can be correlated to the request result
