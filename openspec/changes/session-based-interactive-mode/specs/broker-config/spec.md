## MODIFIED Requirements

### Requirement: TOML broker configuration
The system SHALL load broker configuration from a TOML document that defines broker settings, named servers, named command profiles, and signer references.

#### Scenario: Server explicitly opts into unrestricted sessions
- **GIVEN** a server entry sets `allow_unrestricted_sessions = true`
- **WHEN** the broker loads the configuration
- **THEN** the parsed server configuration preserves that unrestricted-session opt-in
- **AND** other server policy fields continue to load normally

### Requirement: Security-oriented validation
The system SHALL reject configuration that violates secure broker invariants.

#### Scenario: Unrestricted-session flag omitted
- **GIVEN** a server entry omits `allow_unrestricted_sessions`
- **WHEN** the broker validates the configuration
- **THEN** validation succeeds
- **AND** unrestricted sessions remain disabled for that server by default
