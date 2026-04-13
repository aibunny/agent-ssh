# broker-config Specification

## Purpose

Define the broker configuration model, loading behavior, and validation rules.

## Requirements

### Requirement: TOML broker configuration
The system SHALL load broker configuration from a TOML document that defines broker settings, named servers, named command profiles, and signer references.

#### Scenario: Valid configuration document
- **GIVEN** a TOML configuration containing `[broker]`, `[signers.<name>]`, `[servers.<alias>]`, and `[profiles.<name>]` tables
- **WHEN** the broker loads the document
- **THEN** the configuration is parsed into strongly typed broker settings
- **AND** the named signers, servers, and profiles are preserved for policy evaluation

### Requirement: Security-oriented validation
The system SHALL reject configuration that violates secure broker invariants.

#### Scenario: Root login without acknowledgement
- **GIVEN** a server entry configured with `user = "root"`
- **AND** `root_login_acknowledged` is absent or false
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports that root login requires an explicit break-glass acknowledgement

#### Scenario: Root login with acknowledgement
- **GIVEN** a server entry configured with `user = "root"`
- **AND** `root_login_acknowledged = true`
- **WHEN** the broker validates the configuration
- **THEN** validation succeeds
- **AND** the configuration remains visibly marked as a discouraged break-glass exception

#### Scenario: Undefined profile reference
- **GIVEN** a server entry that references a profile name not defined under `[profiles]`
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the broker reports the missing profile by name
