use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::TableConfig;

/// Server‑wide configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String, // e.g., "127.0.0.1:8080"
    pub tables: Vec<TableConfig>,
    pub audit_log_path: PathBuf,
    pub encryption_key_env_var: String, // name of env var holding ChaCha20‑Poly1305 key
}

impl ServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.bind_address.is_empty() {
            return Err("bind_address must not be empty".to_string());
        }
        // Simple validation: must contain ':'
        if !self.bind_address.contains(':') {
            return Err("bind_address must be in format 'host:port'".to_string());
        }
        if self.audit_log_path.as_os_str().is_empty() {
            return Err("audit_log_path must not be empty".to_string());
        }
        if self.encryption_key_env_var.is_empty() {
            return Err("encryption_key_env_var must not be empty".to_string());
        }
        for (i, table) in self.tables.iter().enumerate() {
            table.validate().map_err(|e| format!("table config {i} invalid: {e}"))?;
        }
        Ok(())
    }
}
