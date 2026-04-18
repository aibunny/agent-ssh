## ADDED Requirements

### Requirement: Unrestricted sessions are explicit and approval-gated
The system SHALL allow arbitrary command execution only inside explicit unrestricted sessions that are both server-opted-in and approval-gated.

#### Scenario: Server is not opted into unrestricted sessions
- **GIVEN** a server alias does not set `allow_unrestricted_sessions = true`
- **WHEN** a caller requests an unrestricted session on that server
- **THEN** the broker denies the request

#### Scenario: Approval is missing for unrestricted mode
- **GIVEN** a server alias sets `allow_unrestricted_sessions = true`
- **AND** the caller does not provide an approval reference
- **WHEN** the caller requests an unrestricted session
- **THEN** the broker denies the request

#### Scenario: Raw command executes in unrestricted mode
- **GIVEN** a server alias sets `allow_unrestricted_sessions = true`
- **AND** that server requires approval
- **AND** the caller provides an approval reference
- **WHEN** the caller opens an unrestricted session and executes `--cmd`
- **THEN** the broker accepts the raw command string for that session
- **AND** the broker executes it over the broker-held SSH session
