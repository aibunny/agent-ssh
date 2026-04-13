# signer-abstraction Specification

## Purpose

Define how the broker requests short-lived SSH authentication material without exposing reusable credentials to agents.

## Requirements

### Requirement: Pluggable signer interface
The system SHALL use a signer abstraction so certificate issuance can be provided by different backends.

#### Scenario: Named signer selected
- **GIVEN** the broker is configured with a default signer
- **WHEN** a run request is prepared
- **THEN** the broker resolves the signer by name
- **AND** execution planning uses the signer abstraction instead of provider-specific logic

### Requirement: Short-lived authentication material
The system SHALL use short-lived SSH authentication material for broker-mediated sessions.

#### Scenario: Session request requires credentials
- **GIVEN** the broker is preparing a remote session
- **WHEN** the signer returns authentication material
- **THEN** the material has an explicit expiration
- **AND** the agent does not receive a reusable long-lived private key
