# approval-flow Specification

## Purpose

Define how the broker blocks protected actions until an explicit approval signal is present.

## Requirements

### Requirement: Protected actions require approval
The system SHALL require approval for actions marked as protected by broker policy.

#### Scenario: Server requires approval
- **GIVEN** a server is configured with `requires_approval = true`
- **WHEN** a caller requests a profile on that server without an approval reference
- **THEN** the broker rejects the request
- **AND** the rejection is audited

### Requirement: Approval metadata is auditable
The system SHALL include approval state in the audit trail for protected actions.

#### Scenario: Approved protected action
- **GIVEN** a protected request includes an approval reference accepted by policy
- **WHEN** the broker processes the request
- **THEN** the broker records that approval was supplied
- **AND** the approval metadata is associated with the audit record
