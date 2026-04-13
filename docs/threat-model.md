# Threat Model

## Assets

- Short-lived session credentials issued by future signer backends
- Broker configuration that maps aliases to infrastructure
- Audit records describing broker decisions and rendered commands
- Approval metadata for protected actions

## Trust Boundaries

- Untrusted caller input enters through the CLI or a future agent-facing layer.
- Configuration is trusted operator input but still validated because mistakes can weaken policy.
- Signer backends are privileged dependencies and must remain broker-controlled.
- Remote SSH servers are separate trust domains that must be configured to trust the broker’s user CA.

## Threats And Initial Mitigations

- Prompt injection: callers can only choose configured aliases, profiles, and named args; arbitrary shell is not accepted.
- Unsafe command composition: template parsing rejects shell metacharacters and renders placeholder values as single escaped arguments.
- Alias confusion: resolution is exact-match only and never falls back to raw host details.
- Credential persistence: signer-backed execution is deferred until ephemeral credential handling is implemented correctly; legacy password compatibility uses opaque secret references and askpass instead of storing raw passwords in agent-visible surfaces.
- Lateral movement: per-server allowed profile lists and approval gates constrain what a caller can request.
- Privilege escalation: root login is permitted only as a break-glass exception and should be reserved for tightly controlled administrative cases; prefer non-root users for routine access.
- Accidental production access: production aliases can be marked as approval-required and remain distinct from staging aliases.
- Weak auditability: broker decisions and rendered commands are written as structured JSONL records.
- Weak temp file handling: design requires restrictive permissions and explicit cleanup for future OpenSSH-compatible credential material.
- Fail2ban exposure: the secure default removes password and keyboard-interactive fallbacks, while the legacy password compatibility lane limits auth to a single password attempt and requires explicit operator acknowledgment plus fail2ban allowlisting guidance.
- Unsafe defaults: missing aliases, unknown profiles, extra args, and missing approvals all fail closed.

## Residual Risks

- Local JSONL audit files can still be altered by a privileged local user.
- Approval references are opaque strings in the foundation milestone, not cryptographically verified approvals.
- Template restrictions may need refinement as supported command profiles expand.
