//! Agent-facing interface helpers for `agent-ssh`.
//!
//! This crate wraps the broker core with config loading, dotenv secret-ref
//! resolution, audit persistence, and session helpers so agents can safely use
//! profile-based runs or broker-held persistent SSH sessions.

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use agent_ssh_broker::{
    Broker, BrokerError, CommandOutput, HostSummary, OpenSessionRequest, ProfileSummary, RunPlan,
    RunRequest, SessionExecRequest,
};
use agent_ssh_common::{ConfigError, SessionMode, SessionRecord, load_config};
use thiserror::Error;

/// Successful profile execution with both the validated plan and command output.
#[derive(Debug, Clone)]
pub struct ProfileRunResult {
    pub plan: RunPlan,
    pub output: CommandOutput,
}

/// User-controlled settings for unrestricted command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommandSettings {
    /// Explicit user opt-in for arbitrary commands.
    pub allow_arbitrary_commands: bool,
    /// Reuse the newest matching unrestricted session when possible.
    pub reuse_existing_connection: bool,
    /// Optional requested TTL when opening a new unrestricted session.
    pub ttl_seconds: Option<u64>,
    /// Optional requested idle timeout when opening a new unrestricted session.
    pub idle_timeout_seconds: Option<u64>,
}

impl Default for AgentCommandSettings {
    fn default() -> Self {
        Self {
            allow_arbitrary_commands: false,
            reuse_existing_connection: true,
            ttl_seconds: None,
            idle_timeout_seconds: None,
        }
    }
}

/// Result of an unrestricted command execution driven by user settings.
#[derive(Debug, Clone)]
pub struct AgentCommandResult {
    pub session_id: String,
    pub reused_session: bool,
    pub output: CommandOutput,
}

/// Agent-friendly wrapper around the broker and session manager.
#[derive(Debug, Clone)]
pub struct AgentSshClient {
    actor: String,
    broker: Broker,
}

impl AgentSshClient {
    /// Load broker config plus sibling `.env` secret references.
    pub fn from_config_path(
        config_path: impl AsRef<Path>,
        actor: impl Into<String>,
    ) -> Result<Self, AgentSshError> {
        Self::from_config_path_with_env_file(config_path, Option::<&Path>::None, actor)
    }

    /// Load broker config plus an optional explicit `.env` file override.
    pub fn from_config_path_with_env_file<P: AsRef<Path>>(
        config_path: impl AsRef<Path>,
        env_file: Option<P>,
        actor: impl Into<String>,
    ) -> Result<Self, AgentSshError> {
        let config_path = config_path.as_ref().to_path_buf();
        let config = load_config(&config_path)?;
        let secret_env =
            build_secret_env(&config_path, env_file.as_ref().map(|path| path.as_ref()))?;
        let broker = Broker::from_config_with_secret_env(config, secret_env)?;

        Ok(Self {
            actor: actor.into(),
            broker,
        })
    }

    pub fn actor(&self) -> &str {
        &self.actor
    }

    pub fn list_hosts(&self) -> Result<Vec<HostSummary>, AgentSshError> {
        self.record_outcome(self.broker.list_hosts(&self.actor))
    }

    pub fn list_profiles(&self, server_alias: &str) -> Result<Vec<ProfileSummary>, AgentSshError> {
        self.record_outcome(self.broker.list_profiles(&self.actor, server_alias))
    }

    /// Execute a configured profile and return both the broker plan and output.
    pub fn run_profile(
        &self,
        server_alias: impl Into<String>,
        profile: impl Into<String>,
        args: BTreeMap<String, String>,
        approval_reference: Option<String>,
    ) -> Result<ProfileRunResult, AgentSshError> {
        let request = RunRequest {
            actor: self.actor.clone(),
            server_alias: server_alias.into(),
            profile: profile.into(),
            args,
            approval_reference,
        };

        let (plan_outcome, exec_outcome) = self.broker.run(request);
        self.append_audit_event(&plan_outcome.audit_event)?;
        self.append_audit_event(&exec_outcome.audit_event)?;

        let plan = plan_outcome.result?;
        let output = exec_outcome.result?;

        Ok(ProfileRunResult { plan, output })
    }

    /// Open a broker-controlled SSH session.
    pub fn open_session(
        &self,
        server_alias: impl Into<String>,
        mode: SessionMode,
        ttl_seconds: Option<u64>,
        idle_timeout_seconds: Option<u64>,
        approval_reference: Option<String>,
    ) -> Result<SessionRecord, AgentSshError> {
        let manager = self.broker.session_manager();
        let (result, event) = manager.open_session(OpenSessionRequest {
            actor: self.actor.clone(),
            server_alias: server_alias.into(),
            mode,
            ttl_seconds,
            idle_timeout_seconds,
            approval_reference,
        });
        self.append_audit_event(&event)?;
        result.map_err(Into::into)
    }

