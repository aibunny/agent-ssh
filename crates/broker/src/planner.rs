use std::{collections::BTreeMap, path::Path};

use agent_ssh_common::{
    AuditAction, AuditEvent, AuditOutcome, AuthMethod, Config, ProfileName, ServerConfig,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
    AuditLogger, BrokerError,
    executor::{self, CommandOutput},
    render::CompiledProfile,
};

#[derive(Debug, Clone)]
pub struct Broker {
    config: Config,
    compiled_profiles: BTreeMap<ProfileName, CompiledProfile>,
    secret_env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HostSummary {
    pub alias: String,
    pub environment: String,
    pub user: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone)]
pub struct ProfileSummary {
    pub name: String,
    pub description: Option<String>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRequest {
    pub actor: String,
    pub server_alias: String,
    pub profile: String,
    pub args: BTreeMap<String, String>,
    pub approval_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunPlan {
    pub server_alias: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub environment: String,
    pub signer: String,
    pub profile: String,
    pub rendered_command: String,
    pub requires_approval: bool,
    pub approval_provided: bool,
    pub execution_mode: ExecutionMode,
    /// How the broker authenticates to the remote server.
    pub auth_method: AuthMethod,
}

impl RunPlan {
    /// Human-readable label for the auth method, safe to print or log.
    pub fn auth_method_label(&self) -> String {
        match &self.auth_method {
            AuthMethod::Certificate => "certificate".to_string(),
            AuthMethod::LegacyPassword => "legacy_password".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    PlanOnly,
}

#[derive(Debug)]
pub struct AuditedOutcome<T> {
    pub result: Result<T, BrokerError>,
    pub audit_event: AuditEvent,
}

#[derive(Debug, Default)]
struct AuditContext {
    server_alias: Option<String>,
    environment: Option<String>,
    profile: Option<String>,
    args: BTreeMap<String, String>,
    rendered_command: Option<String>,
    requires_approval: bool,
    approval_reference: Option<String>,
    signer: Option<String>,
    transport: Option<String>,
    /// Kind label only; never exposes env var values or raw password material.
    auth_method_kind: Option<String>,
    exit_code: Option<i32>,
}

impl Broker {
    pub fn from_config(config: Config) -> Result<Self, BrokerError> {
        Self::from_config_with_secret_env(config, BTreeMap::new())
    }

    pub fn from_config_with_secret_env(
        config: Config,
        secret_env: BTreeMap<String, String>,
    ) -> Result<Self, BrokerError> {
        let mut compiled_profiles = BTreeMap::new();
        for (name, profile) in &config.profiles {
            let compiled = CompiledProfile::compile(name.as_str(), profile)?;
            compiled_profiles.insert(name.clone(), compiled);
        }

        Ok(Self {
            config,
            compiled_profiles,
            secret_env,
        })
    }

    pub fn audit_logger(&self) -> AuditLogger {
        AuditLogger::new(self.audit_log_path())
    }

    pub fn audit_log_path(&self) -> &Path {
        &self.config.broker.audit_log_path
    }

    pub fn config_validated_event(&self, actor: &str) -> AuditEvent {
        new_audit_event(
            actor,
            AuditAction::ConfigValidate,
            AuditOutcome::Succeeded,
            "configuration is valid".to_string(),
            AuditContext::default(),
        )
    }

    pub fn list_hosts(&self, actor: &str) -> AuditedOutcome<Vec<HostSummary>> {
        let hosts = self
            .config
            .servers
            .iter()
            .map(|(alias, server)| HostSummary {
                alias: alias.as_str().to_string(),
                environment: server.environment.clone(),
                user: server.user.clone(),
                requires_approval: self.server_requires_approval(server),
            })
            .collect::<Vec<_>>();

        let audit_event = new_audit_event(
            actor,
            AuditAction::HostsList,
            AuditOutcome::Succeeded,
            format!("listed {} configured hosts", hosts.len()),
            AuditContext::default(),
        );

        AuditedOutcome {
            result: Ok(hosts),
            audit_event,
        }
    }

    pub fn list_profiles(&self, actor: &str, alias: &str) -> AuditedOutcome<Vec<ProfileSummary>> {
        let result = match self.config.servers.get(alias) {
            Some(server) => Ok(self.profiles_for_server(server)),
            None => Err(BrokerError::UnknownServer {
                alias: alias.to_string(),
            }),
        };

        let audit_event = match &result {
            Ok(profiles) => new_audit_event(
                actor,
                AuditAction::ProfilesList,
                AuditOutcome::Succeeded,
                format!("listed {} profiles for server '{alias}'", profiles.len()),
                AuditContext {
                    server_alias: Some(alias.to_string()),
                    environment: self
                        .config
                        .servers
                        .get(alias)
                        .map(|server| server.environment.clone()),
                    requires_approval: self
                        .config
                        .servers
                        .get(alias)
                        .map(|server| self.server_requires_approval(server))
                        .unwrap_or(false),
                    ..AuditContext::default()
                },
            ),
            Err(error) => new_audit_event(
                actor,
                AuditAction::ProfilesList,
                AuditOutcome::Blocked,
                error.to_string(),
                AuditContext {
                    server_alias: Some(alias.to_string()),
                    ..AuditContext::default()
                },
            ),
        };

        AuditedOutcome {
            result,
            audit_event,
        }
    }

    pub fn plan_run(&self, request: RunRequest) -> AuditedOutcome<RunPlan> {
        let result = self.plan_run_inner(&request);
        let normalized_approval_reference =
            normalize_approval_reference(request.approval_reference.as_deref());
        let audit_event = match &result {
            Ok(plan) => new_audit_event(
                &request.actor,
                AuditAction::RunPlan,
                AuditOutcome::Planned,
                "request validated and execution plan created".to_string(),
                AuditContext {
                    server_alias: Some(plan.server_alias.clone()),
                    environment: Some(plan.environment.clone()),
                    profile: Some(plan.profile.clone()),
                    args: request.args.clone(),
                    rendered_command: Some(plan.rendered_command.clone()),
                    requires_approval: plan.requires_approval,
                    approval_reference: normalized_approval_reference.clone(),
                    signer: Some(plan.signer.clone()),
                    transport: Some("system_ssh".to_string()),
                    // Log kind label only; password values are never recorded.
                    auth_method_kind: Some(plan.auth_method_label()),
                    ..AuditContext::default()
                },
            ),
            Err(error) => {
                let server = self.config.servers.get(request.server_alias.as_str());
                new_audit_event(
                    &request.actor,
                    AuditAction::RunPlan,
                    AuditOutcome::Blocked,
                    error.to_string(),
                    AuditContext {
                        server_alias: Some(request.server_alias.clone()),
                        environment: server.map(|server| server.environment.clone()),
                        profile: Some(request.profile.clone()),
                        args: request.args.clone(),
                        requires_approval: self.requires_approval_for_request(&request),
                        approval_reference: normalized_approval_reference.clone(),
                        signer: server.map(|server| self.effective_signer_name(server).to_string()),
                        transport: Some("system_ssh".to_string()),
                        auth_method_kind: server.map(|s| auth_method_kind_label(&s.auth_method)),
                        ..AuditContext::default()
                    },
                )
            }
        };

        AuditedOutcome {
            result,
            audit_event,
        }
    }

    /// Plan **and execute** a run request, capturing all command output.
    ///
    /// Internally calls [`Self::plan_run`] first so every blocked or invalid
    /// request is recorded in the audit log before any SSH connection is
    /// attempted. If planning succeeds the SSH command is executed and a second
    /// audit event records the outcome, including the exit code.
    pub fn run(
        &self,
        request: RunRequest,
    ) -> (AuditedOutcome<RunPlan>, AuditedOutcome<CommandOutput>) {
        // Phase 1 — plan (policy + rendering).
        let plan_outcome = self.plan_run(request.clone());

        let plan = match &plan_outcome.result {
            Ok(plan) => plan.clone(),
            Err(_) => {
                // Planning failed; return a stub exec outcome pairing the same error.
                let blocked_event = new_audit_event(
                    &request.actor,
                    AuditAction::RunExecute,
                    AuditOutcome::Blocked,
                    "execution skipped because planning was blocked".to_string(),
                    AuditContext {
                        server_alias: Some(request.server_alias.clone()),
                        profile: Some(request.profile.clone()),
                        ..AuditContext::default()
                    },
                );
                let exec_outcome = AuditedOutcome {
                    result: Err(BrokerError::UnknownServer {
                        alias: request.server_alias.clone(),
                    }),
                    audit_event: blocked_event,
                };
                return (plan_outcome, exec_outcome);
            }
        };

        // Phase 2 — execute.
        let server = self
            .config
            .servers
            .get(plan.server_alias.as_str())
            .expect("planned server alias must exist");
        let exec_result = executor::execute_plan(&plan, server, &self.secret_env);

        let exec_event = match &exec_result {
            Ok(output) => new_audit_event(
                &request.actor,
                AuditAction::RunExecute,
                AuditOutcome::Executed,
                format!("command completed with exit code {}", output.exit_code),
                AuditContext {
                    server_alias: Some(plan.server_alias.clone()),
                    environment: Some(plan.environment.clone()),
                    profile: Some(plan.profile.clone()),
                    args: request.args.clone(),
                    rendered_command: Some(plan.rendered_command.clone()),
                    requires_approval: plan.requires_approval,
                    approval_reference: normalize_approval_reference(
                        request.approval_reference.as_deref(),
                    ),
                    signer: Some(plan.signer.clone()),
                    transport: Some("system_ssh".to_string()),
                    auth_method_kind: Some(plan.auth_method_label()),
                    exit_code: Some(output.exit_code),
                },
            ),
            Err(error) => new_audit_event(
                &request.actor,
                AuditAction::RunExecute,
                AuditOutcome::Failed,
                error.to_string(),
                AuditContext {
                    server_alias: Some(plan.server_alias.clone()),
                    environment: Some(plan.environment.clone()),
                    profile: Some(plan.profile.clone()),
                    args: request.args.clone(),
                    rendered_command: Some(plan.rendered_command.clone()),
                    requires_approval: plan.requires_approval,
                    signer: Some(plan.signer.clone()),
                    transport: Some("system_ssh".to_string()),
                    auth_method_kind: Some(plan.auth_method_label()),
                    ..AuditContext::default()
                },
            ),
        };

        (
            plan_outcome,
            AuditedOutcome {
                result: exec_result,
                audit_event: exec_event,
            },
        )
    }

    fn plan_run_inner(&self, request: &RunRequest) -> Result<RunPlan, BrokerError> {
        let server = self
            .config
            .servers
            .get(request.server_alias.as_str())
            .ok_or_else(|| BrokerError::UnknownServer {
                alias: request.server_alias.clone(),
            })?;

        let profile = self
            .config
            .profiles
            .get(request.profile.as_str())
            .ok_or_else(|| BrokerError::UnknownProfile {
                profile: request.profile.clone(),
            })?;

        if !server
            .allowed_profiles
            .iter()
            .any(|profile_name| profile_name.as_str() == request.profile)
        {
            return Err(BrokerError::ProfileNotAllowed {
                server: request.server_alias.clone(),
                profile: request.profile.clone(),
            });
        }

        let requires_approval = self.server_requires_approval(server) || profile.requires_approval;
        let approval_reference =
            normalize_approval_reference(request.approval_reference.as_deref());
        if requires_approval && approval_reference.is_none() {
            return Err(BrokerError::ApprovalRequired {
                server: request.server_alias.clone(),
                profile: request.profile.clone(),
            });
        }

        let compiled = self
            .compiled_profiles
            .get(request.profile.as_str())
            .ok_or_else(|| BrokerError::UnknownProfile {
                profile: request.profile.clone(),
            })?;

        let rendered_command = compiled.render(request.profile.as_str(), &request.args)?;

        Ok(RunPlan {
            server_alias: request.server_alias.clone(),
            host: server.host.clone(),
            port: server.port,
            user: server.user.clone(),
            environment: server.environment.clone(),
            signer: self.effective_signer_name(server).to_string(),
            profile: request.profile.clone(),
            rendered_command,
            requires_approval,
            approval_provided: approval_reference.is_some(),
            execution_mode: ExecutionMode::PlanOnly,
            auth_method: server.auth_method.clone(),
        })
    }

    fn profiles_for_server(&self, server: &ServerConfig) -> Vec<ProfileSummary> {
        server
            .allowed_profiles
            .iter()
            .filter_map(|profile_name| {
                self.config
                    .profiles
                    .get(profile_name.as_str())
                    .map(|profile| ProfileSummary {
                        name: profile_name.as_str().to_string(),
                        description: profile.description.clone(),
                        requires_approval: self.server_requires_approval(server)
                            || profile.requires_approval,
                    })
            })
            .collect()
    }

    fn effective_signer_name<'a>(&'a self, server: &'a ServerConfig) -> &'a str {
        server
            .signer
            .as_ref()
            .unwrap_or(&self.config.broker.default_signer)
            .as_str()
    }

    fn requires_approval_for_request(&self, request: &RunRequest) -> bool {
        let server_requires_approval = self
            .config
            .servers
            .get(request.server_alias.as_str())
            .map(|server| self.server_requires_approval(server))
            .unwrap_or(false);
        let profile_requires_approval = self
            .config
            .profiles
            .get(request.profile.as_str())
            .map(|profile| profile.requires_approval)
            .unwrap_or(false);

        server_requires_approval || profile_requires_approval
    }

    fn server_requires_approval(&self, server: &ServerConfig) -> bool {
        server.requires_approval || matches!(server.auth_method, AuthMethod::LegacyPassword)
    }
}

fn auth_method_kind_label(method: &AuthMethod) -> String {
    match method {
        AuthMethod::Certificate => "certificate".to_string(),
        AuthMethod::LegacyPassword => "legacy_password".to_string(),
    }
}

fn normalize_approval_reference(value: Option<&str>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn new_audit_event(
    actor: &str,
    action: AuditAction,
    outcome: AuditOutcome,
    message: String,
    context: AuditContext,
) -> AuditEvent {
    AuditEvent {
        event_id: Uuid::new_v4(),
        occurred_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        actor: actor.to_string(),
        action,
        outcome,
        message,
        server_alias: context.server_alias,
        environment: context.environment,
        profile: context.profile,
        args: context.args,
        rendered_command: context.rendered_command,
        requires_approval: context.requires_approval,
        approval_reference: context.approval_reference,
        signer: context.signer,
        transport: context.transport,
        auth_method_kind: context.auth_method_kind,
        exit_code: context.exit_code,
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use agent_ssh_common::{AuditOutcome, AuthMethod, parse_config};
    use tempfile::tempdir;

    use super::{Broker, RunRequest};

    const VALID_CONFIG: &str = r#"
[broker]
cert_ttl_seconds = 120
audit_log_path = "./data/test-audit.jsonl"
default_signer = "step_ca"

[signers.step_ca]
kind = "step-ca"

[servers.staging-api]
host = "10.0.1.10"
user = "deploy"
environment = "staging"
allowed_profiles = ["logs", "disk"]

[servers.prod-web-1]
host = "10.0.10.21"
user = "deploy"
environment = "production"
allowed_profiles = ["logs"]
requires_approval = true

[profiles.logs]
template = "journalctl -u {{service}} --since {{since}} --no-pager"

[profiles.disk]
template = "df -h"
"#;

    const LEGACY_PASSWORD_CONFIG: &str = r#"
[broker]
cert_ttl_seconds = 120
audit_log_path = "./data/test-audit.jsonl"
default_signer = "step_ca"

[signers.step_ca]
kind = "step-ca"

[servers.legacy-db]
host = "10.0.5.12"
user = "deploy"
environment = "migration"
allowed_profiles = ["disk"]
auth_method = "legacy_password"
password_secret_ref_env_var = "AGENT_SSH_LEGACY_DB_PASSWORD_REF"
legacy_password_acknowledged = true
fail2ban_allowlist_confirmed = true

[profiles.disk]
template = "df -h"
"#;

    fn fixture_broker() -> Broker {
        let config = match parse_config(VALID_CONFIG) {
            Ok(config) => config,
            Err(error) => panic!("valid config should parse: {error}"),
        };

        match Broker::from_config(config) {
            Ok(broker) => broker,
            Err(error) => panic!("valid broker should initialize: {error}"),
        }
    }

    // ── Core planning ────────────────────────────────────────────────────────

    #[test]
    fn plan_run_escapes_arguments_and_marks_plan_only() {
        let broker = fixture_broker();
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());
        args.insert("since".to_string(), "10 min ago".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "logs".to_string(),
            args,
            approval_reference: None,
        });

        let plan = match outcome.result {
            Ok(plan) => plan,
            Err(error) => panic!("plan should succeed: {error}"),
        };

        assert_eq!(
            plan.rendered_command,
            "journalctl -u 'api' --since '10 min ago' --no-pager"
        );
        assert!(matches!(outcome.audit_event.outcome, AuditOutcome::Planned));
    }

    #[test]
    fn blocks_missing_approval() {
        let broker = fixture_broker();
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());
        args.insert("since".to_string(), "10 min ago".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "prod-web-1".to_string(),
            profile: "logs".to_string(),
            args,
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        assert!(matches!(outcome.audit_event.outcome, AuditOutcome::Blocked));
        assert!(outcome.audit_event.requires_approval);
    }

    #[test]
    fn accepts_valid_approval_reference() {
        let broker = fixture_broker();
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "nginx".to_string());
        args.insert("since".to_string(), "5 min ago".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "prod-web-1".to_string(),
            profile: "logs".to_string(),
            args,
            approval_reference: Some("CAB-9999".to_string()),
        });

        let plan = outcome.result.expect("plan with approval should succeed");
        assert!(plan.approval_provided);
        assert!(plan.requires_approval);
    }

    #[test]
    fn writes_jsonl_audit_records() {
        let mut broker = fixture_broker();
        let tempdir = match tempdir() {
            Ok(tempdir) => tempdir,
            Err(error) => panic!("tempdir should be created: {error}"),
        };
        broker.config.broker.audit_log_path = tempdir.path().join("audit.jsonl");
        let logger = broker.audit_logger();
        let event = broker.config_validated_event("test");

        if let Err(error) = logger.append(&event) {
            panic!("audit append should succeed: {error}");
        }

        let contents = match fs::read_to_string(tempdir.path().join("audit.jsonl")) {
            Ok(contents) => contents,
            Err(error) => panic!("audit log should be readable: {error}"),
        };

        assert!(contents.contains("\"action\":\"config_validate\""));
        assert!(contents.contains("\"outcome\":\"succeeded\""));
    }

    #[test]
    fn rejects_blank_approval_references() {
        let broker = fixture_broker();
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());
        args.insert("since".to_string(), "10 min ago".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "prod-web-1".to_string(),
            profile: "logs".to_string(),
            args,
            approval_reference: Some("   ".to_string()),
        });

        assert!(outcome.result.is_err());
        assert!(outcome.audit_event.requires_approval);
        assert_eq!(outcome.audit_event.approval_reference, None);
    }

