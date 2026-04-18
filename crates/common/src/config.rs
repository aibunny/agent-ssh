use std::{collections::BTreeMap, fs, path::Path, path::PathBuf};

use serde::Deserialize;

use crate::{ConfigError, ProfileName, ServerAlias, SignerName, ValidationError};

// Conservative upper bounds that prevent abuse while accommodating real deployments.
const MAX_HOST_LEN: usize = 253;
const MAX_USER_LEN: usize = 32;
const MAX_ENVIRONMENT_LEN: usize = 64;
const MAX_TEMPLATE_LEN: usize = 4096;
/// How the broker authenticates to a remote server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    /// Issue a short-lived SSH certificate via the configured signer (default).
    Certificate,
    /// Use a broker-managed, compatibility-only password flow.
    LegacyPassword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyPasswordConfig {
    pub secret_ref_env_var: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub broker: BrokerConfig,
    pub signers: BTreeMap<SignerName, SignerConfig>,
    pub servers: BTreeMap<ServerAlias, ServerConfig>,
    pub profiles: BTreeMap<ProfileName, ProfileConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerConfig {
    pub cert_ttl_seconds: u64,
    pub audit_log_path: PathBuf,
    pub default_signer: SignerName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerConfig {
    pub kind: String,
    pub ca_url: Option<String>,
    pub provisioner: Option<String>,
    pub subject: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub environment: String,
    pub allowed_profiles: Vec<ProfileName>,
    pub requires_approval: bool,
    pub signer: Option<SignerName>,
    pub auth_method: AuthMethod,
    pub legacy_password: Option<LegacyPasswordConfig>,
    /// Whether unrestricted interactive sessions are allowed for this server.
    /// Requires `requires_approval = true` to be effective.
    pub allow_unrestricted_sessions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileConfig {
    pub description: Option<String>,
    pub template: String,
    pub requires_approval: bool,
}

pub fn load_config(path: impl AsRef<Path>) -> Result<Config, ConfigError> {
    let path = path.as_ref();
    let source = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    parse_config(&source)
}

pub fn parse_config(source: &str) -> Result<Config, ConfigError> {
    let raw: RawConfig = toml::from_str(source)?;
    validate_config(raw).map_err(ConfigError::from)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    broker: RawBrokerConfig,
    #[serde(default)]
    signers: BTreeMap<String, RawSignerConfig>,
    #[serde(default)]
    servers: BTreeMap<String, RawServerConfig>,
    #[serde(default)]
    profiles: BTreeMap<String, RawProfileConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBrokerConfig {
    cert_ttl_seconds: u64,
    audit_log_path: String,
    default_signer: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSignerConfig {
    kind: String,
    ca_url: Option<String>,
    provisioner: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawServerConfig {
    host: String,
    port: Option<u16>,
    user: String,
    environment: String,
    allowed_profiles: Vec<String>,
    requires_approval: Option<bool>,
    signer: Option<String>,
    root_login_acknowledged: Option<bool>,
    /// Optional explicit auth selector.
    auth_method: Option<String>,
    /// Retained only so we can reject raw password-env configs with a clear error.
    password_env_var: Option<String>,
    /// Retained only so we can reject inline passwords with a clear error.
    password: Option<String>,
    /// Name of the env var whose value is an opaque secret reference such as
    /// `os_keychain:agent-ssh:prod-web-1`.
    password_secret_ref_env_var: Option<String>,
    /// Explicit operator acknowledgment that this server is using the
    /// compatibility-only password lane.
    legacy_password_acknowledged: Option<bool>,
    /// Explicit operator acknowledgment that fail2ban allowlisting must be
    /// handled on the remote side for this server.
    fail2ban_allowlist_confirmed: Option<bool>,
    /// Whether unrestricted interactive sessions are allowed for this server.
    allow_unrestricted_sessions: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProfileConfig {
    description: Option<String>,
    template: String,
    requires_approval: Option<bool>,
}

fn validate_config(raw: RawConfig) -> Result<Config, ValidationError> {
    let mut issues = Vec::new();

    let default_signer = match SignerName::new(&raw.broker.default_signer) {
        Ok(name) => Some(name),
        Err(reason) => {
            issues.push(format!("broker.default_signer: {reason}"));
            None
        }
    };

    let audit_log_path = raw.broker.audit_log_path.trim();
    if audit_log_path.is_empty() {
        issues.push("broker.audit_log_path must not be empty".to_string());
    }

    if raw.broker.cert_ttl_seconds == 0 {
        issues.push("broker.cert_ttl_seconds must be greater than zero".to_string());
    }

    if raw.broker.cert_ttl_seconds > 3600 {
        issues
            .push("broker.cert_ttl_seconds must be less than or equal to 3600 seconds".to_string());
    }

    if raw.signers.is_empty() {
        issues.push("at least one signer must be configured under [signers]".to_string());
    }

    if raw.servers.is_empty() {
        issues.push("at least one server must be configured under [servers]".to_string());
    }

    if raw.profiles.is_empty() {
        issues.push("at least one profile must be configured under [profiles]".to_string());
    }

    let mut signers = BTreeMap::new();
    for (name, signer) in raw.signers {
        let signer_name = match SignerName::new(&name) {
            Ok(name) => name,
            Err(reason) => {
                issues.push(format!("signers.{name}: {reason}"));
                continue;
            }
        };

        let kind = signer.kind.trim();
        if !is_non_empty_without_controls(kind) {
            issues.push(format!(
                "signers.{name}.kind must not be empty or contain control characters"
            ));
            continue;
        }

        signers.insert(
            signer_name,
            SignerConfig {
                kind: kind.to_string(),
                ca_url: normalize_optional_string(signer.ca_url),
                provisioner: normalize_optional_string(signer.provisioner),
                subject: normalize_optional_string(signer.subject),
            },
        );
    }

    if let Some(default_signer) = &default_signer
        && !signers.contains_key(default_signer.as_str())
    {
        issues.push(format!(
            "broker.default_signer references unknown signer '{}'",
            default_signer
        ));
    }

    let mut profiles = BTreeMap::new();
    for (name, profile) in raw.profiles {
        let profile_name = match ProfileName::new(&name) {
            Ok(name) => name,
            Err(reason) => {
                issues.push(format!("profiles.{name}: {reason}"));
                continue;
            }
        };

        let template = profile.template.trim();
        if template.is_empty() {
            issues.push(format!("profiles.{name}.template must not be empty"));
            continue;
        }

        if template.len() > MAX_TEMPLATE_LEN {
            issues.push(format!(
                "profiles.{name}.template must not exceed {MAX_TEMPLATE_LEN} characters"
            ));
            continue;
        }

        profiles.insert(
            profile_name,
            ProfileConfig {
                description: normalize_optional_string(profile.description),
                template: template.to_string(),
                requires_approval: profile.requires_approval.unwrap_or(false),
            },
        );
    }

    let mut servers = BTreeMap::new();
    for (server_name, server) in raw.servers {
        let alias = match ServerAlias::new(&server_name) {
            Ok(alias) => alias,
            Err(reason) => {
                issues.push(format!("servers.{server_name}: {reason}"));
                continue;
            }
        };

        let host = server.host.trim();
        if !is_non_empty_without_whitespace(host) {
            issues.push(format!(
                "servers.{server_name}.host must not be empty or contain whitespace"
            ));
        } else if host.len() > MAX_HOST_LEN {
            issues.push(format!(
                "servers.{server_name}.host must not exceed {MAX_HOST_LEN} characters"
            ));
        }

        let user = server.user.trim();
        if !is_valid_remote_user(user) {
            issues.push(format!(
                "servers.{server_name}.user must use a conservative SSH username character set"
            ));
        } else if user.len() > MAX_USER_LEN {
            issues.push(format!(
                "servers.{server_name}.user must not exceed {MAX_USER_LEN} characters"
            ));
        }

        if user == "root" && !server.root_login_acknowledged.unwrap_or(false) {
            issues.push(format!(
                "servers.{server_name}.user = \"root\" is discouraged and requires root_login_acknowledged = true"
            ));
        }

        if user != "root" && server.root_login_acknowledged.unwrap_or(false) {
            issues.push(format!(
                "servers.{server_name}.root_login_acknowledged is only allowed when user = \"root\""
            ));
        }

        let environment = server.environment.trim();
        if !is_non_empty_without_controls(environment) {
            issues.push(format!(
                "servers.{server_name}.environment must not be empty or contain control characters"
            ));
        } else if environment.len() > MAX_ENVIRONMENT_LEN {
            issues.push(format!(
                "servers.{server_name}.environment must not exceed {MAX_ENVIRONMENT_LEN} characters"
            ));
        }

        let port = server.port.unwrap_or(22);
        if port == 0 {
            issues.push(format!(
                "servers.{server_name}.port must be greater than zero"
            ));
        }

        if server.allowed_profiles.is_empty() {
            issues.push(format!(
                "servers.{server_name}.allowed_profiles must not be empty"
            ));
        }

        let mut allowed_profiles = Vec::new();
        for profile_name in server.allowed_profiles {
            let parsed = match ProfileName::new(&profile_name) {
                Ok(name) => name,
                Err(reason) => {
                    issues.push(format!(
                        "servers.{server_name}.allowed_profiles contains invalid profile '{profile_name}': {reason}"
                    ));
                    continue;
                }
            };

            if !profiles.contains_key(parsed.as_str()) {
                issues.push(format!(
                    "servers.{server_name}.allowed_profiles references unknown profile '{profile_name}'"
                ));
                continue;
            }

            allowed_profiles.push(parsed);
        }

        let signer = match server.signer {
            Some(signer_name) => match SignerName::new(&signer_name) {
                Ok(parsed_signer_name) => {
                    if !signers.contains_key(parsed_signer_name.as_str()) {
                        issues.push(format!(
                            "servers.{server_name}.signer references unknown signer '{signer_name}'"
                        ));
                    }
                    Some(parsed_signer_name)
                }
                Err(reason) => {
                    issues.push(format!("servers.{server_name}.signer: {reason}"));
                    None
                }
            },
            None => None,
        };

        if signer.is_none() && default_signer.is_none() {
            issues.push(format!(
                "servers.{server_name} has no valid signer because broker.default_signer is invalid"
            ));
        }

        if server.password_env_var.is_some() {
            issues.push(format!(
                "servers.{server_name}.password_env_var is not supported; use password_secret_ref_env_var only with auth_method = \"legacy_password\" and an opaque secret reference"
            ));
        }

        if server.password.is_some() {
            issues.push(format!(
                "servers.{server_name}.password is not supported; plaintext passwords are not allowed"
            ));
        }

        let auth_method = match server.auth_method.as_deref().map(str::trim) {
            None | Some("certificate") => AuthMethod::Certificate,
            Some("legacy_password") => AuthMethod::LegacyPassword,
            Some("") => {
                issues.push(format!(
                    "servers.{server_name}.auth_method must not be empty when provided"
                ));
                AuthMethod::Certificate
            }
            Some(other) => {
                issues.push(format!(
                    "servers.{server_name}.auth_method must be 'certificate' or 'legacy_password', got '{other}'"
                ));
                AuthMethod::Certificate
            }
        };

        let password_secret_ref_env_var =
            server.password_secret_ref_env_var.as_deref().map(str::trim);
        let legacy_password = match auth_method {
            AuthMethod::Certificate => {
                if password_secret_ref_env_var.is_some() {
                    issues.push(format!(
                        "servers.{server_name}.password_secret_ref_env_var is only allowed when auth_method = \"legacy_password\""
                    ));
                }
                if server.legacy_password_acknowledged.unwrap_or(false) {
                    issues.push(format!(
                        "servers.{server_name}.legacy_password_acknowledged is only allowed when auth_method = \"legacy_password\""
                    ));
                }
                if server.fail2ban_allowlist_confirmed.unwrap_or(false) {
                    issues.push(format!(
                        "servers.{server_name}.fail2ban_allowlist_confirmed is only allowed when auth_method = \"legacy_password\""
                    ));
                }
                None
            }
            AuthMethod::LegacyPassword => {
                let Some(secret_ref_env_var) = password_secret_ref_env_var else {
                    issues.push(format!(
                        "servers.{server_name}.password_secret_ref_env_var is required when auth_method = \"legacy_password\""
                    ));
                    servers.insert(
                        alias,
                        ServerConfig {
                            host: host.to_string(),
                            port,
                            user: user.to_string(),
                            environment: environment.to_string(),
                            allowed_profiles,
                            requires_approval: server.requires_approval.unwrap_or(false),
                            signer,
                            auth_method,
                            legacy_password: None,
                            allow_unrestricted_sessions: server
                                .allow_unrestricted_sessions
                                .unwrap_or(false),
                        },
                    );
                    continue;
                };

                if !is_valid_env_var_name(secret_ref_env_var) {
                    issues.push(format!(
                        "servers.{server_name}.password_secret_ref_env_var must be a valid environment variable name"
                    ));
                }
                if !server.legacy_password_acknowledged.unwrap_or(false) {
                    issues.push(format!(
                        "servers.{server_name}.legacy_password_acknowledged must be true when auth_method = \"legacy_password\""
                    ));
                }
                if !server.fail2ban_allowlist_confirmed.unwrap_or(false) {
                    issues.push(format!(
                        "servers.{server_name}.fail2ban_allowlist_confirmed must be true when auth_method = \"legacy_password\""
                    ));
                }

                Some(LegacyPasswordConfig {
                    secret_ref_env_var: secret_ref_env_var.to_string(),
                })
            }
        };

        servers.insert(
            alias,
            ServerConfig {
                host: host.to_string(),
                port,
                user: user.to_string(),
                environment: environment.to_string(),
                allowed_profiles,
                requires_approval: server.requires_approval.unwrap_or(false),
                signer,
                auth_method,
                legacy_password,
                allow_unrestricted_sessions: server.allow_unrestricted_sessions.unwrap_or(false),
            },
        );
    }

    if !issues.is_empty() {
        return Err(ValidationError::new(issues));
    }

    let Some(default_signer) = default_signer else {
        return Err(ValidationError::new(vec![
            "broker.default_signer must be valid".to_string(),
        ]));
    };

    Ok(Config {
        broker: BrokerConfig {
            cert_ttl_seconds: raw.broker.cert_ttl_seconds,
            audit_log_path: PathBuf::from(audit_log_path),
            default_signer,
        },
        signers,
        servers,
        profiles,
    })
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn is_non_empty_without_controls(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && !trimmed.chars().any(char::is_control)
}

fn is_non_empty_without_whitespace(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && !trimmed.chars().any(char::is_whitespace)
}

fn is_valid_remote_user(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    if !matches!(first, 'a'..='z' | 'A'..='Z' | '_') {
        return false;
    }

    chars.all(|char| matches!(char, 'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-'))
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
    use super::{AuthMethod, MAX_HOST_LEN, MAX_TEMPLATE_LEN, MAX_USER_LEN, parse_config};

    const VALID_CONFIG: &str = r#"
[broker]
cert_ttl_seconds = 120
audit_log_path = "./data/audit.jsonl"
default_signer = "step_ca"

[signers.step_ca]
kind = "step-ca"

[servers.staging-api]
host = "10.0.1.10"
port = 22
user = "deploy"
environment = "staging"
allowed_profiles = ["logs", "disk"]

[profiles.logs]
template = "journalctl -u {{service}} --since {{since}} --no-pager"

[profiles.disk]
template = "df -h"
"#;

    // ── Basic parsing ────────────────────────────────────────────────────────

    #[test]
    fn parses_valid_configuration() {
        let config = match parse_config(VALID_CONFIG) {
            Ok(config) => config,
            Err(error) => panic!("valid config should parse: {error}"),
        };

        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.profiles.len(), 2);
        assert_eq!(config.broker.default_signer.as_str(), "step_ca");
        assert_eq!(
            config
                .servers
                .get("staging-api")
                .expect("server exists")
                .allowed_profiles
                .len(),
            2
        );
    }

    #[test]
    fn rejects_root_user() {
        let source = VALID_CONFIG.replace("user = \"deploy\"", "user = \"root\"");
        let error = match parse_config(&source) {
            Ok(_) => panic!("root user without acknowledgment must be rejected"),
            Err(error) => error,
        };
        let message = error.to_string();

        assert!(message.contains("root_login_acknowledged"));
    }

    #[test]
    fn accepts_root_user_when_acknowledged() {
        let source = VALID_CONFIG.replace(
            "user = \"deploy\"",
            "user = \"root\"\nroot_login_acknowledged = true",
        );
        let config = parse_config(&source).expect("acknowledged root should parse");
        let server = config.servers.get("staging-api").expect("server exists");
        assert_eq!(server.user, "root");
    }

    #[test]
    fn rejects_root_acknowledgement_for_non_root_user() {
        let source = VALID_CONFIG.replace(
            "user = \"deploy\"",
            "user = \"deploy\"\nroot_login_acknowledged = true",
        );
        let error = parse_config(&source).expect_err("non-root ack flag must be rejected");
        assert!(
            error
                .to_string()
                .contains("root_login_acknowledged is only allowed")
        );
    }

    #[test]
    fn rejects_unknown_profile_reference() {
        let source = VALID_CONFIG.replace("[\"logs\", \"disk\"]", "[\"logs\", \"missing\"]");
        let error = match parse_config(&source) {
            Ok(_) => panic!("unknown profile should be rejected"),
            Err(error) => error,
        };
        let message = error.to_string();

        assert!(message.contains("references unknown profile 'missing'"));
    }

    #[test]
    fn trims_host_user_and_environment_values() {
        let source = VALID_CONFIG
            .replace("host = \"10.0.1.10\"", "host = \" 10.0.1.10 \"")
            .replace("user = \"deploy\"", "user = \" deploy \"")
            .replace("environment = \"staging\"", "environment = \" staging \"");
        let config = match parse_config(&source) {
            Ok(config) => config,
            Err(error) => panic!("trimmed values should still parse: {error}"),
        };
        let server = match config.servers.get("staging-api") {
            Some(server) => server,
            None => panic!("staging-api should exist"),
        };

        assert_eq!(server.host, "10.0.1.10");
        assert_eq!(server.user, "deploy");
        assert_eq!(server.environment, "staging");
    }

    // ── AuthMethod: certificate (default) ────────────────────────────────────

    #[test]
    fn defaults_to_certificate_auth() {
        let config = parse_config(VALID_CONFIG).expect("valid config");
        let server = config.servers.get("staging-api").expect("server exists");
        assert_eq!(server.auth_method, AuthMethod::Certificate);
    }

    #[test]
    fn accepts_explicit_certificate_auth_method() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"certificate\"",
        );
        let config = parse_config(&source).expect("explicit certificate should parse");
        let server = config.servers.get("staging-api").expect("server exists");
        assert_eq!(server.auth_method, AuthMethod::Certificate);
    }

    // ── Legacy password auth ────────────────────────────────────────────────

    #[test]
    fn accepts_legacy_password_auth_with_secret_reference() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"legacy_password\"\npassword_secret_ref_env_var = \"AGENT_SSH_STAGING_PASSWORD_REF\"\nlegacy_password_acknowledged = true\nfail2ban_allowlist_confirmed = true",
        );
        let config = parse_config(&source).expect("legacy password auth should parse");
        let server = config.servers.get("staging-api").expect("server exists");
        assert_eq!(server.auth_method, AuthMethod::LegacyPassword);
        assert_eq!(
            server
                .legacy_password
                .as_ref()
                .expect("legacy password config")
                .secret_ref_env_var,
            "AGENT_SSH_STAGING_PASSWORD_REF"
        );
    }

    #[test]
    fn rejects_password_env_var() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\npassword_env_var = \"LEGACY_SSH_PASS\"",
        );
        let error = parse_config(&source).expect_err("password_env_var must be rejected");
        assert!(error.to_string().contains("password_secret_ref_env_var"));
    }

    #[test]
    fn rejects_missing_legacy_password_reference() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"legacy_password\"\nlegacy_password_acknowledged = true\nfail2ban_allowlist_confirmed = true",
        );
        let error = parse_config(&source).expect_err("missing secret ref env var must be rejected");
        assert!(
            error
                .to_string()
                .contains("password_secret_ref_env_var is required")
        );
    }

    #[test]
    fn rejects_legacy_password_without_acknowledgement() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"legacy_password\"\npassword_secret_ref_env_var = \"AGENT_SSH_STAGING_PASSWORD_REF\"\nfail2ban_allowlist_confirmed = true",
        );
        let error = parse_config(&source).expect_err("legacy password ack must be required");
        assert!(
            error
                .to_string()
                .contains("legacy_password_acknowledged must be true")
        );
    }

    #[test]
    fn rejects_legacy_password_without_fail2ban_confirmation() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"legacy_password\"\npassword_secret_ref_env_var = \"AGENT_SSH_STAGING_PASSWORD_REF\"\nlegacy_password_acknowledged = true",
        );
        let error = parse_config(&source).expect_err("fail2ban confirmation must be required");
        assert!(
            error
                .to_string()
                .contains("fail2ban_allowlist_confirmed must be true")
        );
    }

    #[test]
    fn rejects_invalid_secret_ref_env_var_name() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"legacy_password\"\npassword_secret_ref_env_var = \"BAD-NAME\"\nlegacy_password_acknowledged = true\nfail2ban_allowlist_confirmed = true",
        );
        let error = parse_config(&source).expect_err("invalid env var name must be rejected");
        assert!(
            error
                .to_string()
                .contains("must be a valid environment variable name")
        );
    }

    #[test]
    fn rejects_inline_password_field() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\npassword = \"supersecret\"",
        );
        let error = parse_config(&source).expect_err("inline password must be rejected");
        assert!(
            error
                .to_string()
                .contains("plaintext passwords are not allowed")
        );
    }

    #[test]
    fn rejects_unknown_auth_method() {
        let source = VALID_CONFIG.replace(
            "allowed_profiles = [\"logs\", \"disk\"]",
            "allowed_profiles = [\"logs\", \"disk\"]\nauth_method = \"ssh-key\"",
        );
        let error = parse_config(&source).expect_err("unknown auth_method must be rejected");
        assert!(
            error
                .to_string()
                .contains("auth_method must be 'certificate' or 'legacy_password'")
        );
    }

    // ── Length limits ────────────────────────────────────────────────────────

    #[test]
    fn rejects_host_exceeding_max_length() {
        let long_host = "a".repeat(MAX_HOST_LEN + 1);
        let source =
            VALID_CONFIG.replace("host = \"10.0.1.10\"", &format!("host = \"{long_host}\""));
        let error = parse_config(&source).expect_err("overlong host must be rejected");
        assert!(error.to_string().contains("host must not exceed"));
    }

    #[test]
    fn rejects_user_exceeding_max_length() {
        let long_user = "a".repeat(MAX_USER_LEN + 1);
        let source = VALID_CONFIG.replace("user = \"deploy\"", &format!("user = \"{long_user}\""));
        let error = parse_config(&source).expect_err("overlong user must be rejected");
        assert!(error.to_string().contains("user must not exceed"));
    }

    #[test]
    fn rejects_template_exceeding_max_length() {
        let long_template = format!("echo {}", "a".repeat(MAX_TEMPLATE_LEN));
        let source = VALID_CONFIG.replace(
            "template = \"df -h\"",
            &format!("template = \"{long_template}\""),
        );
        let error = parse_config(&source).expect_err("overlong template must be rejected");
        assert!(error.to_string().contains("template must not exceed"));
    }

    // ── Structural rejections ────────────────────────────────────────────────

    #[test]
    fn rejects_zero_cert_ttl() {
        let source = VALID_CONFIG.replace("cert_ttl_seconds = 120", "cert_ttl_seconds = 0");
        let error = parse_config(&source).expect_err("zero TTL must be rejected");
        assert!(error.to_string().contains("must be greater than zero"));
    }

    #[test]
    fn rejects_cert_ttl_exceeding_one_hour() {
        let source = VALID_CONFIG.replace("cert_ttl_seconds = 120", "cert_ttl_seconds = 3601");
        let error = parse_config(&source).expect_err("TTL > 3600 must be rejected");
        assert!(error.to_string().contains("less than or equal to 3600"));
    }

    #[test]
    fn rejects_empty_server_list() {
        // Remove the server section entirely.
        let source = VALID_CONFIG
            .lines()
            .filter(|line| {
                !line.starts_with("[servers")
                    && !line.contains("host =")
                    && !line.contains("port =")
                    && !line.contains("user =")
                    && !line.contains("environment =")
                    && !line.contains("allowed_profiles =")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let error = parse_config(&source).expect_err("no servers must be rejected");
        assert!(error.to_string().contains("at least one server"));
    }

    #[test]
    fn rejects_zero_port() {
        let source = VALID_CONFIG.replace("port = 22", "port = 0");
        let error = parse_config(&source).expect_err("port 0 must be rejected");
        assert!(error.to_string().contains("port must be greater than zero"));
    }

    #[test]
    fn rejects_user_with_invalid_chars() {
        // Leading digit is not allowed for SSH usernames.
        let source = VALID_CONFIG.replace("user = \"deploy\"", "user = \"1deploy\"");
        let error = parse_config(&source).expect_err("bad username must be rejected");
        assert!(error.to_string().contains("conservative SSH username"));
    }

    #[test]
    fn collects_multiple_validation_issues() {
        let source = VALID_CONFIG
            .replace("user = \"deploy\"", "user = \"root\"")
            .replace("port = 22", "port = 0");
        let error = parse_config(&source).expect_err("multiple issues must be rejected");
        let message = error.to_string();
        // Both issues should appear in the combined error.
        assert!(message.contains("root_login_acknowledged"), "{message}");
        assert!(
            message.contains("port must be greater than zero"),
            "{message}"
        );
    }
}
