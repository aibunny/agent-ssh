## ADDED Requirements

### Requirement: Foundation configuration schema
The broker foundation SHALL define a TOML schema with top-level broker settings, named servers, named profiles, and named signers.

#### Scenario: Foundation config contains multiple servers
- **GIVEN** a TOML document with multiple `[servers.<alias>]` tables
- **WHEN** the broker parses the configuration
- **THEN** each server is loaded into the broker registry by alias
- **AND** the broker preserves broker-wide settings such as audit log path and certificate TTL

### Requirement: Insecure configuration is rejected before startup
The broker foundation SHALL fail startup when the configuration violates security constraints.

#### Scenario: Server references root login
- **GIVEN** a server entry configured with `user = "root"`
- **WHEN** startup validation runs
- **THEN** the broker refuses to start
- **AND** the validation error identifies the offending alias

#### Scenario: Server references unknown signer
- **GIVEN** a server entry references a signer name that is not configured
- **WHEN** startup validation runs
- **THEN** the broker refuses to start
- **AND** the validation error identifies the missing signer