    #[test]
    fn rejects_unknown_alias_exactly() {
        let broker = fixture_broker();
        let outcome = broker.list_profiles("test", "staging");

        assert!(outcome.result.is_err());
    }

    #[test]
    fn rejects_profile_not_allowed_for_server() {
        let broker = fixture_broker();
        // "logs" is not in staging-api's allowed_profiles... wait, it is.
        // Use a profile that is not allowed: "disk" is allowed for staging-api but not prod-web-1.
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "nginx".to_string());
        args.insert("since".to_string(), "1 min ago".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "prod-web-1".to_string(),
            profile: "disk".to_string(), // not in prod-web-1.allowed_profiles
            args,
            approval_reference: Some("CAB-1".to_string()),
        });

        assert!(outcome.result.is_err());
        let err_msg = outcome.result.expect_err("expected error").to_string();
        assert!(err_msg.contains("not allowed"), "{err_msg}");
    }

    #[test]
    fn rejects_unknown_profile() {
        let broker = fixture_broker();
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "nonexistent".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
    }

    #[test]
    fn rejects_unknown_server_in_run() {
        let broker = fixture_broker();
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "does-not-exist".to_string(),
            profile: "logs".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        let err_msg = outcome.result.expect_err("expected error").to_string();
        assert!(err_msg.contains("not configured"), "{err_msg}");
    }

    // ── Auth method propagation ──────────────────────────────────────────────

    #[test]
    fn plan_carries_certificate_auth_method_by_default() {
        let broker = fixture_broker();
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        let plan = outcome.result.expect("plan should succeed");
        assert_eq!(plan.auth_method, AuthMethod::Certificate);
        assert_eq!(plan.auth_method_label(), "certificate");
    }

    #[test]
    fn audit_log_records_certificate_auth_method_kind() {
        let mut broker = fixture_broker();
        let tempdir = tempdir().expect("tempdir");
        broker.config.broker.audit_log_path = tempdir.path().join("audit.jsonl");
        let logger = broker.audit_logger();

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        logger.append(&outcome.audit_event).expect("append");

        let contents = fs::read_to_string(tempdir.path().join("audit.jsonl")).expect("read");

        assert!(
            contents.contains("\"auth_method_kind\":\"certificate\""),
            "certificate auth label should be logged: {contents}"
        );
        assert!(
            !contents.contains("sshpass"),
            "password-oriented transport details must not appear in audit log: {contents}"
        );
    }

    #[test]
    fn legacy_password_runs_require_approval_even_when_server_does_not_flag_it() {
        let config = parse_config(LEGACY_PASSWORD_CONFIG).expect("legacy config");
        let broker = Broker::from_config(config).expect("broker");

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "legacy-db".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        assert!(outcome.audit_event.requires_approval);
        assert_eq!(
            outcome.audit_event.auth_method_kind.as_deref(),
            Some("legacy_password")
        );
    }

    #[test]
    fn plan_carries_legacy_password_auth_method_and_redacted_audit_kind() {
        let config = parse_config(LEGACY_PASSWORD_CONFIG).expect("legacy config");
        let broker = Broker::from_config(config).expect("broker");

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "legacy-db".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: Some("CAB-77".to_string()),
        });

        let plan = outcome.result.expect("plan should succeed");
        assert_eq!(plan.auth_method, AuthMethod::LegacyPassword);
        assert_eq!(plan.auth_method_label(), "legacy_password");
        assert_eq!(
            outcome.audit_event.auth_method_kind.as_deref(),
            Some("legacy_password")
        );
        let audit_debug = format!("{:?}", outcome.audit_event);
        assert!(
            !audit_debug.contains("AGENT_SSH_LEGACY_DB_PASSWORD_REF"),
            "env var names must not appear in audit output: {audit_debug}"
        );
        assert!(
            !audit_debug.contains("os_keychain:"),
            "secret references must not appear in audit output: {audit_debug}"
        );
    }

    #[test]
    fn list_hosts_marks_legacy_password_server_as_approval_required() {
        let config = parse_config(LEGACY_PASSWORD_CONFIG).expect("legacy config");
        let broker = Broker::from_config(config).expect("broker");
        let outcome = broker.list_hosts("test");
        let hosts = outcome.result.expect("hosts");
        let legacy = hosts
            .iter()
            .find(|host| host.alias == "legacy-db")
            .expect("legacy-db host");
        assert!(legacy.requires_approval);
    }

    // ── Injection / adversarial inputs ──────────────────────────────────────

    #[test]
    fn rejects_extra_args_not_in_template() {
        let broker = fixture_broker();
        let mut args = BTreeMap::new();
        // "disk" template has no placeholders.
        args.insert("injected".to_string(), "value".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "disk".to_string(),
            args,
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        let err = outcome.result.expect_err("expected error").to_string();
        assert!(err.contains("unexpected argument"), "{err}");
    }

    #[test]
    fn rejects_missing_required_arg() {
        let broker = fixture_broker();
        // "logs" requires service and since; supply only service.
        let mut args = BTreeMap::new();
        args.insert("service".to_string(), "api".to_string());

        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "logs".to_string(),
            args,
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        let err = outcome.result.expect_err("expected error").to_string();
        assert!(err.contains("missing required argument"), "{err}");
    }

    #[test]
    fn audit_event_does_not_contain_rendered_command_on_block() {
        let broker = fixture_broker();
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "prod-web-1".to_string(),
            profile: "logs".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });

        assert!(outcome.result.is_err());
        // On a blocked request the rendered command must be absent.
        assert_eq!(outcome.audit_event.rendered_command, None);
    }

    #[test]
    fn list_hosts_returns_all_servers() {
        let broker = fixture_broker();
        let outcome = broker.list_hosts("test");
        let hosts = outcome.result.expect("list_hosts should succeed");
        assert_eq!(hosts.len(), 2);
        assert!(hosts.iter().any(|host| host.alias == "staging-api"));
        assert!(hosts.iter().any(|host| host.alias == "prod-web-1"));
    }

    #[test]
    fn list_profiles_returns_allowed_profiles_only() {
        let broker = fixture_broker();
        let outcome = broker.list_profiles("test", "prod-web-1");
        let profiles = outcome.result.expect("list_profiles should succeed");
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "logs");
    }
}
