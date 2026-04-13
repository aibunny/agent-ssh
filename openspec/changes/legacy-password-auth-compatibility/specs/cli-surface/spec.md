## ADDED Requirements

### Requirement: CLI secret-reference loading is redaction-safe
The CLI SHALL resolve only opaque secret-reference variables for legacy password auth and SHALL not print or echo those values.

#### Scenario: `.env` provides a secret reference
- **GIVEN** the resolved config path has a sibling `.env` file
- **AND** it defines the environment variable named by `password_secret_ref_env_var`
- **WHEN** the CLI builds the broker runtime context
- **THEN** the secret reference is available for execution
- **AND** the raw password is still not stored in the CLI config model or output

#### Scenario: Dry run for legacy password mode
- **GIVEN** a legacy password server is selected for `agent-ssh exec --dry-run`
- **WHEN** the CLI prints the planned invocation
- **THEN** the output identifies the auth method as `legacy_password`
- **AND** it does not include plaintext password material or secret-reference values
