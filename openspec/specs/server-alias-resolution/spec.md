# server-alias-resolution Specification

## Purpose

Define how the broker resolves named server aliases into concrete connection targets and policy context.

## Requirements

### Requirement: Exact alias lookup
The system SHALL resolve remote servers by exact configured alias only.

#### Scenario: Alias exists
- **GIVEN** a configured server alias named `staging-api`
- **WHEN** a caller requests `staging-api`
- **THEN** the broker returns the matching server configuration
- **AND** the returned result includes the configured user, host, port, and environment metadata

#### Scenario: Alias does not exist
- **GIVEN** no configured server alias named `staging-api-1`
- **WHEN** a caller requests `staging-api-1`
- **THEN** the broker rejects the request
- **AND** the broker does not attempt fallback, prefix matching, or environment inference

### Requirement: Multi-server registry
The system SHALL support multiple configured servers at the same time without ambiguity.

#### Scenario: Multiple aliases share environment
- **GIVEN** multiple configured production server aliases
- **WHEN** the broker lists or resolves servers
- **THEN** each alias remains independently addressable
- **AND** no host is selected without an explicit alias match
