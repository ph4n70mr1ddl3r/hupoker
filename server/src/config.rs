use anyhow::{Context, Result};
use game_engine::ServerConfig;
use std::fs;
use std::path::Path;

pub fn load_config(path: impl AsRef<Path>) -> Result<ServerConfig> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("failed to read config file {:?}", path.as_ref()))?;
    let config: ServerConfig = toml::from_str(&content)
        .with_context(|| format!("failed to parse config file {:?}", path.as_ref()))?;
    config.validate()
        .with_context(|| "config validation failed")?;
    Ok(config)
}