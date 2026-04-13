# command-profile-execution Specification

## Purpose

Define how named command profiles are selected, validated, rendered, and prepared for remote execution.

## Requirements

### Requirement: Named command profiles
The system SHALL expose named command profiles instead of arbitrary shell input in the first secure release.

#### Scenario: Allowed profile
- **GIVEN** a server allows the `logs` profile
- **WHEN** a caller requests the `logs` profile for that server
- **THEN** the broker continues policy evaluation for that profile

#### Scenario: Disallowed profile
- **GIVEN** a server does not allow the `disk` profile
- **WHEN** a caller requests the `disk` profile for that server
- **THEN** the broker rejects the request
- **AND** the broker does not attempt to run the command

### Requirement: Safe argument rendering
The system SHALL safely bind profile arguments into a validated command form before execution.

#### Scenario: Valid profile arguments
- **GIVEN** a profile template with placeholders for `service` and `since` in fixed option-value positions
- **WHEN** a caller provides values for `service` and `since`
- **THEN** the broker renders a single command using the configured template
- **AND** the rendered command preserves argument boundaries

#### Scenario: Unknown profile argument
- **GIVEN** a profile template that does not declare an argument named `path`
- **WHEN** a caller provides `path`
- **THEN** the broker rejects the request
- **AND** the rendered command is not produced

#### Scenario: Placeholder attempts to control executable or flags
- **GIVEN** a profile template places a placeholder in the executable or flag position
- **WHEN** the broker validates the profile
- **THEN** validation fails
- **AND** the profile is not available for request planning
