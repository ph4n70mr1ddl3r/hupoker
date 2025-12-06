use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{
    audit_log::AuditLog, connection_manager::ConnectionManager,
    hand_state::create_hand_state_message, table_manager::TableManager,
};
use game_engine::ServerConfig;

#[derive(Clone)]
pub struct Server {
    config: ServerConfig,
    audit_log: Arc<Mutex<AuditLog>>,
    table_manager: Arc<Mutex<TableManager>>,
    connection_manager: ConnectionManager,
}

impl Server {
    pub fn new(config: ServerConfig, audit_log: AuditLog) -> Self {
        use game_engine::{Table, TableId};

        // Create tables from config
        let mut table_manager = TableManager::new();
        for (i, table_config) in config.tables.iter().enumerate() {
            let table = Table {
                id: TableId::new(format!("table-{}", i)),
                seats: [None, None],
                current_hand: None,
                next_button_position: 0,
                hand_count: 0,
                config: table_config.clone(),
                created_at: Utc::now(),
            };
            table_manager.add_table(table);
        }

        Self {
            config,
            audit_log: Arc::new(Mutex::new(audit_log)),
            table_manager: Arc::new(Mutex::new(table_manager)),
            connection_manager: ConnectionManager::new(),
        }
    }

    /// Attempt to start a hand at the given table if both seats are occupied and no hand is in progress.
    /// Generates a cryptographically random seed, logs it to the audit log, and creates the hand.
    /// Returns the HandId if a hand was started, or None otherwise.
    pub async fn start_hand_if_possible(
        &self,
        table_id: &game_engine::TableId,
    ) -> Option<game_engine::HandId> {
        use getrandom::getrandom;
        // Lock table manager
        let mut tm = self.table_manager.lock().await;
        // Check if both seats occupied and no current hand
        let table = match tm.get_table(table_id) {
            Some(t) => t,
            None => return None,
        };
        let config = table.config.clone();
        let button_position = table.next_button_position;
        let occupied_seats: Vec<_> = table.seats.iter().filter_map(|s| s.as_ref()).collect();
        if occupied_seats.len() != 2 || table.current_hand.is_some() {
            return None;
        }
        // Generate random seed
        let mut seed = [0u8; 32];
        if let Err(e) = getrandom(&mut seed) {
            error!("failed to generate random seed: {}", e);
            return None;
        }
        // Start hand
        let hand_id = match tm.start_hand(table_id, seed) {
            Ok(hand_id) => hand_id,
            Err(e) => {
                error!("failed to start hand: {}", e);
                return None;
            }
        };
        // Log seed to audit log (encrypted)
        {
            let mut audit_log = self.audit_log.lock().await;
            if let Err(e) = audit_log.log_seed(hand_id, table_id, &seed) {
                error!("failed to log seed: {}", e);
                // Continue anyway
            }
        }
        info!("started hand {:?} at table {}", hand_id, table_id.as_str());
        // Get the newly created hand
        let hand = tm
            .get_table(table_id)
            .and_then(|t| t.current_hand.as_ref())
            .expect("hand just started");
        let hand_clone = hand.clone();
        drop(tm); // release lock before broadcasting
                  // Broadcast HandState to both seats
        self.connection_manager
            .broadcast_to_table(table_id, |seat| {
                let acting_seat = hand_clone.button_position; // small blind acts first preflop
                let time_remaining_ms = config.action_timeout_secs * 1000;
                create_hand_state_message(
                    &hand_clone,
                    table_id.clone(),
                    seat,
                    Some(acting_seat),
                    time_remaining_ms,
                )
            })
            .await;
        Some(hand_id)
    }

    pub async fn bind(&self) -> Result<TcpListener> {
        let listener = TcpListener::bind(&self.config.bind_address).await?;
        info!("server listening on {}", self.config.bind_address);
        Ok(listener)
    }

