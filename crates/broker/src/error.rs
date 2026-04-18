use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrokerError {
    // ── Planning errors ──────────────────────────────────────────────────────
    #[error("server alias '{alias}' is not configured")]
    UnknownServer { alias: String },
    #[error("profile '{profile}' is not configured")]
    UnknownProfile { profile: String },
    #[error("profile '{profile}' is not allowed for server '{server}'")]
    ProfileNotAllowed { server: String, profile: String },
    #[error("approval is required for server '{server}' and profile '{profile}'")]
    ApprovalRequired { server: String, profile: String },
    #[error("missing required argument '{name}' for profile '{profile}'")]
    MissingArgument { profile: String, name: String },
    #[error("unexpected argument '{name}' for profile '{profile}'")]
    UnexpectedArgument { profile: String, name: String },
    #[error(
        "argument '{name}' for profile '{profile}' contains control characters and is rejected"
    )]
    InvalidArgumentValue { profile: String, name: String },
    #[error("profile '{profile}' has an unsafe template: {reason}")]
    UnsafeTemplate { profile: String, reason: String },

    // ── Audit errors ─────────────────────────────────────────────────────────
    #[error("failed to write audit log at {path}: {source}")]
    AuditIo { path: PathBuf, source: io::Error },
    #[error("failed to serialize audit event: {source}")]
    AuditSerialize { source: serde_json::Error },

    // ── Execution errors ─────────────────────────────────────────────────────
    #[error("legacy password auth is misconfigured for server '{server}'")]
    LegacyPasswordConfigMissing { server: String },
    #[error(
        "legacy password secret reference env var '{env_var}' is not set for server '{server}'"
    )]
    LegacyPasswordSecretRefMissing { server: String, env_var: String },
    #[error("legacy password secret reference is invalid for server '{server}': {reason}")]
    LegacyPasswordSecretRefInvalid { server: String, reason: String },
    #[error("legacy password auth is not supported on platform '{platform}'")]
    LegacyPasswordUnsupportedPlatform { platform: String },
    #[error("failed to prepare legacy password askpass helper: {source}")]
    LegacyPasswordAskpassIo { source: io::Error },

    /// The `ssh` binary is not in PATH.
    #[error(
        "the 'ssh' command was not found in PATH\n\
         Install OpenSSH:\n\
         \x20 Debian/Ubuntu:  sudo apt install openssh-client\n\
         \x20 macOS:           ssh ships with the OS — check your PATH"
    )]
    SshNotFound,

    /// An I/O error while spawning or waiting for the SSH process.
    #[error("failed to run ssh: {source}")]
    SshIo { source: io::Error },

    // ── Session errors ────────────────────────────────────────────────────────
    /// Session ID was not found in the session registry.
    #[error("session '{id}' was not found; it may have expired or been closed")]
    SessionNotFound { id: String },
    /// Session TTL has elapsed.
    #[error("session '{id}' has expired (TTL elapsed)")]
    SessionExpired { id: String },
    /// Session idle timeout has elapsed.
    #[error("session '{id}' has been closed due to idle timeout")]
    SessionIdleTimeout { id: String },
    /// Server does not allow unrestricted sessions.
    #[error(
        "server '{server}' does not allow unrestricted sessions; \
         set allow_unrestricted_sessions = true in the server config to enable"
    )]
    UnrestrictedSessionNotAllowed { server: String },
    /// Server requires `requires_approval = true` for unrestricted sessions.
    #[error("server '{server}' must have requires_approval = true to allow unrestricted sessions")]
    UnrestrictedSessionRequiresServerApprovalFlag { server: String },
    /// Unrestricted session was requested without an approval reference.
    #[error(
        "an approval reference is required to open an unrestricted session on server '{server}'"
    )]
    UnrestrictedSessionRequiresApproval { server: String },
    /// A command was denied by session policy.
    #[error("command denied for server '{server}': {reason}")]
    SessionCommandDenied { server: String, reason: String },
    /// Command exceeds the maximum allowed length.
    #[error("command length {length} exceeds maximum {max} characters")]
    SessionCommandTooLong { length: usize, max: usize },
    /// I/O error within a session operation.
    #[error("session I/O error: {source}")]
    SessionIo { source: io::Error },
    /// Failed to read or write the session registry.
    #[error("session registry error at {path}: {source}")]
    SessionRegistryIo { path: PathBuf, source: io::Error },
    /// Failed to parse a session registry record.
    #[error("session registry parse error: {source}")]
    SessionRegistryParse { source: serde_json::Error },
}
