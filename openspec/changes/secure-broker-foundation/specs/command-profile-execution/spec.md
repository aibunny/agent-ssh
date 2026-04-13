## ADDED Requirements

### Requirement: Template grammar is shell-safe by construction
The broker foundation SHALL reject profile templates that contain unsafe shell composition primitives.

#### Scenario: Unsafe shell operator in template
- **GIVEN** a profile template containing a pipe, redirection, subshell, or command separator
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the profile is not available for execution planning

#### Scenario: Placeholder is used as executable or flag
- **GIVEN** a profile template places a placeholder in the executable or flag position
- **WHEN** the broker validates the configuration
- **THEN** validation fails
- **AND** the caller cannot use profile arguments to choose the executable or flags

### Requirement: Rendered commands preserve argument boundaries
The broker foundation SHALL render placeholder arguments as single escaped shell arguments.

#### Scenario: Argument contains spaces
- **GIVEN** a profile template token `{{since}}`
- **AND** the caller provides `since=10 min ago`
- **WHEN** the broker renders the command
- **THEN** the rendered command contains one escaped argument for the entire value
- **AND** the value does not create new shell tokens