    /// Convenience wrapper for arbitrary-command sessions.
    pub fn open_unrestricted_session(
        &self,
        server_alias: impl Into<String>,
        approval_reference: Option<String>,
        ttl_seconds: Option<u64>,
        idle_timeout_seconds: Option<u64>,
    ) -> Result<SessionRecord, AgentSshError> {
        self.open_session(
            server_alias,
            SessionMode::Unrestricted,
            ttl_seconds,
            idle_timeout_seconds,
            approval_reference,
        )
    }

    /// Execute a command in an existing session.
    pub fn exec_session(
        &self,
        session_id: impl Into<String>,
        profile: Option<String>,
        args: BTreeMap<String, String>,
        command: Option<String>,
    ) -> Result<CommandOutput, AgentSshError> {
        let manager = self.broker.session_manager();
        let (result, event) = manager.exec_in_session(SessionExecRequest {
            actor: self.actor.clone(),
            session_id: session_id.into(),
            profile,
            args,
            command,
        });
        self.append_audit_event(&event)?;
        result.map_err(Into::into)
    }

    /// Run a named profile in a restricted session.
    pub fn exec_session_profile(
        &self,
        session_id: impl Into<String>,
        profile: impl Into<String>,
        args: BTreeMap<String, String>,
    ) -> Result<CommandOutput, AgentSshError> {
        self.exec_session(session_id, Some(profile.into()), args, None)
    }

    /// Run any raw command in an unrestricted session.
    pub fn exec_unrestricted(
        &self,
        session_id: impl Into<String>,
        command: impl Into<String>,
    ) -> Result<CommandOutput, AgentSshError> {
        self.exec_session(session_id, None, BTreeMap::new(), Some(command.into()))
    }

    /// Run any command when the user's settings explicitly allow it.
    ///
    /// If `reuse_existing_connection` is enabled, the client reuses the newest
    /// unrestricted session for the target server when possible, which keeps
    /// the SSH connection broker-held and avoids unnecessary reconnects.
    pub fn run_unrestricted_command_with_settings(
        &self,
        server_alias: impl Into<String>,
        command: impl Into<String>,
        settings: &AgentCommandSettings,
        approval_reference: Option<String>,
    ) -> Result<AgentCommandResult, AgentSshError> {
        if !settings.allow_arbitrary_commands {
            return Err(AgentSshError::ArbitraryCommandsDisabledBySettings);
        }

        let server_alias = server_alias.into();
        let command = command.into();

        if settings.reuse_existing_connection
            && let Some(session) = self.reusable_unrestricted_session_for_server(&server_alias)
        {
            match self.exec_unrestricted(session.id.clone(), command.clone()) {
                Ok(output) => {
                    return Ok(AgentCommandResult {
                        session_id: session.id,
                        reused_session: true,
                        output,
                    });
                }
                Err(AgentSshError::Broker(BrokerError::SessionExpired { .. }))
                | Err(AgentSshError::Broker(BrokerError::SessionIdleTimeout { .. }))
                | Err(AgentSshError::Broker(BrokerError::SessionNotFound { .. })) => {}
                Err(error) => return Err(error),
            }
        }

        let session = self.open_unrestricted_session(
            server_alias,
            approval_reference,
            settings.ttl_seconds,
            settings.idle_timeout_seconds,
        )?;
        let output = self.exec_unrestricted(session.id.clone(), command)?;

        Ok(AgentCommandResult {
            session_id: session.id,
            reused_session: false,
            output,
        })
    }

    pub fn close_session(&self, session_id: &str) -> Result<(), AgentSshError> {
        let manager = self.broker.session_manager();
        let (result, event) = manager.close_session(session_id, &self.actor);
        self.append_audit_event(&event)?;
        result.map_err(Into::into)
    }

    /// List currently persisted sessions after cleaning up expired entries.
    pub fn list_sessions(&self) -> Vec<SessionRecord> {
        self.broker.session_manager().list_sessions()
    }

    fn record_outcome<T>(
        &self,
        outcome: agent_ssh_broker::AuditedOutcome<T>,
    ) -> Result<T, AgentSshError> {
        self.append_audit_event(&outcome.audit_event)?;
        outcome.result.map_err(Into::into)
    }

