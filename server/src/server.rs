use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{audit_log::AuditLog, table_manager::TableManager};
use game_engine::ServerConfig;

#[derive(Clone)]
pub struct Server {
    config: ServerConfig,
    audit_log: Arc<Mutex<AuditLog>>,
    table_manager: Arc<Mutex<TableManager>>,
}

impl Server {
    pub fn new(config: ServerConfig, audit_log: AuditLog) -> Self {
        use chrono::Utc;
        use game_engine::{Table, TableId};

        // Create tables from config
        let mut table_manager = TableManager::new();
        for (i, table_config) in config.tables.iter().enumerate() {
            let table = Table {
                id: TableId::new(format!("table-{}", i)),
                seats: [None, None],
                current_hand: None,
                config: table_config.clone(),
                created_at: Utc::now(),
            };
            table_manager.add_table(table);
        }

        Self {
            config,
            audit_log: Arc::new(Mutex::new(audit_log)),
            table_manager: Arc::new(Mutex::new(table_manager)),
        }
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
    use tracing::{debug, warn};
    use uuid::Uuid;

    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    // Step 1: read ClientHello
    let client_hello: Message = read_message(&mut reader).await?;
    debug!("received {:?}", client_hello);
    let (version, client_name, client_version) = match client_hello {
        Message::ClientHello { version, client_name, client_version } => {
            (version, client_name, client_version)
        }
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

    // Generate a connection ID for this client
    let connection_id = ConnectionId::new(Uuid::new_v4());
    // Main message loop
    loop {
        let msg: Message = read_message(&mut reader).await?;
        debug!("received {:?}", msg);
        match msg {
            Message::JoinTable { version, table_id, seat } => {
                // Validate seat
                if seat != 0 && seat != 1 {
                    // Send error
                    let error = Message::Error {
                        version: "1.0".to_string(),
                        code: "invalid_seat".to_string(),
                        message: "seat must be 0 or 1".to_string(),
                        original_type: "join_table".to_string(),
                    };
                    write_message(&mut writer, &error).await?;
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
                        let table_state = Message::TableState {
                            version: "1.0".to_string(),
                            table_id,
                            seats,
                            config,
                            current_hand_id: None,
                        };
                        write_message(&mut writer, &table_state).await?;
                    }
                    Err(e) => {
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "seat_taken".to_string(),
                            message: e,
                            original_type: "join_table".to_string(),
                        };
                        write_message(&mut writer, &error).await?;
                    }
                }
                writer.flush().await?;
            }
            Message::Heartbeat { version: _, timestamp: _ } => {
                // Echo heartbeat
                let heartbeat = Message::Heartbeat {
                    version: "1.0".to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                write_message(&mut writer, &heartbeat).await?;
                writer.flush().await?;
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
                write_message(&mut writer, &error).await?;
                writer.flush().await?;
            }
        }
    }
}
