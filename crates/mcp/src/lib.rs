//! Future agent-facing interface layer for `agent-ssh`.
//!
//! The first milestone keeps this crate intentionally small so later MCP work can
//! reuse the broker core without reshaping the repository.

pub use agent_ssh_broker as broker;
