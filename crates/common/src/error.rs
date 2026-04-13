use std::{fmt, io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read configuration from {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to parse TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("{0}")]
    Validation(#[from] ValidationError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    issues: Vec<String>,
}

impl ValidationError {
    pub fn new(mut issues: Vec<String>) -> Self {
        issues.sort();
        issues.dedup();
        Self { issues }
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "configuration validation failed:")?;

        for issue in &self.issues {
            writeln!(formatter, "- {issue}")?;
        }

        Ok(())
    }
}

impl std::error::Error for ValidationError {}
