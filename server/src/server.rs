use crate::constants::{MILLISECONDS_PER_SECOND, NUM_SEATS, PROTOCOL_VERSION};
use crate::protocol::messages::Message;
use crate::{
    audit_log::AuditLog,
    connection_manager::{ConnectionManager, ConnectionSender},
    hand_state::create_hand_state_message,
    table_manager::TableManager,
};
use anyhow::Result;
use chrono::Utc;
use game_engine::{Action, ActionKind, ServerConfig, Street};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{interval, timeout, Duration};
use tracing::{error, info, warn};

const DEFAULT_STACK_SIZE: u64 = 1500;
const ALL_SEATS: &[u8; 2] = &[0, 1];
const HANDSHAKE_TIMEOUT_SECS: u64 = 10;

#[derive(Clone)]
pub struct Server {
    config: ServerConfig,
    audit_log: Arc<Mutex<AuditLog>>,
    table_manager: Arc<Mutex<TableManager>>,
    connection_manager: ConnectionManager,
}

impl Server {
    fn award_pots_to_winners(hand: &game_engine::Hand, winners: &[game_engine::Seat]) -> [u64; 2] {
        let button = hand.button_position;
        let mut pots: Vec<(u64, &[u8])> = vec![(hand.pot.main, ALL_SEATS)];
        for side_pot in &hand.pot.side_pots {
            pots.push((side_pot.amount, &side_pot.eligible_seats));
        }
        let mut awards = [0u64, 0u64];
        for (amount, eligible_seats) in pots {
            let mut eligible_winners: Vec<_> =
                winners.iter().filter(|&seat| eligible_seats.contains(seat)).copied().collect();
            if eligible_winners.is_empty() {
                continue;
            }
            let share = amount / eligible_winners.len() as u64;
            let remainder = amount % eligible_winners.len() as u64;
            if remainder > 0 {
                eligible_winners.sort_by_key(|&s| if s == button { 0 } else { 1 });
            }
            for (idx, &seat) in eligible_winners.iter().enumerate() {
                let mut award = share;
                if idx == 0 && remainder > 0 {
                    award = award.saturating_add(remainder);
                }
                if (seat as usize) < awards.len() {
                    awards[seat as usize] = awards[seat as usize].saturating_add(award);
                }
            }
        }
        awards
    }

    async fn finish_hand(
        &self,
        table_id: &game_engine::TableId,
        hand: &game_engine::Hand,
        winners: &[game_engine::Seat],
    ) {
        let mut tm = self.table_manager.lock().await;
        let table = match tm.get_table_mut(table_id) {
            Some(t) => t,
            None => {
                error!("table {} not found in finish_hand", table_id.as_str());
                return;
            }
        };

        if !winners.is_empty() {
            let awards = Self::award_pots_to_winners(hand, winners);
            for (seat, award) in awards.iter().enumerate() {
                if let Some(player) = table.seats[seat].as_mut() {
                    player.stack = player.stack.saturating_add(*award);
                }
            }
        }
        table.current_hand = None;
        let table_state =
            Self::create_table_state_message(table_id, &table.seats, &table.config, None);
        drop(tm);
        self.connection_manager.broadcast_to_table(table_id, |_| table_state.clone()).await;
    }

