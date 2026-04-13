## ADDED Requirements

### Requirement: Minimum inspection commands are implemented
The broker foundation SHALL implement `config validate`, `hosts list`, and `profiles list --server <alias>`.

#### Scenario: Validate succeeds
- **GIVEN** a valid broker configuration file
- **WHEN** the operator runs `agent-ssh config validate`
- **THEN** the CLI exits successfully
- **AND** it reports that the configuration is valid

#### Scenario: Profiles list is alias-scoped
- **GIVEN** a valid configured alias
- **WHEN** the operator runs `agent-ssh profiles list --server <alias>`
- **THEN** the CLI lists only profiles allowed for that alias
- **AND** the CLI rejects aliases that are not configured

### Requirement: Run command records a broker decision
The broker foundation SHALL implement `agent-ssh run` as an alias-based broker request that records the decision outcome.

#### Scenario: Run request is planned successfully
- **GIVEN** a valid alias, allowed profile, and complete argument set
- **WHEN** the operator runs `agent-ssh run --server staging-api --profile logs --arg service=api --arg since=10 min ago`
- **THEN** the broker validates and renders the request
- **AND** an audit event is written for the resulting decision
