use anyhow::{Context, Result};
use server::{audit_log::AuditLog, config, server::Server};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().init();

    // Load configuration
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".to_string());
    let config = config::load_config(&config_path)
        .with_context(|| format!("failed to load config from {}", config_path))?;

    // Create audit log (encryption key optional)
    let encryption_key = std::env::var(&config.encryption_key_env_var)
        .ok()
        .and_then(|key| hex::decode(key).ok())
        .and_then(|bytes| {
            if bytes.len() == 32 {
                Some(bytes.try_into().unwrap())
            } else {
                tracing::warn!("encryption key must be 32 bytes, got {} bytes", bytes.len());
                None
            }
        });
    let audit_log = AuditLog::new(&config.audit_log_path, encryption_key)
        .with_context(|| format!("failed to open audit log at {:?}", config.audit_log_path))?;

    // Create and run server
    let server = Server::new(config, audit_log);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;
    runtime.block_on(async {
        let listener = server.bind().await.context("failed to bind server")?;
        server.run(listener).await.context("server error")
    })
}
