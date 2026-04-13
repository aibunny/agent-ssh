use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use agent_ssh_common::AuditEvent;

use crate::BrokerError;

#[derive(Debug, Clone)]
pub struct AuditLogger {
    path: PathBuf,
}

impl AuditLogger {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn append(&self, event: &AuditEvent) -> Result<(), BrokerError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|source| BrokerError::AuditIo {
                path: self.path.clone(),
                source,
            })?;
        }

        let serialized = serde_json::to_string(event)
            .map_err(|source| BrokerError::AuditSerialize { source })?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|source| BrokerError::AuditIo {
                path: self.path.clone(),
                source,
            })?;

        writeln!(file, "{serialized}").map_err(|source| BrokerError::AuditIo {
            path: self.path.clone(),
            source,
        })?;

        Ok(())
    }
}
