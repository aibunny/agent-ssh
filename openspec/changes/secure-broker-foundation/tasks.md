## 1. OpenSpec Foundation

- [x] 1.1 Create `openspec/project.md` and the long-lived capability specs for broker configuration, alias resolution, profile execution, signer abstraction, audit logging, approval flow, and CLI surface
- [x] 1.2 Author `secure-broker-foundation` proposal, design, and spec deltas before implementation
- [x] 1.3 Write repository docs for architecture, threat model, configuration, and collaboration

## 2. Repository Scaffold

- [x] 2.1 Create the Rust workspace, crate skeletons, and top-level project files (`README.md`, `LICENSE`, `.gitignore`, toolchain, lint, and dependency policy)
- [x] 2.2 Add example configuration, step-ca example notes, and helper scripts for local development and SSH CA trust bootstrap

## 3. Broker Foundation Implementation

- [x] 3.1 Implement TOML config loading, strong validation, and shared domain types in `crates/common`
- [x] 3.2 Implement alias registry, approval checks, safe profile rendering, signer/execution planning types, and audit logging in `crates/broker`
- [x] 3.3 Implement the minimum CLI commands in `crates/cli` for `config validate`, `hosts list`, `profiles list`, and `run`
- [x] 3.4 Add tests for config parsing, alias resolution, profile rendering, and audit logging behavior

## 4. Verification

- [x] 4.1 Run formatting, linting, tests, and OpenSpec validation
- [x] 4.2 Reconcile code, docs, and specs so the milestone state is internally consistent

## 5. Claude Specialist Collaboration

- [x] 5.1 Assign Claude Code CLI a bounded review of config-schema and command-rendering safety using the `secure-broker-foundation` OpenSpec artifacts as context
- [x] 5.2 Verify Claude’s findings, record them under this change, and update implementation or docs where needed
