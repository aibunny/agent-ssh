## MODIFIED Requirements

### Requirement: Security-oriented validation
The system SHALL reject configuration that violates secure broker invariants.

#### Scenario: Insecure server user
- **GIVEN** a server entry configured with `user = "root"`
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports that root login is not allowed

#### Scenario: Undefined profile reference
- **GIVEN** a server entry that references a profile name not defined under `[profiles]`
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports the missing profile by name

#### Scenario: Password authentication is configured
- **GIVEN** a server entry sets `auth_method = "password"` or `password_env_var`
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports that password authentication is not supported in the secure release
