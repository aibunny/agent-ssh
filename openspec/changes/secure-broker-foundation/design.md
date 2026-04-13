## Context

`agent-ssh` is a new SSH broker for AI agents, so early interface choices directly affect security posture. The first milestone must stand up the project around OpenSpec, exact server aliases, named command profiles, auditability, and a signer abstraction, while avoiding fake security claims about execution paths that are not yet fully implemented.

The design is informed by current OpenSpec workflow guidance and by authoritative SSH references:

- OpenSpec separates long-lived truth in `openspec/specs/` from proposed change deltas in `openspec/changes/`.
- OpenSSH user certificate trust is anchored on server-side `TrustedUserCAKeys`, and `AuthorizedPrincipalsFile` can further restrict accepted principals when certificates are used.
- Smallstep `step-ca` can initialize SSH host and user CAs and issue short-lived SSH certificates, which maps well to a pluggable signer backend.
- Rust temporary-file guidance warns that path-based temp resources need careful lifecycle and permission handling, especially when cleanup depends on destructors.

## Goals / Non-Goals

**Goals:**

- Establish OpenSpec as the source of truth from the first commit onward.
- Create durable capability specs plus a concrete foundation change for the first milestone.
- Build a Rust workspace with clean crate boundaries for configuration, policy, execution planning, and future agent integration.
- Implement secure TOML config parsing and validation for multiple named servers and named profiles.
- Enforce exact alias lookup, server profile allowlists, approval gating, and safe profile rendering.
- Write structured audit records for broker actions and blocked requests.
- Define signer and execution interfaces so later work can add real SSH certificate issuance without reshaping the public model.
- Capture one scoped Claude Code CLI review inside the active OpenSpec change.

**Non-Goals:**

- Shipping a full step-ca signer implementation in this milestone.
- Shipping a production-ready approval service or external workflow engine.
- Allowing arbitrary shell commands, fuzzy host resolution, password authentication, or root login.
- Claiming secure remote execution before short-lived certificate issuance is implemented end-to-end.

## Decisions

### 1. Keep OpenSpec artifacts in-repo and create them before feature code

OpenSpec will be initialized immediately, with:

- Long-lived capability truth in `openspec/specs/`
- The foundation change in `openspec/changes/secure-broker-foundation/`
- Project-wide conventions in `openspec/project.md`

This keeps Codex and Claude aligned through git-visible artifacts instead of chat-only plans.

Alternatives considered:

- Delay specs until after scaffolding. Rejected because it weakens the required change-first workflow.
- Keep planning in chat only. Rejected because it is not durable or reviewable.

### 2. Use a four-crate Rust workspace with strict responsibility boundaries

- `common`: config schema, parsing, validation, domain identifiers, and shared errors
- `broker`: policy evaluation, alias registry, safe profile rendering, signer trait, execution planning, approvals, and audit logging
- `cli`: clap-based command-line entry point
- `mcp`: reserved library crate for future agent-facing integration

This keeps security-sensitive policy logic out of the CLI and makes later agent interfaces reuse the same broker core.

Alternatives considered:

- Single binary crate. Rejected because policy and parsing logic would be harder to isolate and test.
- Make `mcp` concrete now. Rejected because the first milestone does not need protocol-specific surface area yet.

### 3. Treat profile templates as a strict tokenized command DSL, not arbitrary shell text

The config format will keep the user’s TOML direction with a `template` string, but the broker will parse templates using a restricted grammar:

- Tokens are separated by ASCII whitespace
- Literal tokens must match a conservative shell-safe character set
- Placeholders must be standalone tokens in the form `{{name}}`
- Placeholders may only appear in fixed option-value positions after a literal option token
- Shell metacharacters such as pipes, redirects, command substitution, subshells, and inline quoting are rejected

At render time, placeholder values are validated and shell-escaped as single arguments before joining the final command string.

This gives administrators a familiar template format without permitting arbitrary shell composition in the first secure release.

Alternatives considered:

- Raw shell templates. Rejected because they undermine the “no arbitrary shell by default” requirement.
- Model commands as structured argv arrays in TOML. Safer, but rejected for the foundation milestone because the prompt explicitly points toward template strings and the stricter parser already prevents dangerous shell constructs.

### 4. Enforce default-deny validation before runtime policy evaluation

Configuration loading will fail on:

