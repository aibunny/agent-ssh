# cli-surface Specification

## Purpose

Define the minimum command-line interface for broker administration and broker-mediated run requests.

## Requirements

### Requirement: Configuration inspection commands
The system SHALL provide CLI commands to validate configuration and inspect configured aliases and profiles.

#### Scenario: Validate config
- **GIVEN** a broker configuration file
- **WHEN** the operator runs `agent-ssh config validate`
- **THEN** the CLI reports whether the configuration is valid

#### Scenario: List configured hosts
- **GIVEN** a broker configuration file with configured server aliases
- **WHEN** the operator runs `agent-ssh hosts list`
- **THEN** the CLI lists the configured aliases
- **AND** the CLI does not require raw host input

#### Scenario: List profiles for server
- **GIVEN** a configured server alias
- **WHEN** the operator runs `agent-ssh profiles list --server <alias>`
- **THEN** the CLI lists only the profiles allowed for that server

### Requirement: Alias-based run command
The system SHALL provide a run command that accepts a server alias, profile name, and named arguments.

#### Scenario: Valid run request
- **GIVEN** a valid alias, allowed profile, and complete set of arguments
- **WHEN** the operator runs `agent-ssh run --server <alias> --profile <name> --arg key=value`
- **THEN** the CLI invokes broker policy evaluation
- **AND** the CLI reports the resulting broker decision

#### Scenario: Raw host is supplied
- **GIVEN** a caller attempts to provide raw host details instead of a server alias
- **WHEN** the CLI parses the request
- **THEN** the CLI rejects the input
- **AND** the caller is directed to use a configured alias
