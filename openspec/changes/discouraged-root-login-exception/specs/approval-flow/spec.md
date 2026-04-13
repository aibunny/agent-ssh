## ADDED Requirements

### Requirement: Root-login exception does not silently change approval behavior
The system SHALL keep existing approval semantics unchanged when a root-login exception is acknowledged.

#### Scenario: Root login on an otherwise unprotected server
- **GIVEN** a server entry configured with `user = "root"`
- **AND** `root_login_acknowledged = true`
- **AND** neither the server nor the selected profile otherwise requires approval
- **WHEN** the broker plans the run
- **THEN** the existing approval rules apply unchanged
