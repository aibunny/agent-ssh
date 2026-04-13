## ADDED Requirements

### Requirement: Foundation audit record schema
The broker foundation SHALL append one JSON object per audit event to the configured audit log path.

#### Scenario: Protected request is blocked
- **GIVEN** a protected run request is missing approval
- **WHEN** the broker handles the request
- **THEN** an audit record is appended
- **AND** the record includes the blocked outcome and reason

### Requirement: Audit records include rendered command context
The broker foundation SHALL include rendered command context when command rendering succeeds.

#### Scenario: Run request passes policy checks
- **GIVEN** a request with a valid alias, allowed profile, and valid arguments
- **WHEN** the broker prepares the request result
- **THEN** the audit record includes the rendered command, profile name, server alias, and argument map
- **AND** the audit record identifies whether execution was only planned or fully attempted