    fn append_audit_event(
        &self,
        event: &agent_ssh_common::AuditEvent,
    ) -> Result<(), AgentSshError> {
        self.broker.audit_logger().append(event)?;
        Ok(())
    }

    fn reusable_unrestricted_session_for_server(
        &self,
        server_alias: &str,
    ) -> Option<SessionRecord> {
        self.list_sessions()
            .into_iter()
            .filter(|session| {
                session.server_alias == server_alias && session.mode == SessionMode::Unrestricted
            })
            .max_by_key(|session| session.last_used_at_unix)
    }
}

#[derive(Debug, Error)]
pub enum AgentSshError {
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("{0}")]
    Broker(#[from] BrokerError),
    #[error("arbitrary commands are disabled by the user's settings")]
    ArbitraryCommandsDisabledBySettings,
    #[error("failed to read dotenv file at {path}: {source}")]
    DotenvRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid .env line {line_number}: {reason}")]
    DotenvInvalidLine { line_number: usize, reason: String },
}

fn build_secret_env(
    config_path: &Path,
    env_file: Option<&Path>,
) -> Result<BTreeMap<String, String>, AgentSshError> {
    let mut env_map = env::vars().collect::<BTreeMap<_, _>>();
    let dotenv_path = env_file
        .map(Path::to_path_buf)
        .unwrap_or_else(|| resolve_dotenv_path(config_path));

    if !dotenv_path.is_file() {
        return Ok(env_map);
    }

    let source = fs::read_to_string(&dotenv_path).map_err(|source| AgentSshError::DotenvRead {
        path: dotenv_path.clone(),
        source,
    })?;
    let dotenv_values = parse_dotenv(&source)?;

    for (name, value) in dotenv_values {
        env_map.entry(name).or_insert(value);
    }

    Ok(env_map)
}

fn resolve_dotenv_path(config_path: &Path) -> PathBuf {
    if let Ok(path) = env::var("AGENT_SSH_ENV_FILE") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".env")
}

fn parse_dotenv(source: &str) -> Result<BTreeMap<String, String>, AgentSshError> {
    let mut values = BTreeMap::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
        let Some((name, raw_value)) = line.split_once('=') else {
            return Err(AgentSshError::DotenvInvalidLine {
                line_number,
                reason: "expected KEY=VALUE".to_string(),
            });
        };
        let name = name.trim();
        if !is_valid_env_var_name(name) {
            return Err(AgentSshError::DotenvInvalidLine {
                line_number,
                reason: format!("'{name}' is not a valid environment variable name"),
            });
        }

        let value = normalize_dotenv_value(raw_value.trim()).map_err(|reason| {
            AgentSshError::DotenvInvalidLine {
                line_number,
                reason,
            }
        })?;
        values.insert(name.to_string(), value);
    }

    Ok(values)
}

fn normalize_dotenv_value(value: &str) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }

    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        if value.len() < 2 {
            return Err("quoted values must be balanced".to_string());
        }
        return Ok(value[1..value.len() - 1].to_string());
    }

    if value.starts_with('"')
        || value.starts_with('\'')
        || value.ends_with('"')
        || value.ends_with('\'')
    {
        return Err("quoted values must start and end with the same quote".to_string());
    }

    Ok(value.to_string())
}

