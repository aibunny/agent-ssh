/// Broker-controlled interactive SSH session management.
///
/// Sessions use SSH ControlMaster multiplexing: a background master process holds
/// the connection while the broker sends individual commands over it. Every session
/// lifecycle event and every command execution is written to the audit log.
use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use agent_ssh_common::{
    AuditAction, AuditEvent, AuditOutcome, AuthMethod, Config, ProfileName, ServerConfig,
    SessionMode, SessionRecord,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::{AuditLogger, BrokerError, executor::CommandOutput, render::CompiledProfile};

/// Maximum TTL a caller may request for a session.
const MAX_SESSION_TTL_SECONDS: u64 = 3600;
/// Default session TTL when none is specified.
const DEFAULT_SESSION_TTL_SECONDS: u64 = 300;
/// Default idle timeout when none is specified.
const DEFAULT_IDLE_TIMEOUT_SECONDS: u64 = 60;
/// Maximum raw command length accepted in unrestricted mode.
const MAX_COMMAND_LEN: usize = 4096;
/// Maximum output captured per command (bytes). Excess is silently truncated.
const MAX_OUTPUT_BYTES: usize = 1_048_576; // 1 MiB

/// Request to open a new interactive session.
pub struct OpenSessionRequest {
    pub actor: String,
    pub server_alias: String,
    pub mode: SessionMode,
    pub ttl_seconds: Option<u64>,
    pub idle_timeout_seconds: Option<u64>,
    pub approval_reference: Option<String>,
}

/// Request to execute a command inside an open session.
pub struct SessionExecRequest {
    pub actor: String,
    pub session_id: String,
    /// Profile name — used in Restricted mode.
    pub profile: Option<String>,
    /// Profile arguments — used in Restricted mode.
    pub args: BTreeMap<String, String>,
    /// Raw command — used in Unrestricted mode.
    pub command: Option<String>,
}

/// Manages the lifecycle of broker-controlled SSH sessions.
pub struct SessionManager {
    config: Config,
    compiled_profiles: BTreeMap<ProfileName, CompiledProfile>,
    data_dir: PathBuf,
}

impl SessionManager {
    pub fn new(
        config: Config,
        compiled_profiles: BTreeMap<ProfileName, CompiledProfile>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            config,
            compiled_profiles,
            data_dir,
        }
    }

    pub fn audit_logger(&self) -> AuditLogger {
        AuditLogger::new(&self.config.broker.audit_log_path)
    }

    /// Open a new session and return the session record.
    ///
    /// Validates policy, establishes the SSH ControlMaster, persists the
    /// session record, and emits an audit event. Returns the record so the
    /// caller can display the session ID.
    pub fn open_session(
        &self,
        request: OpenSessionRequest,
    ) -> (Result<SessionRecord, BrokerError>, AuditEvent) {
        let result = self.open_session_inner(&request);
        let event = self.session_open_audit_event(&request, &result);
        (result, event)
    }

    fn open_session_inner(
        &self,
        request: &OpenSessionRequest,
    ) -> Result<SessionRecord, BrokerError> {
        let server = self
            .config
            .servers
            .get(request.server_alias.as_str())
            .ok_or_else(|| BrokerError::UnknownServer {
                alias: request.server_alias.clone(),
            })?;

        // Validate unrestricted session policy.
        if request.mode == SessionMode::Unrestricted {
            if !server.allow_unrestricted_sessions {
                return Err(BrokerError::UnrestrictedSessionNotAllowed {
                    server: request.server_alias.clone(),
                });
            }
            if !server.requires_approval {
                return Err(BrokerError::UnrestrictedSessionRequiresServerApprovalFlag {
                    server: request.server_alias.clone(),
                });
            }
            if request
                .approval_reference
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err(BrokerError::UnrestrictedSessionRequiresApproval {
                    server: request.server_alias.clone(),
                });
            }
        }

        // Legacy password sessions are not supported for ControlMaster.
        if matches!(server.auth_method, AuthMethod::LegacyPassword) {
            return Err(BrokerError::SessionCommandDenied {
                server: request.server_alias.clone(),
                reason: "legacy_password auth is not supported for persistent sessions; \
                         use `agent-ssh exec` instead"
                    .to_string(),
            });
        }

        let ttl = request
            .ttl_seconds
            .unwrap_or(DEFAULT_SESSION_TTL_SECONDS)
            .min(MAX_SESSION_TTL_SECONDS);
        let idle_timeout = request
            .idle_timeout_seconds
            .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECONDS);

        let id = Uuid::new_v4();
        let id_str = id.to_string();
        let id_hex = id.simple().to_string();
        let socket_path = format!("/tmp/agent-ssh-{}.sock", &id_hex[..8]);

        let now = now_unix();

        let session = SessionRecord {
            id: id_str,
            server_alias: request.server_alias.clone(),
            host: server.host.clone(),
            port: server.port,
            user: server.user.clone(),
            environment: server.environment.clone(),
            auth_method_kind: auth_method_kind_label(&server.auth_method),
            mode: request.mode,
            opened_at_unix: now,
            last_used_at_unix: now,
            ttl_seconds: ttl,
            idle_timeout_seconds: idle_timeout,
            approval_reference: normalize_approval_ref(request.approval_reference.as_deref()),
            control_socket_path: socket_path,
        };

        self.establish_master(&session, server)?;
        self.save_session(&session)?;

        Ok(session)
    }

    /// Execute a command in an existing session.
    pub fn exec_in_session(
        &self,
        request: SessionExecRequest,
    ) -> (Result<CommandOutput, BrokerError>, AuditEvent) {
        let session_snapshot = self.load_session(&request.session_id).ok();
        let result = self.exec_in_session_inner(&request);
        let event = self.session_command_audit_event(session_snapshot.as_ref(), &request, &result);
        (result, event)
    }

    fn exec_in_session_inner(
        &self,
        request: &SessionExecRequest,
    ) -> Result<CommandOutput, BrokerError> {
        let mut session = self.load_session(&request.session_id)?;
        let now = now_unix();

        if session.is_expired(now) {
            let _ = self.close_and_cleanup_session(&session);
            return Err(BrokerError::SessionExpired {
                id: request.session_id.clone(),
            });
        }

        if session.is_idle_timed_out(now) {
            let _ = self.close_and_cleanup_session(&session);
            return Err(BrokerError::SessionIdleTimeout {
                id: request.session_id.clone(),
            });
        }

        let command = self.resolve_command(&session, request)?;

        if !self.check_master_alive(&session) {
            let _ = self.close_and_cleanup_session(&session);
            return Err(BrokerError::SessionExpired {
                id: request.session_id.clone(),
            });
        }

        let raw_output = self.exec_via_master(&session, &command)?;

        // Truncate large output to prevent memory exhaustion.
        let output = truncate_output(raw_output);

        // Update last-used timestamp.
        session.last_used_at_unix = now_unix();
        self.save_session(&session)?;

        Ok(output)
    }

    /// Close an open session.
    pub fn close_session(
        &self,
        session_id: &str,
        actor: &str,
    ) -> (Result<(), BrokerError>, AuditEvent) {
        let session_snapshot = self.load_session(session_id).ok();
        let result = self.close_session_inner(session_id);
        let event =
            self.session_close_audit_event(session_snapshot.as_ref(), session_id, actor, &result);
        (result, event)
    }

    fn close_session_inner(&self, session_id: &str) -> Result<(), BrokerError> {
        let session = self.load_session(session_id)?;
        self.close_and_cleanup_session(&session)?;
        Ok(())
    }

    /// List all non-expired sessions and clean up expired ones.
    pub fn list_sessions(&self) -> Vec<SessionRecord> {
        let sessions_dir = self.sessions_dir();
        let Ok(entries) = fs::read_dir(&sessions_dir) else {
            return Vec::new();
        };

        let now = now_unix();
        let mut sessions = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            let Ok(session) = serde_json::from_str::<SessionRecord>(&content) else {
                continue;
            };

            if session.is_expired(now) || session.is_idle_timed_out(now) {
                let _ = self.close_and_cleanup_session(&session);
            } else {
                sessions.push(session);
            }
        }

        sessions
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }

    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{id}.json"))
    }

    fn save_session(&self, session: &SessionRecord) -> Result<(), BrokerError> {
        let dir = self.sessions_dir();
        fs::create_dir_all(&dir).map_err(|source| BrokerError::SessionRegistryIo {
            path: dir.clone(),
            source,
        })?;
        let path = self.session_path(&session.id);
        let content = serde_json::to_string_pretty(session)
            .map_err(|source| BrokerError::SessionRegistryParse { source })?;
        fs::write(&path, &content).map_err(|source| BrokerError::SessionRegistryIo { path, source })
    }

    fn load_session(&self, id: &str) -> Result<SessionRecord, BrokerError> {
        let path = self.session_path(id);
        let content = fs::read_to_string(&path).map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                BrokerError::SessionNotFound { id: id.to_string() }
            } else {
                BrokerError::SessionRegistryIo {
                    path: path.clone(),
                    source: error,
                }
            }
        })?;
        serde_json::from_str(&content)
            .map_err(|source| BrokerError::SessionRegistryParse { source })
    }

    fn cleanup_session(&self, session: &SessionRecord) -> Result<(), BrokerError> {
        let path = self.session_path(&session.id);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|source| BrokerError::SessionRegistryIo { path, source })?;
        }
        // Best-effort socket cleanup.
        let _ = fs::remove_file(&session.control_socket_path);
        Ok(())
    }

    fn close_and_cleanup_session(&self, session: &SessionRecord) -> Result<(), BrokerError> {
        self.close_master(session);
        self.cleanup_session(session)
    }

    fn establish_master(
        &self,
        session: &SessionRecord,
        server: &ServerConfig,
    ) -> Result<(), BrokerError> {
        let target = format!("{}@{}", session.user, session.host);
        let socket = &session.control_socket_path;

        let mut cmd = Command::new("ssh");
        cmd.args([
            "-f",
            "-N",
            "-M",
            "-o",
            "ControlPersist=yes",
            "-o",
            &format!("ControlPath={socket}"),
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-o",
            "ConnectTimeout=30",
            "-o",
            "ServerAliveInterval=30",
            "-o",
            "ServerAliveCountMax=3",
            "-o",
            "ForwardAgent=no",
            "-o",
            "ForwardX11=no",
            "-o",
            "PermitLocalCommand=no",
        ]);

        match server.auth_method {
            AuthMethod::Certificate => {
                cmd.args([
                    "-o",
                    "PreferredAuthentications=publickey",
                    "-o",
                    "PubkeyAuthentication=yes",
                    "-o",
                    "PasswordAuthentication=no",
                    "-o",
                    "KbdInteractiveAuthentication=no",
                    "-o",
                    "NumberOfPasswordPrompts=0",
                    "-o",
                    "IdentitiesOnly=yes",
                ]);
            }
            AuthMethod::LegacyPassword => {
                return Err(BrokerError::SessionCommandDenied {
                    server: session.server_alias.clone(),
                    reason: "legacy_password auth is not supported for persistent sessions"
                        .to_string(),
                });
            }
        }

        cmd.args(["-p", &session.port.to_string(), &target]);

        let output = cmd.output().map_err(|error| {
            if error.kind() == ErrorKind::NotFound {
                BrokerError::SshNotFound
            } else {
                BrokerError::SshIo { source: error }
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrokerError::SessionIo {
                source: std::io::Error::other(format!(
                    "SSH ControlMaster failed to establish: {stderr}"
                )),
            });
        }

        Ok(())
    }

    fn check_master_alive(&self, session: &SessionRecord) -> bool {
        if !Path::new(&session.control_socket_path).exists() {
            return false;
        }
        let target = format!("{}@{}", session.user, session.host);
        Command::new("ssh")
            .args([
                "-O",
                "check",
                "-o",
                &format!("ControlPath={}", session.control_socket_path),
                "-p",
                &session.port.to_string(),
                &target,
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn exec_via_master(
        &self,
        session: &SessionRecord,
        command: &str,
    ) -> Result<CommandOutput, BrokerError> {
        let target = format!("{}@{}", session.user, session.host);
        let output = Command::new("ssh")
            .args([
                "-o",
                "ControlMaster=no",
                "-o",
                &format!("ControlPath={}", session.control_socket_path),
                "-o",
                "BatchMode=yes",
                "-o",
                "ForwardAgent=no",
                "-o",
                "ForwardX11=no",
                "-p",
                &session.port.to_string(),
                &target,
                command,
            ])
            .output()
            .map_err(|error| {
                if error.kind() == ErrorKind::NotFound {
                    BrokerError::SshNotFound
                } else {
                    BrokerError::SshIo { source: error }
                }
            })?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    fn close_master(&self, session: &SessionRecord) {
        if !Path::new(&session.control_socket_path).exists() {
            return;
        }
        let target = format!("{}@{}", session.user, session.host);
        let _ = Command::new("ssh")
            .args([
                "-O",
                "exit",
                "-o",
                &format!("ControlPath={}", session.control_socket_path),
                "-p",
                &session.port.to_string(),
                &target,
            ])
            .output();
    }

    fn resolve_command(
        &self,
        session: &SessionRecord,
        request: &SessionExecRequest,
    ) -> Result<String, BrokerError> {
        match session.mode {
            SessionMode::Restricted => {
                let profile_name = request.profile.as_deref().ok_or_else(|| {
                    BrokerError::SessionCommandDenied {
                        server: session.server_alias.clone(),
                        reason: "restricted sessions require --profile".to_string(),
                    }
                })?;

                let server = self
                    .config
                    .servers
                    .get(session.server_alias.as_str())
                    .ok_or_else(|| BrokerError::UnknownServer {
                        alias: session.server_alias.clone(),
                    })?;

                if !server
                    .allowed_profiles
                    .iter()
                    .any(|p| p.as_str() == profile_name)
                {
                    return Err(BrokerError::ProfileNotAllowed {
                        server: session.server_alias.clone(),
                        profile: profile_name.to_string(),
                    });
                }

                let compiled = self.compiled_profiles.get(profile_name).ok_or_else(|| {
                    BrokerError::UnknownProfile {
                        profile: profile_name.to_string(),
                    }
                })?;

                compiled.render(profile_name, &request.args)
            }
            SessionMode::Unrestricted => {
                let cmd = request.command.as_deref().ok_or_else(|| {
                    BrokerError::SessionCommandDenied {
                        server: session.server_alias.clone(),
                        reason: "unrestricted sessions require --cmd".to_string(),
                    }
                })?;

                if cmd.len() > MAX_COMMAND_LEN {
                    return Err(BrokerError::SessionCommandTooLong {
                        length: cmd.len(),
                        max: MAX_COMMAND_LEN,
                    });
                }

                if cmd.chars().any(|c| c.is_control() && c != '\t') {
                    return Err(BrokerError::SessionCommandDenied {
                        server: session.server_alias.clone(),
                        reason: "command must not contain control characters".to_string(),
                    });
                }

                Ok(cmd.to_string())
            }
        }
    }

    // ── Audit event builders ─────────────────────────────────────────────────

    fn session_open_audit_event(
        &self,
        request: &OpenSessionRequest,
        result: &Result<SessionRecord, BrokerError>,
    ) -> AuditEvent {
        let (outcome, message, session_id, environment, auth_method_kind) = match result {
            Ok(session) => (
                AuditOutcome::Succeeded,
                format!(
                    "session '{}' opened on server '{}' (mode={}, ttl={}s)",
                    session.id, session.server_alias, session.mode, session.ttl_seconds,
                ),
                Some(session.id.clone()),
                Some(session.environment.clone()),
                Some(session.auth_method_kind.clone()),
            ),
            Err(error) => {
                let env = self
                    .config
                    .servers
                    .get(request.server_alias.as_str())
                    .map(|s| s.environment.clone());
                let kind = self
                    .config
                    .servers
                    .get(request.server_alias.as_str())
                    .map(|s| auth_method_kind_label(&s.auth_method));
                (AuditOutcome::Denied, error.to_string(), None, env, kind)
            }
        };

        new_audit_event(
            &request.actor,
            AuditAction::SessionOpen,
            outcome,
            message,
            request.server_alias.clone(),
            environment,
            session_id,
            normalize_approval_ref(request.approval_reference.as_deref()),
            auth_method_kind,
            None,
        )
    }

    fn session_command_audit_event(
        &self,
        session_snapshot: Option<&SessionRecord>,
        request: &SessionExecRequest,
        result: &Result<CommandOutput, BrokerError>,
    ) -> AuditEvent {
        let session = session_snapshot
            .cloned()
            .or_else(|| self.load_session(&request.session_id).ok());
        let server_alias = session
            .as_ref()
            .map(|s| s.server_alias.clone())
            .unwrap_or_else(|| "(unknown)".to_string());
        let environment = session.as_ref().map(|s| s.environment.clone());
        let auth_method_kind = session.as_ref().map(|s| s.auth_method_kind.clone());

        let (outcome, message, exit_code) = match result {
            Ok(output) => (
                AuditOutcome::Executed,
                format!(
                    "session command completed with exit code {}",
                    output.exit_code
                ),
                Some(output.exit_code),
            ),
            Err(error) => {
                let outcome = match error {
                    BrokerError::SessionCommandDenied { .. }
                    | BrokerError::SessionCommandTooLong { .. }
                    | BrokerError::ProfileNotAllowed { .. } => AuditOutcome::Denied,
                    BrokerError::SessionExpired { .. } | BrokerError::SessionIdleTimeout { .. } => {
                        AuditOutcome::Expired
                    }
                    _ => AuditOutcome::Failed,
                };
                (outcome, error.to_string(), None)
            }
        };

        new_audit_event(
            &request.actor,
            AuditAction::SessionCommand,
            outcome,
            message,
            server_alias,
            environment,
            Some(request.session_id.clone()),
            None,
            auth_method_kind,
            exit_code,
        )
    }

    fn session_close_audit_event(
        &self,
        session_snapshot: Option<&SessionRecord>,
        session_id: &str,
        actor: &str,
        result: &Result<(), BrokerError>,
    ) -> AuditEvent {
        let session = session_snapshot
            .cloned()
            .or_else(|| self.load_session(session_id).ok());
        let server_alias = session
            .as_ref()
            .map(|s| s.server_alias.clone())
            .unwrap_or_else(|| "(unknown)".to_string());
        let environment = session.as_ref().map(|s| s.environment.clone());
        let auth_method_kind = session.as_ref().map(|s| s.auth_method_kind.clone());

        let (outcome, message) = match result {
            Ok(()) => (
                AuditOutcome::Succeeded,
                format!("session '{session_id}' closed"),
            ),
            Err(error) => (AuditOutcome::Failed, error.to_string()),
        };

        new_audit_event(
            actor,
            AuditAction::SessionClose,
            outcome,
            message,
            server_alias,
            environment,
            Some(session_id.to_string()),
            None,
            auth_method_kind,
            None,
        )
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn auth_method_kind_label(method: &AuthMethod) -> String {
    match method {
        AuthMethod::Certificate => "certificate".to_string(),
        AuthMethod::LegacyPassword => "legacy_password".to_string(),
    }
}

fn normalize_approval_ref(value: Option<&str>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn now_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn truncate_output(mut output: CommandOutput) -> CommandOutput {
    if output.stdout.len() > MAX_OUTPUT_BYTES {
        output.stdout.truncate(MAX_OUTPUT_BYTES);
        output.stdout.push_str("\n[output truncated]\n");
    }
    if output.stderr.len() > MAX_OUTPUT_BYTES {
        output.stderr.truncate(MAX_OUTPUT_BYTES);
        output.stderr.push_str("\n[output truncated]\n");
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn new_audit_event(
    actor: &str,
    action: AuditAction,
    outcome: AuditOutcome,
    message: String,
    server_alias: String,
    environment: Option<String>,
    session_id: Option<String>,
    approval_reference: Option<String>,
    auth_method_kind: Option<String>,
    exit_code: Option<i32>,
) -> AuditEvent {
    AuditEvent {
        event_id: Uuid::new_v4(),
        occurred_at: now_rfc3339(),
        actor: actor.to_string(),
        action,
        outcome,
        message,
        server_alias: Some(server_alias),
        environment,
        profile: None,
        args: BTreeMap::new(),
        rendered_command: None,
        requires_approval: false,
        approval_reference,
        signer: None,
        transport: Some("system_ssh".to_string()),
        auth_method_kind,
        exit_code,
        session_id,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agent_ssh_common::{SessionMode, SessionRecord, parse_config};
    use tempfile::tempdir;

    use super::{OpenSessionRequest, SessionExecRequest, SessionManager};
    use crate::render::CompiledProfile;

    const UNRESTRICTED_CONFIG: &str = r#"
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
allowed_profiles = ["disk"]
requires_approval = true
allow_unrestricted_sessions = true

[servers.cert-only]
host = "10.0.1.11"
user = "deploy"
environment = "staging"
allowed_profiles = ["disk"]

[profiles.disk]
template = "df -h"
"#;

    fn fixture_manager() -> (SessionManager, tempfile::TempDir) {
        let config = parse_config(UNRESTRICTED_CONFIG).expect("config");
        let data_dir = tempdir().expect("tempdir");
        let mut compiled = BTreeMap::new();
        for (name, profile) in &config.profiles {
            compiled.insert(
                name.clone(),
                CompiledProfile::compile(name.as_str(), profile).expect("compiled"),
            );
        }
        let manager = SessionManager::new(config, compiled, data_dir.path().to_path_buf());
        (manager, data_dir)
    }

    #[test]
    fn session_record_ttl_detection() {
        let now = 1_000_000_i64;
        let session = SessionRecord {
            id: "test-id".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: now - 400,
            last_used_at_unix: now - 10,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: None,
            control_socket_path: "/tmp/test.sock".to_string(),
        };

        // 400 seconds elapsed > 300 TTL → expired
        assert!(session.is_expired(now), "session should be expired");
        // 10 seconds idle < 60 timeout → not idle-timed-out
        assert!(
            !session.is_idle_timed_out(now),
            "session should not be idle-timed-out"
        );
    }

    #[test]
    fn session_record_idle_timeout_detection() {
        let now = 1_000_000_i64;
        let session = SessionRecord {
            id: "test-id".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: now - 50,
            last_used_at_unix: now - 90,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: None,
            control_socket_path: "/tmp/test.sock".to_string(),
        };

        // 50 seconds elapsed < 300 TTL → not expired
        assert!(!session.is_expired(now), "session should not be expired");
        // 90 seconds idle > 60 timeout → idle-timed-out
        assert!(
            session.is_idle_timed_out(now),
            "session should be idle-timed-out"
        );
    }

    #[test]
    fn session_record_round_trips_through_json() {
        let record = SessionRecord {
            id: "abc123".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Unrestricted,
            opened_at_unix: 1_000_000,
            last_used_at_unix: 1_000_030,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: Some("CAB-42".to_string()),
            control_socket_path: "/tmp/agent-ssh-abc12345.sock".to_string(),
        };

        let json = serde_json::to_string(&record).expect("serialize");
        let round_tripped: SessionRecord = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(record, round_tripped);
        assert_eq!(round_tripped.mode, SessionMode::Unrestricted);
    }

    #[test]
    fn open_session_blocks_unrestricted_when_flag_not_set() {
        let (manager, _dir) = fixture_manager();
        let (result, event) = manager.open_session(OpenSessionRequest {
            actor: "test".to_string(),
            server_alias: "cert-only".to_string(),
            mode: SessionMode::Unrestricted,
            ttl_seconds: None,
            idle_timeout_seconds: None,
            approval_reference: Some("CAB-1".to_string()),
        });

        assert!(result.is_err());
        let err = result.expect_err("expected error").to_string();
        assert!(
            err.contains("allow_unrestricted_sessions"),
            "error should mention config flag: {err}"
        );
        assert!(
            matches!(event.outcome, agent_ssh_common::AuditOutcome::Denied),
            "audit outcome should be Denied"
        );
    }

    #[test]
    fn open_session_blocks_unrestricted_without_approval() {
        let (manager, _dir) = fixture_manager();
        let (result, _event) = manager.open_session(OpenSessionRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            mode: SessionMode::Unrestricted,
            ttl_seconds: None,
            idle_timeout_seconds: None,
            approval_reference: None,
        });

        assert!(result.is_err());
        let err = result.expect_err("expected error").to_string();
        assert!(
            err.contains("approval"),
            "error should mention approval: {err}"
        );
    }

    #[test]
    fn open_session_blocks_unrestricted_without_server_requires_approval_flag() {
        // staging-api has requires_approval = true — good
        // But if a server has allow_unrestricted_sessions=true but NOT requires_approval, it should fail
        // We test the cert-only server (no allow_unrestricted_sessions)
        let (manager, _dir) = fixture_manager();
        let (result, _) = manager.open_session(OpenSessionRequest {
            actor: "test".to_string(),
            server_alias: "cert-only".to_string(),
            mode: SessionMode::Unrestricted,
            ttl_seconds: None,
            idle_timeout_seconds: None,
            approval_reference: Some("CAB-1".to_string()),
        });

        assert!(result.is_err());
        // Should fail because allow_unrestricted_sessions is false
        assert!(
            result
                .expect_err("expected error")
                .to_string()
                .contains("allow_unrestricted_sessions")
        );
    }

    #[test]
    fn save_and_load_session_record() {
        let (manager, _dir) = fixture_manager();
        let record = SessionRecord {
            id: "save-load-test".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: 1_000_000,
            last_used_at_unix: 1_000_000,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: None,
            control_socket_path: "/tmp/test-save-load.sock".to_string(),
        };

        manager.save_session(&record).expect("save");
        let loaded = manager.load_session("save-load-test").expect("load");
        assert_eq!(loaded, record);
    }

    #[test]
    fn exec_in_session_fails_for_missing_session() {
        let (manager, _dir) = fixture_manager();
        let (result, event) = manager.exec_in_session(SessionExecRequest {
            actor: "test".to_string(),
            session_id: "nonexistent-session-id".to_string(),
            profile: None,
            args: BTreeMap::new(),
            command: Some("ls".to_string()),
        });

        assert!(result.is_err());
        assert!(
            result
                .expect_err("expected error")
                .to_string()
                .contains("not found")
        );
        assert_eq!(event.session_id.as_deref(), Some("nonexistent-session-id"));
    }

    #[test]
    fn exec_in_session_fails_for_expired_session() {
        let (manager, _dir) = fixture_manager();
        let expired_record = SessionRecord {
            id: "expired-test".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: 0, // epoch — definitely expired
            last_used_at_unix: 0,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: None,
            control_socket_path: "/tmp/expired-test.sock".to_string(),
        };
        manager.save_session(&expired_record).expect("save");

        let (result, event) = manager.exec_in_session(SessionExecRequest {
            actor: "test".to_string(),
            session_id: "expired-test".to_string(),
            profile: None,
            args: BTreeMap::new(),
            command: Some("ls".to_string()),
        });

        assert!(result.is_err());
        let err = result.expect_err("expected error").to_string();
        assert!(
            err.contains("expired"),
            "error should mention expiry: {err}"
        );
        assert_eq!(event.server_alias.as_deref(), Some("staging-api"));
        assert_eq!(event.session_id.as_deref(), Some("expired-test"));
        assert!(
            matches!(event.outcome, agent_ssh_common::AuditOutcome::Expired),
            "audit outcome should preserve expiry context"
        );
    }

    #[test]
    fn exec_in_session_rejects_command_exceeding_max_length() {
        let (manager, _dir) = fixture_manager();
        let session = SessionRecord {
            id: "length-test".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Unrestricted,
            opened_at_unix: 4_000_000_000,
            last_used_at_unix: 4_000_000_000,
            ttl_seconds: 86400,
            idle_timeout_seconds: 3600,
            approval_reference: Some("CAB-1".to_string()),
            control_socket_path: "/tmp/length-test.sock".to_string(),
        };
        manager.save_session(&session).expect("save");

        let long_cmd = "a".repeat(4097);
        let (result, _event) = manager.exec_in_session(SessionExecRequest {
            actor: "test".to_string(),
            session_id: "length-test".to_string(),
            profile: None,
            args: BTreeMap::new(),
            command: Some(long_cmd),
        });

        assert!(result.is_err());
        let err = result.expect_err("expected error").to_string();
        assert!(
            err.contains("exceeds maximum"),
            "error should mention length: {err}"
        );
    }

    #[test]
    fn exec_in_restricted_mode_rejects_disallowed_profile() {
        let (manager, _dir) = fixture_manager();
        let session = SessionRecord {
            id: "restricted-test".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: 4_000_000_000,
            last_used_at_unix: 4_000_000_000,
            ttl_seconds: 86400,
            idle_timeout_seconds: 3600,
            approval_reference: None,
            control_socket_path: "/tmp/restricted-test.sock".to_string(),
        };
        manager.save_session(&session).expect("save");

        let (result, _event) = manager.exec_in_session(SessionExecRequest {
            actor: "test".to_string(),
            session_id: "restricted-test".to_string(),
            profile: Some("nonexistent-profile".to_string()),
            args: BTreeMap::new(),
            command: None,
        });

        assert!(result.is_err());
        let err = result.expect_err("expected error").to_string();
        assert!(
            err.contains("not allowed") || err.contains("not configured"),
            "error should mention profile policy: {err}"
        );
    }

    #[test]
    fn close_session_keeps_audit_context_after_registry_cleanup() {
        let (manager, _dir) = fixture_manager();
        let session = SessionRecord {
            id: "close-test".to_string(),
            server_alias: "staging-api".to_string(),
            host: "10.0.1.10".to_string(),
            port: 22,
            user: "deploy".to_string(),
            environment: "staging".to_string(),
            auth_method_kind: "certificate".to_string(),
            mode: SessionMode::Restricted,
            opened_at_unix: 4_000_000_000,
            last_used_at_unix: 4_000_000_000,
            ttl_seconds: 300,
            idle_timeout_seconds: 60,
            approval_reference: None,
            control_socket_path: "/tmp/close-test.sock".to_string(),
        };
        manager.save_session(&session).expect("save");

        let (result, event) = manager.close_session("close-test", "test");

        result.expect("close should succeed");
        assert_eq!(event.server_alias.as_deref(), Some("staging-api"));
        assert_eq!(event.session_id.as_deref(), Some("close-test"));
        assert!(
            matches!(event.outcome, agent_ssh_common::AuditOutcome::Succeeded),
            "close should be audited as succeeded"
        );
        assert!(
            manager.load_session("close-test").is_err(),
            "session record should be removed after close"
        );
    }
}