    pub async fn run(&self, listener: TcpListener) -> Result<()> {
        let server = self.clone();
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            info!("new connection from {}", peer_addr);
            let server = server.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, server).await {
                    error!("connection error: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(stream: TcpStream, server: Server) -> Result<()> {
    use crate::protocol::codec::{read_message, write_message};
    use crate::protocol::messages::Message;
    use game_engine::{ConnectionId, Player};
    use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
    use tokio::sync::mpsc;
    use tracing::{debug, warn};
    use uuid::Uuid;

    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    // Step 1: read ClientHello
    let client_hello: Message = read_message(&mut reader).await?;
    debug!("received {:?}", client_hello);
    let (version, client_name, client_version) = match client_hello {
        Message::ClientHello {
            version,
            client_name: _client_name,
            client_version: _client_version,
        } => (version, _client_name, _client_version),
        _ => {
            warn!("first message not ClientHello");
            return Ok(());
        }
    };
    // Validate version (simple check: major version == 1)
    if !version.starts_with("1.") {
        // Send error? For now, just close.
        warn!("unsupported version {}", version);
        return Ok(());
    }
    // Send ServerHello accepting the connection
    let server_hello = Message::ServerHello {
        version: "1.0".to_string(),
        status: "accepted".to_string(),
        server_name: "hupoker-server".to_string(),
        server_version: "0.1.0".to_string(),
    };
    write_message(&mut writer, &server_hello).await?;
    writer.flush().await?;
    debug!("sent ServerHello");

    // Create channel for outgoing messages and spawn writer task
    let (tx, mut rx) = mpsc::unbounded_channel();
    let writer_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = write_message(&mut writer, &msg).await {
                debug!("failed to write message: {}", e);
                break;
            }
            if let Err(e) = writer.flush().await {
                debug!("failed to flush writer: {}", e);
                break;
            }
        }
        debug!("writer task exiting");
    });

    // Generate a connection ID for this client
    let connection_id = ConnectionId::new(Uuid::new_v4());
    // Track which table/seat this connection is occupying (if any)
    let mut current_table = None;
    let mut current_seat = None;
    // Main message loop
    loop {
        let msg: Message = read_message(&mut reader).await?;
        debug!("received {:?}", msg);
        match msg {
            Message::JoinTable { version, table_id, seat } => {
                // Validate seat
                if seat != 0 && seat != 1 {
                    // Send error via channel
                    let error = Message::Error {
                        version: "1.0".to_string(),
                        code: "invalid_seat".to_string(),
                        message: "seat must be 0 or 1".to_string(),
                        original_type: "join_table".to_string(),
                    };
                    if tx.send(error).is_err() {
                        break;
                    }
                    continue;
                }
                // Create player object
                let player = Player {
                    seat,
                    stack: 1500, // TODO: get from table config
                    connection_id,
                    disconnected_at: None,
                    is_sitting_out: false,
                };
                // Attempt to occupy seat and get table state
                let result = {
                    let mut tm = server.table_manager.lock().await;
                    if let Err(e) = tm.occupy_seat(&table_id, seat, player) {
                        Err(e)
                    } else {
                        // Build TableState
                        let table = tm.get_table(&table_id).expect("table must exist");
                        let seats = table
                            .seats
                            .iter()
                            .enumerate()
                            .map(|(i, maybe_player)| crate::protocol::messages::TableSeat {
                                seat: i as u8,
                                player: maybe_player.clone(),
                            })
                            .collect();
                        Ok((table.config.clone(), seats))
                    }
                };
                match result {
                    Ok((config, seats)) => {
                        // Register this connection with the connection manager
                        server
                            .connection_manager
                            .register(table_id.clone(), seat, tx.clone())
                            .await;
                        current_table = Some(table_id.clone());
                        current_seat = Some(seat);
                        // Attempt to start a hand if both seats are now occupied
                        let current_hand_id = server.start_hand_if_possible(&table_id).await;
                        let table_state = Message::TableState {
                            version: "1.0".to_string(),
                            table_id,
                            seats,
                            config,
                            current_hand_id,
                        };
                        if tx.send(table_state).is_err() {
                            break;
                        }
                        // TODO: if hand started, broadcast HandState
                    }
                    Err(e) => {
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "seat_taken".to_string(),
                            message: e,
                            original_type: "join_table".to_string(),
                        };
                        if tx.send(error).is_err() {
                            break;
                        }
                        // Unregister this connection if it was registered
                        if let (Some(table_id), Some(seat)) = (current_table, current_seat) {
                            server.connection_manager.unregister(&table_id, seat).await;
                        }
                    }
                }
            }
            Message::Heartbeat { version: _, timestamp: _ } => {
                // Echo heartbeat
                let heartbeat = Message::Heartbeat {
                    version: "1.0".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                if tx.send(heartbeat).is_err() {
                    break;
                }
            }
            _ => {
                warn!("unexpected message type");
                // Send error
                let error = Message::Error {
                    version: "1.0".to_string(),
                    code: "unexpected_message".to_string(),
                    message: "message not allowed in current state".to_string(),
                    original_type: "unknown".to_string(),
                };
                if tx.send(error).is_err() {
                    break;
                }
            }
        }
    }
}