fn is_valid_env_var_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !matches!(first, 'A'..='Z' | 'a'..='z' | '_') {
        return false;
    }

    chars.all(|char| matches!(char, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_'))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use agent_ssh_common::{SessionMode, SessionRecord};

    use super::{AgentCommandSettings, AgentSshClient};

    fn write_config(tempdir: &tempfile::TempDir) -> PathBuf {
        let config_path = tempdir.path().join("agent-ssh.toml");
        let audit_log_path = tempdir.path().join("audit.jsonl");
        let config = format!(
            r#"
[broker]
cert_ttl_seconds = 120
audit_log_path = "{}"
default_signer = "step_ca"

[signers.step_ca]
kind = "step-ca"

[servers.staging-api]
host = "10.0.1.10"
user = "deploy"
environment = "staging"
allowed_profiles = ["disk"]
requires_approval = true
allow_unrestricted_sessions = true

[profiles.disk]
template = "df -h"
"#,
            audit_log_path.display()
        );
        fs::write(&config_path, config).expect("write config");
        config_path
    }

    fn read_audit_log(tempdir: &tempfile::TempDir) -> String {
        fs::read_to_string(tempdir.path().join("audit.jsonl")).expect("read audit log")
    }

    fn write_session_record(tempdir: &tempfile::TempDir, record: &SessionRecord) {
        let sessions_dir = tempdir.path().join("sessions");
        fs::create_dir_all(&sessions_dir).expect("sessions dir");
        let path = sessions_dir.join(format!("{}.json", record.id));
        let body = serde_json::to_string_pretty(record).expect("serialize session");
        fs::write(path, body).expect("write session");
    }

    #[test]
    fn open_unrestricted_session_requires_approval_and_records_audit() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let config_path = write_config(&tempdir);
        let client = AgentSshClient::from_config_path(&config_path, "agent").expect("client");

        let result = client.open_unrestricted_session("staging-api", None, None, None);

        assert!(result.is_err(), "missing approval should be rejected");
        let audit = read_audit_log(&tempdir);
        assert!(audit.contains("\"action\":\"session_open\""), "{audit}");
        assert!(audit.contains("\"outcome\":\"denied\""), "{audit}");
        assert!(audit.contains("approval reference"), "{audit}");
    }

    #[test]
    fn exec_unrestricted_records_missing_session_errors() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let config_path = write_config(&tempdir);
        let client = AgentSshClient::from_config_path(&config_path, "agent").expect("client");

        let result = client.exec_unrestricted("missing-session", "uname -a");

        assert!(result.is_err(), "missing session should be rejected");
        let audit = read_audit_log(&tempdir);
        assert!(audit.contains("\"action\":\"session_command\""), "{audit}");
        assert!(
            audit.contains("\"session_id\":\"missing-session\""),
            "{audit}"
        );
    }

    #[test]
    fn unrestricted_commands_can_be_disabled_by_user_settings() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let config_path = write_config(&tempdir);
        let client = AgentSshClient::from_config_path(&config_path, "agent").expect("client");
        let settings = AgentCommandSettings::default();

        let result = client.run_unrestricted_command_with_settings(
            "staging-api",
            "uname -a",
            &settings,
            Some("CAB-1".to_string()),
        );

        assert!(matches!(
            result,
            Err(super::AgentSshError::ArbitraryCommandsDisabledBySettings)
        ));
    }

    #[test]
    fn reuses_newest_unrestricted_session_for_server() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let config_path = write_config(&tempdir);
        let client = AgentSshClient::from_config_path(&config_path, "agent").expect("client");

        write_session_record(
            &tempdir,
            &SessionRecord {
                id: "older-session".to_string(),
                server_alias: "staging-api".to_string(),
                host: "10.0.1.10".to_string(),
                port: 22,
                user: "deploy".to_string(),
                environment: "staging".to_string(),
                auth_method_kind: "certificate".to_string(),
                mode: SessionMode::Unrestricted,
                opened_at_unix: 4_000_000_000,
                last_used_at_unix: 4_000_000_100,
                ttl_seconds: 3600,
                idle_timeout_seconds: 600,
                approval_reference: Some("CAB-1".to_string()),
                control_socket_path: "/tmp/older-session.sock".to_string(),
            },
        );
        write_session_record(
            &tempdir,
            &SessionRecord {
                id: "newest-session".to_string(),
                server_alias: "staging-api".to_string(),
                host: "10.0.1.10".to_string(),
                port: 22,
                user: "deploy".to_string(),
                environment: "staging".to_string(),
                auth_method_kind: "certificate".to_string(),
                mode: SessionMode::Unrestricted,
                opened_at_unix: 4_000_000_000,
                last_used_at_unix: 4_000_000_500,
                ttl_seconds: 3600,
                idle_timeout_seconds: 600,
                approval_reference: Some("CAB-1".to_string()),
                control_socket_path: "/tmp/newest-session.sock".to_string(),
            },
        );
        write_session_record(
            &tempdir,
            &SessionRecord {
                id: "restricted-session".to_string(),
                server_alias: "staging-api".to_string(),
                host: "10.0.1.10".to_string(),
                port: 22,
                user: "deploy".to_string(),
                environment: "staging".to_string(),
                auth_method_kind: "certificate".to_string(),
                mode: SessionMode::Restricted,
                opened_at_unix: 4_000_000_000,
                last_used_at_unix: 4_000_000_999,
                ttl_seconds: 3600,
                idle_timeout_seconds: 600,
                approval_reference: None,
                control_socket_path: "/tmp/restricted-session.sock".to_string(),
            },
        );

        let selected = client
            .reusable_unrestricted_session_for_server("staging-api")
            .expect("should find unrestricted session");

        assert_eq!(selected.id, "newest-session");
    }
}
