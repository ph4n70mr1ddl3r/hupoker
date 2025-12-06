use anyhow::Result;
use game_engine::{ServerConfig, TableConfig};
use server::{audit_log::AuditLog, protocol::messages::Message, server::Server};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

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
    // Create a temporary listener to get a free port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);

    // Build config with short reconnection timeout for test
    let config = ServerConfig {
        bind_address: addr.to_string(),
        tables: vec![TableConfig {
            small_blind: 10,
            big_blind: 20,
            starting_stack: 1500,
            action_timeout_secs: 30,
            reconnection_timeout_secs: 2, // 2 seconds for test
        }],
        audit_log_path: PathBuf::from("test-audit-reconn.log"),
        encryption_key_env_var: "HUPOKER_ENCRYPTION_KEY".to_string(),
    };
    let audit_log = AuditLog::new(&config.audit_log_path, None)?;
    let server = Server::new(config, audit_log);
    let server_listener = server.bind().await?;
    let server_handle = tokio::spawn(async move {
        let _ = server.run(server_listener).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Helper to connect and join a seat, returns reader, writer, table_state JSON, and any hand_state messages received before table_state
    async fn connect_and_join(
        addr: std::net::SocketAddr,
        seat: u8,
    ) -> Result<(
        BufReader<tokio::net::tcp::OwnedReadHalf>,
        BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        serde_json::Value,
        Vec<serde_json::Value>,
    )> {
        let stream = TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
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

        // Send JoinTable
        let join_table = serde_json::json!({
            "type": "join_table",
            "version": "1.0",
            "table_id": "table-0",
            "seat": seat
        });
        let line = serde_json::to_string(&join_table)? + "\n";
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;

        // Read messages until we get a table_state response
        let mut table_state = None;
        let mut hand_states = Vec::new();
        while table_state.is_none() {
            let mut response = String::new();
            reader.read_line(&mut response).await?;
            eprintln!("DEBUG seat {}: received line: {}", seat, response.trim());
            let msg: serde_json::Value = serde_json::from_str(&response)?;
            match msg["type"].as_str() {
                Some("table_state") => {
                    assert_eq!(msg["table_id"], "table-0");
                    table_state = Some(msg);
                }
                Some("hand_state") => {
                    hand_states.push(msg);
                }
                _ => panic!("unexpected message type: {}", msg["type"]),
            }
        }

        Ok((reader, writer, table_state.unwrap(), hand_states))
    }

    // Connect player 0
    let (mut reader0, _writer0, _table_state0, _hand_states0) = connect_and_join(addr, 0).await?;
    // Connect player 1
    let (mut reader1, writer1, _table_state1, _hand_states1) = connect_and_join(addr, 1).await?;

    // Wait for hand start (server automatically starts hand after both seats join)
    // Read HandState for player 0 (should receive)
    let mut response = String::new();
    reader0.read_line(&mut response).await?;
    let hand_state0: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(hand_state0["type"], "hand_state");
    // Read HandState for player 1 (should receive)
    let mut response = String::new();
    reader1.read_line(&mut response).await?;
    let hand_state1: serde_json::Value = serde_json::from_str(&response)?;
    assert_eq!(hand_state1["type"], "hand_state");

    // Player 1 disconnects (drop reader and writer)
    drop(reader1);
    drop(writer1);

    // Wait a bit but within reconnection timeout
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Player 1 reconnects (new TCP connection) and joins same seat
    let (mut reader1_new, _writer1_new, table_state_reconn, hand_states_reconn) =
        connect_and_join(addr, 1).await?;
    // Seat 1 should have a player
    let seats = table_state_reconn["seats"].as_array().unwrap();
    assert!(seats[1]["player"].is_object());

    // Should also receive HandState (with hole cards) if hand still active
    let hand_state_reconn = if let Some(hand_state) = hand_states_reconn.into_iter().next() {
        hand_state
    } else {
        let mut response = String::new();
        reader1_new.read_line(&mut response).await?;
        serde_json::from_str(&response)?
    };
    assert_eq!(hand_state_reconn["type"], "hand_state");
    // Verify hole cards are present (should be array of 2 cards)
    let hole_cards = &hand_state_reconn["hole_cards"];
    assert!(hole_cards.is_array());
    assert_eq!(hole_cards.as_array().unwrap().len(), 2);

    // Clean up: kill server task
    server_handle.abort();
    Ok(())
}

/// A simple test client that holds a TCP connection and can send/receive messages.
#[allow(dead_code)]
struct TestClient {
    reader: tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::io::BufWriter<tokio::net::tcp::OwnedWriteHalf>,
}

#[allow(dead_code)]
impl TestClient {
    async fn connect(addr: std::net::SocketAddr) -> Result<Self> {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let reader = tokio::io::BufReader::new(read_half);
        let writer = tokio::io::BufWriter::new(write_half);
        Ok(Self { reader, writer })
    }

    /// Send a JSON message (newline-delimited)
    async fn send(&mut self, msg: &serde_json::Value) -> Result<()> {
        let line = serde_json::to_string(msg)? + "\n";
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Receive a JSON message, parsing into the generic Message enum.
    async fn recv(&mut self) -> Result<Message> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;
        let msg: Message = serde_json::from_str(&line)?;
        Ok(msg)
    }

    /// Receive with a timeout.
    async fn recv_timeout(&mut self, dur: Duration) -> Result<Message> {
        timeout(dur, self.recv()).await?
    }
}

/// Integration test for action timeout: a player that does not act within the allowed time
/// should be auto‑folded.
#[tokio::test]
async fn action_timeout_auto_fold() -> Result<()> {
    // Helper from reconnection_flow (copied)
    async fn connect_and_join(
        addr: std::net::SocketAddr,
        seat: u8,
    ) -> Result<(
        tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
        tokio::io::BufWriter<tokio::net::tcp::OwnedWriteHalf>,
        serde_json::Value,
        Vec<serde_json::Value>,
    )> {
        let stream = tokio::net::TcpStream::connect(addr).await?;
        let (read_half, write_half) = stream.into_split();
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);

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

        // Send JoinTable
        let join_table = serde_json::json!({
            "type": "join_table",
            "version": "1.0",
            "table_id": "table-0",
            "seat": seat
        });
        let line = serde_json::to_string(&join_table)? + "\n";
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;

        // Read messages until we get a table_state response
        let mut table_state = None;
        let mut hand_states = Vec::new();
        while table_state.is_none() {
            let mut response = String::new();
            reader.read_line(&mut response).await?;
            let msg: serde_json::Value = serde_json::from_str(&response)?;
            match msg["type"].as_str() {
                Some("table_state") => {
                    assert_eq!(msg["table_id"], "table-0");
                    table_state = Some(msg);
                }
                Some("hand_state") => {
                    hand_states.push(msg);
                }
                _ => panic!("unexpected message type: {}", msg["type"]),
            }
        }

        Ok((reader, writer, table_state.unwrap(), hand_states))
    }

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
        audit_log_path: std::path::PathBuf::from("test-audit-timeout.log"),
        encryption_key_env_var: "HUPOKER_ENCRYPTION_KEY".to_string(),
    };
    let audit_log = AuditLog::new(&config.audit_log_path, None)?;
    let server = Server::new(config, audit_log);
    let server_listener = server.bind().await?;
    let server_handle = tokio::spawn(async move {
        let _ = server.run(server_listener).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect player 0
    let (mut reader0, _writer0, _table_state0, hand_states0) = connect_and_join(addr, 0).await?;
    // Connect player 1
    let (mut reader1, _writer1, _table_state1, hand_states1) = connect_and_join(addr, 1).await?;

    // Determine if hand already started (hand_states may contain initial HandState)
    // If not, read one HandState for each player
    let hand_state0 = if let Some(hs) = hand_states0.into_iter().next() {
        hs
    } else {
        let mut line = String::new();
        reader0.read_line(&mut line).await?;
        serde_json::from_str(&line)?
    };
    let hand_state1 = if let Some(hs) = hand_states1.into_iter().next() {
        hs
    } else {
        let mut line = String::new();
        reader1.read_line(&mut line).await?;
        serde_json::from_str(&line)?
    };
    assert_eq!(hand_state0["type"], "hand_state");
    assert_eq!(hand_state1["type"], "hand_state");

    // Determine acting player (small blind) from hand_state0
    let acting_seat = hand_state0["acting_seat"].as_u64().unwrap() as u8;
    // Wait for timeout (1 second) plus margin
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // After timeout, both players should receive updated HandState with fold action
    let mut line0 = String::new();
    reader0.read_line(&mut line0).await?;
    let hand_state_after0: serde_json::Value = serde_json::from_str(&line0)?;
    let mut line1 = String::new();
    reader1.read_line(&mut line1).await?;
    let hand_state_after1: serde_json::Value = serde_json::from_str(&line1)?;
    assert_eq!(hand_state_after0["type"], "hand_state");
    assert_eq!(hand_state_after1["type"], "hand_state");

    // Verify that the acting player folded (check actions list)
    let actions = &hand_state_after0["actions"];
    assert!(actions
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["kind"] == "Fold" && a["seat"] == acting_seat));

    // Clean up: kill server task
    server_handle.abort();
    Ok(())
}
