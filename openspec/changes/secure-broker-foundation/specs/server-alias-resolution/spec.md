## ADDED Requirements

### Requirement: Alias listing and exact resolution
The broker foundation SHALL expose configured aliases for inspection and resolve only exact alias matches during request planning.

#### Scenario: List configured hosts
- **GIVEN** a valid broker configuration with multiple server aliases
- **WHEN** the operator runs `agent-ssh hosts list`
- **THEN** the CLI outputs the configured aliases
- **AND** the output is derived from the broker registry rather than raw host input

#### Scenario: Alias miss does not fall back
- **GIVEN** a request for an alias that is not configured
- **WHEN** the broker resolves the target server
- **THEN** the request is rejected
- **AND** no host, environment, or prefix fallback is attempted
