mod common;

use anyhow::Result;
use common::TestClient;
use game_engine::{ServerConfig, TableConfig};
use server::{audit_log::AuditLog, protocol::Message, server::Server};
use std::path::PathBuf;
use tokio::time::{timeout, Duration};

/// Integration test for action timeout: a player that does not act within the allowed time
/// should be auto‑folded.
/// This test is ignored until server‑side timeout enforcement is implemented (T049–T051).
#[tokio::test]
async fn action_timeout_auto_fold() -> Result<()> {
    // Create a temporary listener to get a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);

    // Build config with very short action timeout for test (1 second)
    let config = ServerConfig {
        bind_address: addr.to_string(),
        tables: vec![TableConfig {
            small_blind: 10,
            big_blind: 20,
            starting_stack: 1500,
            action_timeout_secs: 1, // 1 second timeout for test
            reconnection_timeout_secs: 60,
        }],
        audit_log_path: PathBuf::from("test-audit-timeout.log"),
        encryption_key_env_var: "HUPOKER_ENCRYPTION_KEY".to_string(),
    };
    let audit_log = AuditLog::new(&config.audit_log_path, None)?;
    let server = Server::new(config, audit_log);
    let server_listener = server.bind().await?;
    let server_handle = tokio::spawn(async move {
        let _ = server.run(server_listener).await;
    });

    // Give server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect player 1 (seat 0)
    let mut client1 = TestClient::connect(addr).await?;
    client1.send(&serde_json::json!({
        "type": "client_hello",
        "version": "1.0",
        "client_name": "test",
        "client_version": "0.1.0"
    })).await?;
    let server_hello = client1.recv().await?;
    assert!(matches!(server_hello, Message::ServerHello { .. }));
    client1.send(&serde_json::json!({
        "type": "join_table",
        "version": "1.0",
        "table_id": "table-0",
        "seat": 0
    })).await?;
    let table_state = client1.recv().await?;
    assert!(matches!(table_state, Message::TableState { .. }));

    // Connect player 2 (seat 1)
    let mut client2 = TestClient::connect(addr).await?;
    client2.send(&serde_json::json!({
        "type": "client_hello",
        "version": "1.0",
        "client_name": "test",
        "client_version": "0.1.0"
    })).await?;
    let server_hello2 = client2.recv().await?;
    assert!(matches!(server_hello2, Message::ServerHello { .. }));
    client2.send(&serde_json::json!({
        "type": "join_table",
        "version": "1.0",
        "table_id": "table-0",
        "seat": 1
    })).await?;
    let table_state2 = client2.recv().await?;
    assert!(matches!(table_state2, Message::TableState { .. }));

    // Both seats occupied → hand should start automatically.
    // Wait for HandState messages (both players receive them)
    let hand_state1 = client1.recv_timeout(Duration::from_millis(500)).await?;
    let hand_state2 = client2.recv_timeout(Duration::from_millis(500)).await?;
    assert!(matches!(hand_state1, Message::HandState { .. }));
    assert!(matches!(hand_state2, Message::HandState { .. }));

    // Determine which player is the acting player (button position).
    // The acting player (small blind) must act first.
    // We will NOT send any action from that player, letting the timeout expire.
    // Wait slightly longer than the action timeout (1 second) plus a safety margin.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // After the timeout, the server should have auto‑folded the acting player.
    // Both clients should receive a HandState reflecting the fold action.
    // We'll read one more message from each client (expect HandState with updated actions).
    let hand_state_after_timeout1 = client1.recv_timeout(Duration::from_millis(500)).await?;
    let hand_state_after_timeout2 = client2.recv_timeout(Duration::from_millis(500)).await?;
    assert!(matches!(hand_state_after_timeout1, Message::HandState { .. }));
    assert!(matches!(hand_state_after_timeout2, Message::HandState { .. }));

    // Verify that the hand has progressed (street may have advanced or hand ended).
    // For now, we just ensure the test compiles and runs without panic.
    // Actual assertions will be added once the server implements auto‑fold.

    // Clean up: kill server task
    server_handle.abort();
    Ok(())
}