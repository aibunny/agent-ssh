/// SSH command execution.
///
/// Takes an already-validated, already-rendered [`RunPlan`] and executes it via
/// the system `ssh` binary. Certificate auth remains the secure default.
/// Legacy password auth is supported only through an explicit compatibility
/// mode that uses a broker-managed askpass helper and never places plaintext
/// password material into config files, CLI args, or audit output.
use std::{
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use agent_ssh_common::{AuthMethod, LegacyPasswordConfig, ServerConfig};
use tempfile::{Builder, TempDir};

use crate::{BrokerError, planner::RunPlan};

/// Output captured from a completed remote command.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output from the remote command (UTF-8, lossy).
    pub stdout: String,
    /// Standard error from the remote command (UTF-8, lossy).
    pub stderr: String,
    /// Exit code of the remote command, or -1 if it was killed by a signal.
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecretReference {
    service: String,
    account: String,
}

struct AskpassHelper {
    _tempdir: TempDir,
    script_path: PathBuf,
}

/// Execute the SSH command described by `plan` and return captured output.
///
/// Both stdout and stderr are captured and returned; neither is streamed to the
/// terminal. This ensures that every byte of output is available to the caller
/// (agent or human) without interleaving.
pub fn execute_plan(
    plan: &RunPlan,
    server: &ServerConfig,
    secret_env: &BTreeMap<String, String>,
) -> Result<CommandOutput, BrokerError> {
    match plan.auth_method {
        AuthMethod::Certificate => execute_with_certificate(plan),
        AuthMethod::LegacyPassword => {
            let legacy_password = server.legacy_password.as_ref().ok_or_else(|| {
                BrokerError::LegacyPasswordConfigMissing {
                    server: plan.server_alias.clone(),
                }
            })?;
            execute_with_legacy_password(plan, legacy_password, secret_env)
        }
    }
}

/// Return a human-readable description of the SSH command that *would* be run,
/// without actually running it. Useful for `--dry-run`.
pub fn describe_invocation(plan: &RunPlan) -> String {
    match plan.auth_method {
        AuthMethod::Certificate => {
            let args = cert_ssh_args(plan);
            format!("ssh {}", args.join(" "))
        }
        AuthMethod::LegacyPassword => {
            let args = legacy_password_ssh_args(plan);
            format!(
                "env SSH_ASKPASS=<broker-managed> SSH_ASKPASS_REQUIRE=force DISPLAY=agent-ssh:0 ssh {}",
                args.join(" ")
            )
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn execute_with_certificate(plan: &RunPlan) -> Result<CommandOutput, BrokerError> {
    let output = Command::new("ssh")
        .args(cert_ssh_args(plan))
        .stdin(Stdio::null())
        .output()
        .map_err(map_ssh_spawn_error)?;

    Ok(command_output_from_process(output))
}

fn execute_with_legacy_password(
    plan: &RunPlan,
    legacy_password: &LegacyPasswordConfig,
    secret_env: &BTreeMap<String, String>,
) -> Result<CommandOutput, BrokerError> {
    let secret_ref_value = secret_env
        .get(legacy_password.secret_ref_env_var.as_str())
        .ok_or_else(|| BrokerError::LegacyPasswordSecretRefMissing {
            server: plan.server_alias.clone(),
            env_var: legacy_password.secret_ref_env_var.clone(),
        })?;
    let secret_ref = parse_secret_reference(&plan.server_alias, secret_ref_value)?;
    let helper = create_askpass_helper(&secret_ref)?;

    let output = Command::new("ssh")
        .args(legacy_password_ssh_args(plan))
        .env("SSH_ASKPASS", helper.script_path())
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", "agent-ssh:0")
        .stdin(Stdio::null())
        .output()
        .map_err(map_ssh_spawn_error)?;

    Ok(command_output_from_process(output))
}

fn command_output_from_process(output: std::process::Output) -> CommandOutput {
    CommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

fn map_ssh_spawn_error(error: std::io::Error) -> BrokerError {
    if error.kind() == ErrorKind::NotFound {
        BrokerError::SshNotFound
    } else {
        BrokerError::SshIo { source: error }
    }
}

/// SSH arguments for certificate-authenticated sessions.
fn cert_ssh_args(plan: &RunPlan) -> Vec<String> {
    vec![
        // No interactive prompts — fail fast if no key/cert is available.
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "PreferredAuthentications=publickey".to_string(),
        "-o".to_string(),
        "PubkeyAuthentication=yes".to_string(),
        "-o".to_string(),
        "PasswordAuthentication=no".to_string(),
        "-o".to_string(),
        "KbdInteractiveAuthentication=no".to_string(),
        "-o".to_string(),
        "NumberOfPasswordPrompts=0".to_string(),
        "-o".to_string(),
        "IdentitiesOnly=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=30".to_string(),
        // Accept new host keys silently; reject changed ones (MITM protection).
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-p".to_string(),
        plan.port.to_string(),
        format!("{}@{}", plan.user, plan.host),
        plan.rendered_command.clone(),
    ]
}

fn legacy_password_ssh_args(plan: &RunPlan) -> Vec<String> {
    vec![
        // Legacy compatibility mode still remains non-interactive from the
        // caller perspective, but asks ssh to use password auth via askpass.
        "-o".to_string(),
        "BatchMode=no".to_string(),
        "-o".to_string(),
        "PreferredAuthentications=password".to_string(),
        "-o".to_string(),
        "PasswordAuthentication=yes".to_string(),
        "-o".to_string(),
        "PubkeyAuthentication=no".to_string(),
        "-o".to_string(),
        "KbdInteractiveAuthentication=no".to_string(),
        "-o".to_string(),
        "NumberOfPasswordPrompts=1".to_string(),
        "-o".to_string(),
        "ConnectTimeout=30".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=accept-new".to_string(),
        "-p".to_string(),
        plan.port.to_string(),
        format!("{}@{}", plan.user, plan.host),
        plan.rendered_command.clone(),
    ]
}

fn parse_secret_reference(server_alias: &str, value: &str) -> Result<SecretReference, BrokerError> {
    let trimmed = value.trim();
    let mut parts = trimmed.split(':');
    let Some(provider) = parts.next() else {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason: "secret reference must not be empty".to_string(),
        });
    };
    let Some(service) = parts.next() else {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason: "secret reference must have the form os_keychain:<service>:<account>"
                .to_string(),
        });
    };
    let Some(account) = parts.next() else {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason: "secret reference must have the form os_keychain:<service>:<account>"
                .to_string(),
        });
    };

    if parts.next().is_some() {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason: "secret reference must have exactly three ':'-separated parts".to_string(),
        });
    }

    if provider != "os_keychain" {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason: "only os_keychain secret references are currently supported".to_string(),
        });
    }

    if !is_safe_secret_reference_component(service) {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason:
                "secret reference service must use only ASCII letters, digits, '.', '_', or '-'"
                    .to_string(),
        });
    }

    if !is_safe_secret_reference_component(account) {
        return Err(BrokerError::LegacyPasswordSecretRefInvalid {
            server: server_alias.to_string(),
            reason:
                "secret reference account must use only ASCII letters, digits, '.', '_', or '-'"
                    .to_string(),
        });
    }

    Ok(SecretReference {
        service: service.to_string(),
        account: account.to_string(),
    })
}

