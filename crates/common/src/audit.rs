use std::collections::BTreeMap;

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    ConfigValidate,
    HostsList,
    ProfilesList,
    RunPlan,
    RunExecute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Succeeded,
    Blocked,
    Invalid,
    Planned,
    /// SSH command was sent to the remote host and output was captured.
    /// Check `exit_code` to determine whether the command itself succeeded.
    Executed,
    /// Broker could not initiate the SSH connection (not a remote command failure).
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditEvent {
    pub event_id: Uuid,
    pub occurred_at: String,
    pub actor: String,
    pub action: AuditAction,
    pub outcome: AuditOutcome,
    pub message: String,
    pub server_alias: Option<String>,
    pub environment: Option<String>,
    pub profile: Option<String>,
    pub args: BTreeMap<String, String>,
    pub rendered_command: Option<String>,
    pub requires_approval: bool,
    pub approval_reference: Option<String>,
    pub signer: Option<String>,
    pub transport: Option<String>,
    /// The auth method kind label for the run (for example, "certificate").
    pub auth_method_kind: Option<String>,
    /// Exit code from the remote command. Present only for `run_execute` events.
    pub exit_code: Option<i32>,
}
