## MODIFIED Requirements

### Requirement: Security-oriented validation
The system SHALL reject configuration that violates secure broker invariants, while allowing explicitly acknowledged discouraged exceptions.

#### Scenario: Root login without acknowledgement
- **GIVEN** a server entry configured with `user = "root"`
- **AND** `root_login_acknowledged` is absent or false
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports that root login is discouraged and must be explicitly acknowledged

#### Scenario: Root login with acknowledgement
- **GIVEN** a server entry configured with `user = "root"`
- **AND** `root_login_acknowledged = true`
- **WHEN** the broker validates the configuration
- **THEN** validation succeeds
- **AND** the configuration remains visibly marked as a discouraged exception
