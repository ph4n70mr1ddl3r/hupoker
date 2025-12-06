use anyhow::Result;
use game_engine::{ServerConfig, TableConfig};
use server::{audit_log::AuditLog, server::Server};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

#[tokio::test]
async fn client_hello_server_hello() -> Result<()> {
    // Create a temporary listener to get a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener); // free the port for the server

    // Build config with that address
    let config = ServerConfig {
        bind_address: addr.to_string(),
        tables: vec![TableConfig {
            small_blind: 10,
            big_blind: 20,
            starting_stack: 1500,
            action_timeout_secs: 30,
            reconnection_timeout_secs: 60,
        }],
        audit_log_path: PathBuf::from("test-audit.log"),
        encryption_key_env_var: "HUPOKER_ENCRYPTION_KEY".to_string(),
    };
    // Create audit log (no encryption)
    let audit_log = AuditLog::new(&config.audit_log_path, None)?;
    let server = Server::new(config, audit_log);
    let server_listener = server.bind().await?;
    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let _ = server.run(server_listener).await;
    });

    // Give server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let stream = TcpStream::connect(addr).await?;
    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    // Send ClientHello
    let client_hello = serde_json::json!({
        "type": "client_hello",
        "version": "1.0",
        "client_name": "test",
        "client_version": "0.1.0"
    });
    let line = serde_json::to_string(&client_hello)? + "\n";
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    // Read ServerHello response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let server_hello: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(server_hello["type"], "server_hello");
    assert_eq!(server_hello["status"], "accepted");

    // Clean up: kill server task
    server_handle.abort();
    // Ignore error from abort
    Ok(())
}

#[tokio::test]
async fn join_table_success() -> Result<()> {
    // Create a temporary listener to get a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);

    // Build config with that address
    let config = ServerConfig {
        bind_address: addr.to_string(),
        tables: vec![TableConfig {
            small_blind: 10,
            big_blind: 20,
            starting_stack: 1500,
            action_timeout_secs: 30,
            reconnection_timeout_secs: 60,
        }],
        audit_log_path: PathBuf::from("test-audit2.log"),
        encryption_key_env_var: "HUPOKER_ENCRYPTION_KEY".to_string(),
    };
    let audit_log = AuditLog::new(&config.audit_log_path, None)?;
    let server = Server::new(config, audit_log);
    let server_listener = server.bind().await?;
    let server_handle = tokio::spawn(async move {
        let _ = server.run(server_listener).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let stream = TcpStream::connect(addr).await?;
    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    // Send ClientHello
    let client_hello = serde_json::json!({
        "type": "client_hello",
        "version": "1.0",
        "client_name": "test",
        "client_version": "0.1.0"
    });
    let line = serde_json::to_string(&client_hello)? + "\n";
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    // Read ServerHello (ignore)
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    // Send JoinTable request for seat 0 at table-0
    let join_table = serde_json::json!({
        "type": "join_table",
        "version": "1.0",
        "table_id": "table-0",
        "seat": 0
    });
    let line = serde_json::to_string(&join_table)? + "\n";
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    // Read TableState response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let table_state: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(table_state["type"], "table_state");
    assert_eq!(table_state["table_id"], "table-0");
    assert_eq!(table_state["seats"].as_array().unwrap().len(), 2);
    // Seat 0 should have a player
    let seats = table_state["seats"].as_array().unwrap();
    assert!(seats[0]["player"].is_object());
    assert!(seats[1]["player"].is_null());

    // Clean up
    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn reconnection_flow() -> Result<()> {
    // TODO: implement reconnection test
    Ok(())
}
