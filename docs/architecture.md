# Architecture

## Overview

`agent-ssh` is split into small Rust crates so security-sensitive logic is centralized in broker code rather than distributed across interface layers.

## Crate Boundaries

- `agent-ssh-common`: config types, domain identifiers, validation, and shared error types
- `agent-ssh-broker`: alias registry, policy checks, safe profile rendering, signer abstraction, approval handling, and audit logging
- `agent-ssh-cli`: command-line interface for validation, inspection, and run planning
- `agent-ssh-mcp`: future agent-facing interface layer

## Request Flow

1. CLI loads and validates a TOML config.
2. Broker resolves the server by exact alias.
3. Broker verifies the requested profile is allowed for that server.
4. Broker checks approval requirements.
5. Broker renders the profile template using validated placeholder arguments.
6. Broker writes a structured audit event.
7. The executor runs system OpenSSH using the auth mode configured for that server.

## Security Boundaries

- Raw host details are configuration-only data.
- Agents interact through aliases and profile names.
- Profile templates use a restricted grammar so operators cannot accidentally configure shell pipelines and inline command substitution.
- Signer selection is separated from request validation so certificate issuance can be hardened independently.
- Audit writing is centralized in broker code so blocked and successful decisions follow the same record shape.

## Current Execution Status

This milestone ships publickey-only system OpenSSH as the secure default. It also includes a compatibility-only `legacy_password` lane that uses broker-managed askpass plus opaque secret references for migration scenarios. Full broker-managed short-lived certificate material is still a later milestone, and the code remains separated so signer-backed identity injection can land without weakening the foundation.
