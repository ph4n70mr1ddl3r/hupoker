use anyhow::Result;
use server::{audit_log::AuditLog, config, server::Server};
use tracing_subscriber;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".to_string());
    let config = config::load_config(&config_path)?;

    // Create audit log (encryption key optional)
    let encryption_key = std::env::var(&config.encryption_key_env_var)
        .ok()
        .and_then(|key| hex::decode(key).ok())
        .and_then(|bytes| if bytes.len() == 32 { Some(bytes.try_into().unwrap()) } else { None });
    let audit_log = AuditLog::new(&config.audit_log_path, encryption_key)?;

    // Create and run server
    let server = Server::new(config, audit_log);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime")
        .block_on(async {
            let listener = server.bind().await?;
            server.run(listener).await
        })
}
