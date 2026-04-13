## ADDED Requirements

### Requirement: Protected requests require explicit approval input
The broker foundation SHALL require an approval reference string for requests protected by server or profile policy.

#### Scenario: Protected server without approval reference
- **GIVEN** a server alias is configured with `requires_approval = true`
- **WHEN** the operator runs `agent-ssh run` without an approval reference
- **THEN** the broker blocks the request
- **AND** the CLI returns a policy error

### Requirement: Approval state is surfaced in results
The broker foundation SHALL surface whether approval was required and whether it was provided.

#### Scenario: Protected profile with approval reference
- **GIVEN** a protected request includes an approval reference
- **WHEN** the broker evaluates the request
- **THEN** the result indicates that approval was required
- **AND** the result records that an approval reference was supplied
