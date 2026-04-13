## ADDED Requirements

### Requirement: Legacy password transport uses broker-managed askpass
The system SHALL execute legacy password SSH sessions without placing plaintext password material into argv, config files, or audit logs.

#### Scenario: Legacy password execution
- **GIVEN** a server uses `auth_method = "legacy_password"`
- **WHEN** the broker executes the SSH session
- **THEN** it uses system OpenSSH with a broker-managed `SSH_ASKPASS` helper
- **AND** the ssh argv does not contain the password
- **AND** the broker fails closed if the secret reference cannot be resolved