- Missing broker, server, or profile sections
- Invalid alias or profile identifiers
- Undefined allowed profiles
- Duplicate or malformed placeholders
- `user = "root"`
- Empty hosts or out-of-range ports
- Invalid signer references when a named signer is required

Runtime policy evaluation will fail on:

- Unknown server aliases
- Profiles not allowed for the selected server
- Missing required args, extra args, or invalid argument values
- Missing approval references for protected requests

Alternatives considered:

- Soft warnings with execution fallback. Rejected because silent fallback is explicitly forbidden.

### 5. Represent approval as explicit broker policy input, not an implicit environment behavior

The first milestone will use a simple approval reference string supplied on protected runs. If a server or profile requires approval and no approval reference is present, the broker will block the action and audit the denial.

This supports the security model now while leaving room for a stronger approval backend later.

Alternatives considered:

- No approval handling until later. Rejected because the first milestone requires explicit approval flow coverage.
- Auto-approval for local CLI use. Rejected because it weakens policy semantics.

### 6. Separate signer planning from actual credential issuance

The broker will define a signer trait and execution-plan types now, but the first milestone will stop short of claiming full certificate-backed SSH execution. This is intentional: secure execution depends on signer implementation, OpenSSH invocation details, temporary credential storage, and server CA trust bootstrap.

The design target for later milestones is:

- Broker creates or receives short-lived session material
- Material is scoped to one request and explicit expiry
- OpenSSH is invoked by the broker, not by the agent directly
- Temporary files and directories are created with restrictive permissions and explicitly cleaned up

This avoids fake completeness while preserving a clean architecture boundary for phase 3 work.

Alternatives considered:

- Fake execution using existing local SSH identities. Rejected because it violates the credential model.
- Delay the signer interface entirely. Rejected because later work would then reshape the architecture after consumers exist.

### 7. Use append-only JSONL audit logs for the foundation milestone

Audit events will be serialized as JSON objects, one per line, with timestamps, action type, target alias, environment, approval state, rendered command, and outcome.

JSONL is easy to append, inspect, diff, and integrate with later ingestion pipelines. The design keeps the audit writer behind a broker interface so stronger integrity controls can be added later.

Alternatives considered:

- SQLite or remote logging first. Rejected for initial scope and dependency weight.
- Plain text logs. Rejected because they are harder to parse and verify.

### 8. Use Claude Code CLI only for bounded review tied to this change

The first specialist task will be a review of configuration-schema and command-rendering safety, using:

- `openspec/changes/secure-broker-foundation/proposal.md`
- `openspec/changes/secure-broker-foundation/design.md`
- `openspec/changes/secure-broker-foundation/tasks.md`
- Relevant spec deltas

Claude’s findings will be written back into a review note under this change and then verified by Codex before task state is updated.

## Risks / Trade-offs

- [Strict template parser may reject legitimate administrative commands] → Start conservative in the secure release and widen only with explicit spec changes and tests.
- [Local JSONL audit logs can still be tampered with by a privileged local operator] → Document the limitation now and design the audit interface for future remote or integrity-protected sinks.
- [Approval references are only syntactic in the first milestone] → Treat this as a foundation interface, not a full approval workflow, and make the limitation explicit in docs and specs.
- [Signer abstraction without implementation can create expectation mismatch] → Make CLI and docs explicit that execution planning exists before certificate-backed transport is complete.
- [Temporary credential cleanup is hard to guarantee on abnormal termination] → Prefer in-memory or unnamed temp files where possible later, and restrict named temporary files to short-lived, permission-controlled use when OpenSSH requires filesystem paths.

## Migration Plan

This is a greenfield repository, so migration is repository bootstrap rather than production rollout:

1. Initialize OpenSpec and write source-of-truth specs.
2. Scaffold the Rust workspace and repository docs.
3. Implement config parsing, validation, policy checks, rendering, and audit logging.
4. Validate build/test/spec state.
5. Add signer-backed execution in a follow-up change rather than retrofitting security-critical behavior into this foundation change.

Rollback is straightforward at this stage: revert the foundational repository changes and associated specs together.

## Open Questions

- Which step-ca authentication mode should the first signer implementation support: local provisioner key, OIDC, or delegated token flow?
- Should approval references become structured objects in TOML-backed policy, or stay opaque until an external approval system exists?
- Should profile templates eventually evolve from tokenized strings to explicit argv arrays once the user-facing experience is established?