    fn create_table_state_message(
        table_id: &game_engine::TableId,
        seats: &[Option<game_engine::Player>; 2],
        config: &game_engine::TableConfig,
        current_hand_id: Option<game_engine::HandId>,
    ) -> Message {
        Message::TableState {
            version: PROTOCOL_VERSION.to_string(),
            table_id: table_id.clone(),
            seats: seats
                .iter()
                .enumerate()
                .map(|(i, maybe_player)| crate::protocol::messages::TableSeat {
                    seat: i as u8,
                    player: maybe_player.clone(),
                })
                .collect(),
            config: config.clone(),
            current_hand_id,
        }
    }

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
    /// Returns Ok(Some(HandId)) if a hand was started, Ok(None) if conditions not met,
    /// or Err if an error occurred.
    pub async fn start_hand_if_possible(
        &self,
        table_id: &game_engine::TableId,
    ) -> Result<Option<game_engine::HandId>, anyhow::Error> {
        use getrandom::getrandom;
        // Lock table manager
        let mut tm = self.table_manager.lock().await;
        // Check if both seats occupied and no current hand
        let table = match tm.get_table(table_id) {
            Some(t) => t,
            None => return Ok(None),
        };
        let config = table.config.clone();
        let occupied_seats: Vec<_> = table.seats.iter().filter_map(|s| s.as_ref()).collect();
        if occupied_seats.len() != 2 || table.current_hand.is_some() {
            return Ok(None);
        }
        // Generate random seed
        let mut seed = [0u8; 32];
        getrandom(&mut seed)
            .map_err(|e| anyhow::anyhow!("failed to generate random seed: {}", e))?;
        // Log seed to audit log (encrypted) BEFORE starting hand - critical for game verifiability
        // We generate the hand_id first so we can log atomically
        let hand_id = game_engine::HandId::new_v4();
        {
            let mut audit_log = self.audit_log.lock().await;
            audit_log.log_seed(hand_id, table_id, &seed).map_err(|e| {
                anyhow::anyhow!(
                    "failed to log seed for hand {}: {}. Aborting hand start.",
                    hand_id,
                    e
                )
            })?;
        }
        // Start hand with pre-generated ID
        tm.start_hand_with_id(table_id, seed, hand_id)
            .map_err(|e| anyhow::anyhow!("failed to start hand: {}", e))?;
        drop(tm);
        info!("started hand {:?} at table {}", hand_id, table_id.as_str());
        let tm = self.table_manager.lock().await;
        let hand = match tm.get_table(table_id).and_then(|t| t.current_hand.as_ref()) {
            Some(h) => h.clone(),
            None => {
                error!("hand disappeared after starting");
                return Ok(Some(hand_id));
            }
        };
        drop(tm);
        self.connection_manager
            .broadcast_to_table(table_id, |seat| {
                let acting_seat = hand.betting.acting_seat();
                let time_remaining_ms = config.action_timeout_secs.saturating_mul(1000);
                create_hand_state_message(
                    &hand,
                    table_id.clone(),
                    seat,
                    acting_seat,
                    time_remaining_ms,
                )
            })
            .await;
        Ok(Some(hand_id))
    }

    pub async fn apply_auto_fold(&self, table_id: game_engine::TableId, seat: game_engine::Seat) {
        use chrono::Utc;
        use game_engine::{Action, ActionKind};
        use tracing::error;

        let mut tm = self.table_manager.lock().await;
        let table = match tm.get_table_mut(&table_id) {
            Some(table) => table,
            None => return,
        };
        let hand = match table.current_hand.as_mut() {
            Some(hand) => hand,
            None => return,
        };
        // Verify it's this seat's turn (should be true if timeout detected)
        let acting_seat = hand.betting.acting_seat();
        if acting_seat != Some(seat) {
            return;
        }
        // Create fold action
        let action = Action { seat, kind: ActionKind::Fold, amount: None, timestamp: Utc::now() };
        // Apply action
        if let Err(e) = hand.apply_action(action.clone()) {
            error!("failed to apply auto-fold: {}", e);
            return;
        }
        // Clone needed data for logging and broadcasting
        let hand_id = hand.id;
        let action_kind = action.kind;
        let action_amount = action.amount;
        let hand_clone = hand.clone();
        let config = table.config.clone();
        drop(tm);
        // Log action
        {
            let mut audit_log = self.audit_log.lock().await;
            if let Err(e) = audit_log.log_action(hand_id, seat, action_kind, action_amount) {
                error!("failed to log auto-fold action: {}", e);
            }
        }

        // Broadcast updated hand state (fold action applied)
        let next_acting_seat = hand_clone.betting.acting_seat();
        let time_remaining_ms = config.action_timeout_secs.saturating_mul(MILLISECONDS_PER_SECOND);
        self.connection_manager
            .broadcast_to_table(&table_id, |player_seat| {
                crate::hand_state::create_hand_state_message(
                    &hand_clone,
                    table_id.clone(),
                    player_seat,
                    next_acting_seat,
                    time_remaining_ms,
                )
            })
            .await;

        // Hand ends due to fold; evaluate winner and award pot
        let winners = {
            let mut tm = self.table_manager.lock().await;
            let table = match tm.get_table_mut(&table_id) {
                Some(table) => table,
                None => return,
            };
            let hand = match table.current_hand.as_ref() {
                Some(hand) => hand,
                None => return,
            };
            hand.evaluate_winner()
        };
        self.log_hand_end_to_audit_log(&hand_clone, &table_id, &winners).await;
        // Finish hand with fresh state
        {
            let mut tm = self.table_manager.lock().await;
            let table = match tm.get_table_mut(&table_id) {
                Some(table) => table,
                None => return,
            };
            if let Some(hand) = table.current_hand.take() {
                self.finish_hand(&table_id, &hand, &winners).await;
            }
        }
    }

