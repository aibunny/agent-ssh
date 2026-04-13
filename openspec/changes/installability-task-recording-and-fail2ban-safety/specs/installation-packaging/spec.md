## ADDED Requirements

### Requirement: Repository and package metadata use the canonical project identity
The repository SHALL publish install and packaging metadata under the canonical `aibunny/agent-ssh` identity.

#### Scenario: Install surfaces reference the canonical repository
- **GIVEN** an operator reads install instructions or distribution metadata
- **WHEN** they inspect repository URLs in docs, installers, or package metadata
- **THEN** those references point to `https://github.com/aibunny/agent-ssh`
- **AND** supported package metadata identifies `aibunny` as the project author or maintainer

### Requirement: Installation instructions match shipped package names and artifacts
The repository SHALL publish installation commands that match the actual package and binary artifacts produced by the workspace.

#### Scenario: Cargo installation uses the real package name
- **GIVEN** a user follows the documented Cargo install path
- **WHEN** they inspect the referenced package name
- **THEN** it matches the installable CLI package defined by the workspace
- **AND** the resulting installed binary name is `agent-ssh`

### Requirement: Linux and macOS distribution automation stays internally consistent
The repository SHALL keep Linux and macOS installer, formula, and release automation aligned.

#### Scenario: Release updates Homebrew formula checksums
- **GIVEN** release archives exist for the supported Linux and macOS targets
- **WHEN** release automation updates the Homebrew formula
- **THEN** each target checksum placeholder is replaced with the matching archive checksum
- **AND** the formula version and install instructions remain consistent with the released artifact names

#### Scenario: Default installer targets a privileged directory
- **GIVEN** the one-line installer uses its default installation directory
- **WHEN** the current user cannot write that directory
- **THEN** the installer performs privileged directory creation and copy only through the explicit escalation path
- **AND** the install flow does not fail earlier on an unprivileged directory-creation step
