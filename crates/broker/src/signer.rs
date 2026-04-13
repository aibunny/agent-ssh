use std::path::PathBuf;

use agent_ssh_common::SignerName;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningRequest {
    pub signer: SignerName,
    pub server_alias: String,
    pub remote_user: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedSessionMaterial {
    pub private_key_path: PathBuf,
    pub certificate_path: PathBuf,
    pub expires_at: String,
}

pub trait Signer {
    fn signer_name(&self) -> &SignerName;
    fn issue(&self, request: &SigningRequest) -> Result<SignedSessionMaterial, SignerFailure>;
}

#[derive(Debug, Error)]
pub enum SignerFailure {
    #[error("signer '{name}' is not implemented in this milestone")]
    NotImplemented { name: String },
    #[error("signer '{name}' failed: {reason}")]
    IssueFailed { name: String, reason: String },
}