    /// Mark a player as disconnected and unregister their connection.
    /// If table_id or seat is None, does nothing.
    pub async fn cleanup_connection(
        &self,
        table_id: Option<&game_engine::TableId>,
        seat: Option<game_engine::Seat>,
    ) {
        let (table_id, seat) = match (table_id, seat) {
            (Some(table_id), Some(seat)) => (table_id, seat),
            _ => return,
        };
        // Mark player as disconnected
        let marked = {
            let mut tm = self.table_manager.lock().await;
            tm.mark_disconnected(table_id, seat)
        };
        if marked {
            // Unregister from connection manager
            self.connection_manager.unregister(table_id, seat).await;
        }
    }

    async fn log_hand_end_to_audit_log(
        &self,
        hand: &game_engine::Hand,
        table_id: &game_engine::TableId,
        winners: &[game_engine::Seat],
    ) {
        let pot_total =
            hand.pot.main.saturating_add(hand.pot.side_pots.iter().map(|p| p.amount).sum::<u64>());
        let mut audit_log = self.audit_log.lock().await;
        if let Err(e) = audit_log.log_hand_end(hand.id, table_id, winners.to_vec(), pot_total) {
            error!("failed to log hand end: {}", e);
        }
    }

    /// Starts a background task that periodically checks for action timeouts
    /// and applies auto‑fold to players who have exceeded the timeout.
    pub fn start_timeout_checker(self) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let timed_out = {
                    let tm = self.table_manager.lock().await;
                    tm.check_action_timeouts()
                };
                for (table_id, seat) in timed_out {
                    if let Err(e) = self.apply_auto_fold_safe(&table_id, seat).await {
                        error!(
                            "failed to apply auto-fold for seat {} at table {}: {}",
                            seat,
                            table_id.as_str(),
                            e
                        );
                    }
                }
            }
        });
    }

    async fn apply_auto_fold_safe(
        &self,
        table_id: &game_engine::TableId,
        seat: game_engine::Seat,
    ) -> Result<(), anyhow::Error> {
        self.apply_auto_fold(table_id.clone(), seat).await;
        Ok(())
    }

    pub async fn bind(&self) -> Result<TcpListener> {
        let listener = TcpListener::bind(&self.config.bind_address).await?;
        info!("server listening on {}", self.config.bind_address);
        Ok(listener)
    }

    fn make_error_msg(code: &str, message: &str, original_type: &str) -> Message {
        Message::Error {
            version: PROTOCOL_VERSION.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            original_type: original_type.to_string(),
        }
    }

    pub async fn run(&self, listener: TcpListener) -> Result<()> {
        let server = self.clone();
        // Start background timeout checker
        server.clone().start_timeout_checker();
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

async fn handle_handshake(
    reader: &mut tokio::io::BufReader<tokio::io::ReadHalf<TcpStream>>,
    writer: &mut tokio::io::BufWriter<tokio::io::WriteHalf<TcpStream>>,
) -> Result<bool> {
    use crate::protocol::codec::{read_message, write_message};
    use crate::protocol::messages::Message;
    use anyhow::Context;
    use tokio::io::AsyncWriteExt;
    use tracing::{debug, warn};

    let client_hello: Message =
        timeout(Duration::from_secs(HANDSHAKE_TIMEOUT_SECS), read_message(reader))
            .await
            .context("handshake timeout")?
            .context("failed to read handshake")?;
    debug!("received {:?}", client_hello);
    let (version, client_name, client_version) = match client_hello {
        Message::ClientHello { version, client_name, client_version } => {
            (version, client_name, client_version)
        }
        _ => {
            warn!("first message not ClientHello, got: {:?}", client_hello);
            return Ok(false);
        }
    };
    if !is_valid_version(&version) {
        warn!("unsupported version {}, expected 1.x", version);
        return Ok(false);
    }
    if !is_valid_client_string(&client_name) {
        warn!("invalid client_name: {:?}", client_name);
        return Ok(false);
    }
    if !is_valid_client_string(&client_version) {
        warn!("invalid client_version: {:?}", client_version);
        return Ok(false);
    }
    let server_hello = Message::ServerHello {
        version: PROTOCOL_VERSION.to_string(),
        status: "accepted".to_string(),
        server_name: "hupoker-server".to_string(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    write_message(writer, &server_hello).await?;
    writer.flush().await?;
    debug!("sent ServerHello");
    Ok(true)
}

fn is_valid_version(version: &str) -> bool {
    let version = version.trim();
    if version.len() > 32 || version.is_empty() {
        return false;
    }
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) && parts[0] == "1"
}

fn is_valid_client_string(s: &str) -> bool {
    s.len() <= 64
        && s.chars().all(|c| {
            c.is_ascii()
                && !c.is_control()
                && (c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        })
}

/// Handles a JoinTable message from a client.
/// Attempts to occupy the requested seat and sends table state.
/// Returns true if connection should continue, false if connection should close.
async fn handle_join_table(
    server: &Server,
    table_id: game_engine::TableId,
    seat: u8,
    connection_id: game_engine::ConnectionId,
    tx: &ConnectionSender,
    current_table: &mut Option<game_engine::TableId>,
    current_seat: &mut Option<u8>,
) -> bool {
    use crate::protocol::messages::Message;
    use game_engine::Player;
    use tracing::error;

    // Validate seat
    if seat >= NUM_SEATS {
        // Send error via channel
        let error = Message::Error {
            version: PROTOCOL_VERSION.to_string(),
            code: "invalid_seat".to_string(),
            message: format!("seat must be between 0 and {}", NUM_SEATS - 1),
            original_type: "join_table".to_string(),
        };
        if tx.try_send(error).is_err() {
            server.cleanup_connection(current_table.as_ref(), *current_seat).await;
        }
        // Close connection after sending error
        return false;
    }
    // Attempt to occupy seat and get table state
    let result: Result<_, String> = {
        let mut tm = server.table_manager.lock().await;
        // Get starting stack from table config (default to 1500 if table not found)
        let starting_stack =
            tm.get_table(&table_id).map(|t| t.config.starting_stack).unwrap_or(DEFAULT_STACK_SIZE);
        // Create player object
        let player = Player {
            seat,
            stack: starting_stack,
            connection_id,
            disconnected_at: None,
            is_sitting_out: false,
        };
        if let Err(e) = tm.occupy_seat(&table_id, seat, player) {
            Err(e)
        } else {
            // Build TableState
            let table = match tm.get_table(&table_id) {
                Some(t) => t,
                None => {
                    server.cleanup_connection(current_table.as_ref(), *current_seat).await;
                    return false;
                }
            };
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
            server.connection_manager.register(table_id.clone(), seat, tx.clone()).await;
            *current_table = Some(table_id.clone());
            *current_seat = Some(seat);
            // Attempt to start a hand if both seats are now occupied
            let current_hand_id = match server.start_hand_if_possible(&table_id).await {
                Ok(hand_id) => hand_id,
                Err(e) => {
                    error!("failed to start hand: {}", e);
                    None
                }
            };
            let table_state = Message::TableState {
                version: PROTOCOL_VERSION.to_string(),
                table_id: table_id.clone(),
                seats,
                config,
                current_hand_id,
            };
            if tx.try_send(table_state).is_err() {
                server.cleanup_connection(current_table.as_ref(), *current_seat).await;
                return false;
            }
            // If a hand is already in progress, send HandState to this player
            let hand_state_msg = {
                let tm = server.table_manager.lock().await;
                if let Some(table) = tm.get_table(&table_id) {
                    if let Some(hand) = &table.current_hand {
                        let time_remaining_ms = table
                            .config
                            .action_timeout_secs
                            .saturating_mul(MILLISECONDS_PER_SECOND);
                        let acting_seat = hand.betting.acting_seat();
                        Some(create_hand_state_message(
                            hand,
                            table_id.clone(),
                            seat,
                            acting_seat,
                            time_remaining_ms,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(msg) = hand_state_msg {
                if tx.try_send(msg).is_err() {
                    server.cleanup_connection(current_table.as_ref(), *current_seat).await;
                    return false;
                }
            }
        }
        Err(e) => {
            let error = Message::Error {
                version: PROTOCOL_VERSION.to_string(),
                code: "seat_taken".to_string(),
                message: e,
                original_type: "join_table".to_string(),
            };
            if tx.try_send(error).is_err() {
                server.cleanup_connection(current_table.as_ref(), *current_seat).await;
                return false;
            }
            // Unregister this connection if it was registered
            if let (Some(table_id), Some(seat)) = (current_table.as_ref(), *current_seat) {
                server.connection_manager.unregister(table_id, seat).await;
            }
        }
    }
    true
}

/// Handles a Heartbeat message from a client.
/// Echoes the heartbeat back to the client.
/// Returns true if connection should continue, false if connection should close.
async fn handle_heartbeat(
    tx: &ConnectionSender,
    server: &Server,
    current_table: &Option<game_engine::TableId>,
    current_seat: Option<u8>,
) -> bool {
    use crate::protocol::messages::Message;
    use chrono::Utc;

    // Echo heartbeat
    let heartbeat = Message::Heartbeat {
        version: PROTOCOL_VERSION.to_string(),
        timestamp: Utc::now().to_rfc3339(),
    };
    if tx.try_send(heartbeat).is_err() {
        server.cleanup_connection(current_table.as_ref(), current_seat).await;
        false
    } else {
        true
    }
}

async fn handle_connection(stream: TcpStream, server: Server) -> Result<()> {
    use crate::protocol::codec::{read_message, write_message};
    use crate::protocol::messages::Message;
    use game_engine::ConnectionId;
    use tokio::io::{AsyncWriteExt, BufReader, BufWriter};
    use tokio::sync::mpsc;
    use tracing::debug;
    use uuid::Uuid;

    let (read_half, write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut writer = BufWriter::new(write_half);

    // Perform handshake
    if !handle_handshake(&mut reader, &mut writer).await? {
        return Ok(());
    }

    // Create bounded channel for outgoing messages (100 msg buffer for backpressure)
    let (tx, mut rx) = mpsc::channel(100);
    tokio::spawn(async move {
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
        let msg: Message = match read_message(&mut reader).await {
            Ok(msg) => msg,
            Err(e) => {
                server.cleanup_connection(current_table.as_ref(), current_seat).await;
                return Err(e);
            }
        };
        debug!("received {:?}", msg);
        match msg {
            Message::JoinTable { version: _, table_id, seat } => {
                if !handle_join_table(
                    &server,
                    table_id,
                    seat,
                    connection_id,
                    &tx,
                    &mut current_table,
                    &mut current_seat,
                )
                .await
                {
                    break Ok(());
                }
            }
            Message::Heartbeat { .. } => {
                if !handle_heartbeat(&tx, &server, &current_table, current_seat).await {
                    break Ok(());
                }
            }
            Message::Action { version: _, hand_id, kind, amount } => {
                // Validate seat and table
                let (table_id, seat) = match (current_table.as_ref(), current_seat) {
                    (Some(table_id), Some(seat)) => (table_id.clone(), seat),
                    _ => {
                        let error = Server::make_error_msg(
                            "not_at_table",
                            "must join a table before acting",
                            "action",
                        );
                        if tx.try_send(error).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        continue;
                    }
                };
                // Lock table manager and get hand
                let mut tm = server.table_manager.lock().await;
                let table = match tm.get_table_mut(&table_id) {
                    Some(table) => table,
                    None => {
                        drop(tm);
                        let error = Server::make_error_msg(
                            "table_not_found",
                            "table no longer exists",
                            "action",
                        );
                        if tx.try_send(error).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        continue;
                    }
                };
                let hand = match table.current_hand.as_mut() {
                    Some(hand) if hand.id == hand_id => hand,
                    _ => {
                        drop(tm);
                        let error = Server::make_error_msg(
                            "hand_not_found",
                            "hand not found or not active",
                            "action",
                        );
                        if tx.try_send(error).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        continue;
                    }
                };
                // Validate it's player's turn
                let acting_seat = hand.betting.acting_seat();
                if acting_seat != Some(seat) {
                    drop(tm);
                    let error = Server::make_error_msg(
                        "not_your_turn",
                        "it is not your turn to act",
                        "action",
                    );
                    if tx.try_send(error).is_err() {
                        server.cleanup_connection(current_table.as_ref(), current_seat).await;
                        break Ok(());
                    }
                    continue;
                }
                // Create action struct
                let action = Action { seat, kind, amount, timestamp: Utc::now() };
                // Apply action
                if let Err(e) = hand.apply_action(action.clone()) {
                    drop(tm);
                    let error = Server::make_error_msg("illegal_action", &e.to_string(), "action");
                    if tx.try_send(error).is_err() {
                        server.cleanup_connection(current_table.as_ref(), current_seat).await;
                        break Ok(());
                    }
                    continue;
                }
                // Clone needed data for logging and broadcasting
                let hand_id = hand.id;
                let action_kind = kind;
                let action_amount = amount;
                let config = table.config.clone();
                let hand_clone = hand.clone();
                drop(tm);
                // Log action to audit log
                {
                    let mut audit_log = server.audit_log.lock().await;
                    if let Err(e) = audit_log.log_action(hand_id, seat, action_kind, action_amount)
                    {
                        error!("failed to log action: {}", e);
                    }
                }

                // Determine acting seat for next player (or None if round complete)
                let next_acting_seat = hand_clone.betting.acting_seat();
                let time_remaining_ms =
                    config.action_timeout_secs.saturating_mul(MILLISECONDS_PER_SECOND);
                server
                    .connection_manager
                    .broadcast_to_table(&table_id, |player_seat| {
                        create_hand_state_message(
                            &hand_clone,
                            table_id.clone(),
                            player_seat,
                            next_acting_seat,
                            time_remaining_ms,
                        )
                    })
                    .await;

                // Handle fold immediately (hand ends)
                if action.kind == ActionKind::Fold {
                    let winners = {
                        let mut tm = server.table_manager.lock().await;
                        let table = match tm.get_table_mut(&table_id) {
                            Some(table) => table,
                            None => continue,
                        };
                        let hand = match table.current_hand.as_ref() {
                            Some(hand) => hand,
                            None => continue,
                        };
                        hand.evaluate_winner()
                    };
                    server.log_hand_end_to_audit_log(&hand_clone, &table_id, &winners).await;
                    {
                        let mut tm = server.table_manager.lock().await;
                        let table = match tm.get_table_mut(&table_id) {
                            Some(table) => table,
                            None => continue,
                        };
                        if let Some(hand) = table.current_hand.take() {
                            server.finish_hand(&table_id, &hand, &winners).await;
                        }
                    }
                } else if hand_clone.betting.is_round_complete() {
                    // Re-lock table manager to advance street
                    let mut tm = server.table_manager.lock().await;
                    let table = match tm.get_table_mut(&table_id) {
                        Some(table) => table,
                        None => continue, // table disappeared
                    };
                    let hand = match table.current_hand.as_mut() {
                        Some(hand) if hand.id == hand_id => hand,
                        _ => continue,
                    };
                    // Advance street
                    if let Err(e) = hand.advance_street() {
                        error!("failed to advance street: {}", e);
                        // If street advance fails, maybe hand is finished? We'll ignore for now.
                    } else {
                        // Street advanced; broadcast new hand state
                        let hand_clone = hand.clone();
                        let config = table.config.clone();
                        drop(tm);
                        let next_acting_seat = hand_clone.betting.acting_seat();
                        let time_remaining_ms =
                            config.action_timeout_secs.saturating_mul(MILLISECONDS_PER_SECOND);
                        server
                            .connection_manager
                            .broadcast_to_table(&table_id, |player_seat| {
                                create_hand_state_message(
                                    &hand_clone,
                                    table_id.clone(),
                                    player_seat,
                                    next_acting_seat,
                                    time_remaining_ms,
                                )
                            })
                            .await;
                        // If street is Showdown, evaluate winner and award pot
                        if hand_clone.current_street == Street::Showdown {
                            let winners = {
                                let mut tm = server.table_manager.lock().await;
                                let table = match tm.get_table_mut(&table_id) {
                                    Some(table) => table,
                                    None => continue,
                                };
                                let hand = match table.current_hand.as_ref() {
                                    Some(hand) => hand,
                                    None => continue,
                                };
                                hand.evaluate_winner()
                            };
                            server
                                .log_hand_end_to_audit_log(&hand_clone, &table_id, &winners)
                                .await;
                            {
                                let mut tm = server.table_manager.lock().await;
                                let table = match tm.get_table_mut(&table_id) {
                                    Some(table) => table,
                                    None => continue,
                                };
                                if let Some(hand) = table.current_hand.take() {
                                    server.finish_hand(&table_id, &hand, &winners).await;
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                warn!("unexpected message type: {:?}", msg);
                let error = Server::make_error_msg(
                    "unexpected_message",
                    "message not allowed in current state",
                    "unknown",
                );
                if tx.try_send(error).is_err() {
                    server.cleanup_connection(current_table.as_ref(), current_seat).await;
                    break Ok(());
                }
            }
        }
    }
}
