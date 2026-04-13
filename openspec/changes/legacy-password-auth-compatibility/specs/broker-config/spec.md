## ADDED Requirements

### Requirement: Legacy password auth is explicit and reference-based
The system SHALL keep password compatibility disabled by default and SHALL accept it only through an explicit legacy auth mode that references broker-owned secret material indirectly.

#### Scenario: Valid legacy password server
- **GIVEN** a server entry sets `auth_method = "legacy_password"`
- **AND** it sets `password_secret_ref_env_var`
- **AND** it sets `legacy_password_acknowledged = true`
- **AND** it sets `fail2ban_allowlist_confirmed = true`
- **WHEN** the broker validates the configuration
- **THEN** the server is accepted as an explicit legacy compatibility target
- **AND** the password itself is not present in the parsed config

#### Scenario: Legacy password mode missing explicit acknowledgement
- **GIVEN** a server entry sets `auth_method = "legacy_password"`
- **AND** `legacy_password_acknowledged` is absent or false
- **WHEN** the broker validates the configuration
- **THEN** validation fails

#### Scenario: Raw password fields are rejected
- **GIVEN** a server entry attempts to provide plaintext password material in TOML or through deprecated raw password fields
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports that only opaque secret references are supported