fn is_safe_secret_reference_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|char| matches!(char, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-'))
}

fn create_askpass_helper(secret_ref: &SecretReference) -> Result<AskpassHelper, BrokerError> {
    let tempdir = Builder::new()
        .prefix("agent-ssh-askpass")
        .tempdir()
        .map_err(|source| BrokerError::LegacyPasswordAskpassIo { source })?;
    let script_path = tempdir.path().join("askpass.sh");
    write_askpass_script(&script_path, secret_ref)?;

    Ok(AskpassHelper {
        _tempdir: tempdir,
        script_path,
    })
}

fn write_askpass_script(path: &Path, secret_ref: &SecretReference) -> Result<(), BrokerError> {
    let script_body = askpass_script_body(secret_ref)?;
    let mut file =
        fs::File::create(path).map_err(|source| BrokerError::LegacyPasswordAskpassIo { source })?;
    file.write_all(script_body.as_bytes())
        .map_err(|source| BrokerError::LegacyPasswordAskpassIo { source })?;
    let mut permissions = file
        .metadata()
        .map_err(|source| BrokerError::LegacyPasswordAskpassIo { source })?
        .permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions)
        .map_err(|source| BrokerError::LegacyPasswordAskpassIo { source })?;
    Ok(())
}

fn askpass_script_body(secret_ref: &SecretReference) -> Result<String, BrokerError> {
    let command = match std::env::consts::OS {
        "macos" => format!(
            "exec security find-generic-password -w -s '{}' -a '{}'\n",
            secret_ref.service, secret_ref.account
        ),
        "linux" => format!(
            "exec secret-tool lookup service '{}' account '{}'\n",
            secret_ref.service, secret_ref.account
        ),
        other => {
            return Err(BrokerError::LegacyPasswordUnsupportedPlatform {
                platform: other.to_string(),
            });
        }
    };

    Ok(format!("#!/bin/sh\nset -eu\n{command}"))
}

