## ADDED Requirements

### Requirement: Signer selection is explicit in request planning
The broker foundation SHALL resolve a signer name before constructing an execution plan.

#### Scenario: Server uses broker default signer
- **GIVEN** a server does not override its signer
- **WHEN** the broker plans a run request
- **THEN** the broker uses the configured broker default signer name
- **AND** the execution plan records which signer was selected

### Requirement: Execution planning is separated from credential issuance
The broker foundation SHALL keep signer interfaces separate from profile rendering and alias policy.

#### Scenario: Signer implementation is unavailable
- **GIVEN** the broker can validate the request and render the command
- **WHEN** no concrete signer implementation is available for execution
- **THEN** the broker returns a clear not-yet-executable result
- **AND** the request remains audited
