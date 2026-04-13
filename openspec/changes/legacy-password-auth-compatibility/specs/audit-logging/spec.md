## ADDED Requirements

### Requirement: Legacy password audit records are redacted
The audit system SHALL record that legacy password auth was used without recording password material or secret references.

#### Scenario: Legacy password run is planned or executed
- **GIVEN** a run targets a server with `auth_method = "legacy_password"`
- **WHEN** the broker emits planning or execution audit events
- **THEN** the events may include `auth_method_kind = "legacy_password"`
- **AND** they do not include plaintext password material
- **AND** they do not include secret-reference env var names or values
