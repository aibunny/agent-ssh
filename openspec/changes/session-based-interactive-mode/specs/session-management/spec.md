## ADDED Requirements

### Requirement: Broker-held persistent SSH sessions
The system SHALL support broker-controlled SSH sessions that keep a remote connection alive across multiple commands.

#### Scenario: Open session establishes a broker-held connection
- **GIVEN** a configured certificate-authenticated server alias
- **WHEN** the broker opens a session for that server
- **THEN** the broker establishes a persistent SSH connection under its control
- **AND** it returns a durable session record with a unique session ID

#### Scenario: Session command reuses the existing connection
- **GIVEN** an open broker-controlled session
- **WHEN** the broker executes another command within that session
- **THEN** the command runs over the existing session transport
- **AND** the broker returns captured stdout, stderr, and exit code

### Requirement: Session lifetime enforcement
The system SHALL enforce bounded session lifetime for broker-controlled sessions.

#### Scenario: Session TTL or idle timeout elapses
- **GIVEN** a broker-controlled session has exceeded its TTL or idle timeout
- **WHEN** the caller attempts to use that session again
- **THEN** the broker denies the request as expired
- **AND** the broker cleans up the session state
