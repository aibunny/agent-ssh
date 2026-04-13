## ADDED Requirements

### Requirement: Legacy password auth always requires approval
The system SHALL require approval for legacy password auth even when the selected server and profile would otherwise be unapproved.

#### Scenario: Legacy password request without approval
- **GIVEN** a server uses `auth_method = "legacy_password"`
- **AND** the caller does not provide an approval reference
- **WHEN** the broker plans the run
- **THEN** the request is blocked

#### Scenario: Legacy password request with approval
- **GIVEN** a server uses `auth_method = "legacy_password"`
- **AND** the caller provides a non-empty approval reference
- **WHEN** the broker plans the run
- **THEN** the broker may continue policy evaluation
