## ADDED Requirements

### Requirement: System OpenSSH transport is publickey-only and non-interactive
The broker SHALL invoke system OpenSSH in a mode that refuses password and keyboard-interactive fallbacks.

#### Scenario: Dry-run or execution builds an SSH command
- **GIVEN** a validated run plan is prepared for execution
- **WHEN** the broker renders the system `ssh` invocation
- **THEN** the invocation enables non-interactive batch behavior
- **AND** it disables password and keyboard-interactive authentication paths
- **AND** it does not invoke `sshpass`

### Requirement: Transport fails closed when publickey authentication is unavailable
The broker SHALL fail the run instead of retrying weaker or interactive authentication methods when publickey authentication cannot proceed.

#### Scenario: Approved publickey authentication cannot complete
- **GIVEN** a validated run plan cannot authenticate through the allowed publickey path
- **WHEN** the broker attempts execution
- **THEN** execution fails
- **AND** the broker does not retry with password or keyboard-interactive authentication

### Requirement: Transport mode remains auditable without storing secrets
The broker SHALL record the transport and auth mode used for planned or executed runs without storing secret credential material.

#### Scenario: Audit event records transport mode
- **GIVEN** the broker writes a plan or execution audit event
- **WHEN** the event is serialized
- **THEN** it includes the system SSH transport label and a publickey/certificate auth label
- **AND** it does not include passwords or reusable credential material