impl AskpassHelper {
    fn script_path(&self) -> &Path {
        &self.script_path
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agent_ssh_common::{AuthMethod, parse_config};

    use super::{askpass_script_body, describe_invocation, execute_plan, parse_secret_reference};
    use crate::{Broker, RunRequest};

    const CERT_CONFIG: &str = r#"
[broker]
cert_ttl_seconds = 120
audit_log_path = "./data/test-audit.jsonl"
default_signer = "step_ca"

[signers.step_ca]
kind = "step-ca"

[servers.staging-api]
host = "10.0.1.10"
port = 2222
user = "deploy"
environment = "staging"
allowed_profiles = ["disk"]

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
port = 22
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

    fn broker(config_source: &str) -> Broker {
        let config = parse_config(config_source).expect("config");
        Broker::from_config(config).expect("broker")
    }

    #[test]
    fn describe_invocation_is_publickey_only_and_non_interactive() {
        let broker = broker(CERT_CONFIG);
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "staging-api".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: None,
        });
        let plan = outcome.result.expect("plan");

        assert_eq!(plan.auth_method, AuthMethod::Certificate);
        let desc = describe_invocation(&plan);
        assert!(
            desc.contains("BatchMode=yes"),
            "cert invocation should include BatchMode: {desc}"
        );
        assert!(
            desc.contains("PreferredAuthentications=publickey"),
            "invocation should prefer publickey auth only: {desc}"
        );
        assert!(
            desc.contains("PubkeyAuthentication=yes"),
            "invocation should enable publickey auth: {desc}"
        );
        assert!(
            desc.contains("PasswordAuthentication=no"),
            "invocation should disable password auth: {desc}"
        );
        assert!(
            desc.contains("KbdInteractiveAuthentication=no"),
            "invocation should disable keyboard-interactive auth: {desc}"
        );
        assert!(
            desc.contains("NumberOfPasswordPrompts=0"),
            "invocation should disable password prompts: {desc}"
        );
        assert!(
            desc.contains("IdentitiesOnly=yes"),
            "invocation should limit offered identities: {desc}"
        );
        assert!(
            desc.contains("-p 2222"),
            "should include custom port: {desc}"
        );
        assert!(
            desc.contains("deploy@10.0.1.10"),
            "should include user@host: {desc}"
        );
        assert!(
            desc.contains("df -h"),
            "should include rendered command: {desc}"
        );
        assert!(
            !desc.contains("SSH_ASKPASS"),
            "certificate invocation should not mention askpass: {desc}"
        );
    }

    #[test]
    fn describe_invocation_for_legacy_password_is_redacted() {
        let broker = broker(LEGACY_PASSWORD_CONFIG);
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "legacy-db".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: Some("CAB-100".to_string()),
        });
        let plan = outcome.result.expect("plan");

        assert_eq!(plan.auth_method, AuthMethod::LegacyPassword);
        let desc = describe_invocation(&plan);
        assert!(
            desc.contains("SSH_ASKPASS=<broker-managed>"),
            "legacy password invocation should be redacted: {desc}"
        );
        assert!(
            desc.contains("PreferredAuthentications=password"),
            "legacy password invocation should force password auth: {desc}"
        );
        assert!(
            desc.contains("NumberOfPasswordPrompts=1"),
            "legacy password invocation should allow only one prompt: {desc}"
        );
        assert!(
            !desc.contains("os_keychain:"),
            "secret references must not appear in dry-run output: {desc}"
        );
    }

    #[test]
    fn execute_plan_fails_closed_when_secret_reference_env_var_is_missing() {
        let config = parse_config(LEGACY_PASSWORD_CONFIG).expect("config");
        let broker = Broker::from_config(config.clone()).expect("broker");
        let outcome = broker.plan_run(RunRequest {
            actor: "test".to_string(),
            server_alias: "legacy-db".to_string(),
            profile: "disk".to_string(),
            args: BTreeMap::new(),
            approval_reference: Some("CAB-100".to_string()),
        });
        let plan = outcome.result.expect("plan");
        let server = config.servers.get("legacy-db").expect("server");
        let error = execute_plan(&plan, server, &BTreeMap::new()).expect_err("missing secret ref");
        assert!(
            error
                .to_string()
                .contains("legacy password secret reference env var"),
            "{error}"
        );
    }

    #[test]
    fn rejects_secret_reference_with_unsafe_characters() {
        let error = parse_secret_reference("legacy-db", "os_keychain:agent ssh:db/account")
            .expect_err("unsafe ref must fail");
        assert!(
            error
                .to_string()
                .contains("secret reference service must use only ASCII letters"),
            "{error}"
        );
    }

    #[test]
    fn askpass_script_body_contains_only_secret_store_lookup() {
        let secret_ref = parse_secret_reference("legacy-db", "os_keychain:agent-ssh:legacy-db")
            .expect("valid secret ref");
        let body = askpass_script_body(&secret_ref).expect("script body");

        assert!(body.starts_with("#!/bin/sh\nset -eu\n"), "{body}");
        assert!(
            body.contains("agent-ssh") && body.contains("legacy-db"),
            "script should contain only non-secret keychain identifiers: {body}"
        );
        assert!(
            !body.contains("supersecret"),
            "password material must never appear in askpass script: {body}"
        );
    }
}
