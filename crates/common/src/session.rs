use serde::{Deserialize, Serialize};

/// How a session may execute commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    /// Only commands that match the server's allowed profiles may execute.
    Restricted,
    /// Any command may execute. Requires explicit server opt-in and an approval reference.
    Unrestricted,
}

impl std::fmt::Display for SessionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionMode::Restricted => write!(f, "restricted"),
            SessionMode::Unrestricted => write!(f, "unrestricted"),
        }
    }
}

/// A persisted record for an open broker session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRecord {
    /// Unique session identifier (UUID v4 as string).
    pub id: String,
    /// Server alias this session is connected to.
    pub server_alias: String,
    /// Remote host.
    pub host: String,
    /// Remote port.
    pub port: u16,
    /// Remote user.
    pub user: String,
    /// Environment label from server config.
    pub environment: String,
    /// Auth method kind: "certificate" or "legacy_password".
    pub auth_method_kind: String,
    /// Execution mode.
    pub mode: SessionMode,
    /// Unix timestamp (seconds) when the session was opened.
    pub opened_at_unix: i64,
    /// Unix timestamp (seconds) of the last command execution (or open time initially).
    pub last_used_at_unix: i64,
    /// Session TTL in seconds from open time.
    pub ttl_seconds: u64,
    /// Idle timeout in seconds from last command.
    pub idle_timeout_seconds: u64,
    /// Approval reference recorded at session open time.
    pub approval_reference: Option<String>,
    /// Filesystem path to the SSH ControlMaster socket.
    pub control_socket_path: String,
}

impl SessionRecord {
    /// Returns true if the session's absolute TTL has elapsed.
    pub fn is_expired(&self, now_unix: i64) -> bool {
        let age = now_unix.saturating_sub(self.opened_at_unix);
        age > self.ttl_seconds as i64
    }

    /// Returns true if the session's idle timeout has elapsed.
    pub fn is_idle_timed_out(&self, now_unix: i64) -> bool {
        let idle = now_unix.saturating_sub(self.last_used_at_unix);
        idle > self.idle_timeout_seconds as i64
    }
}
