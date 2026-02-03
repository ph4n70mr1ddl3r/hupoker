use anyhow::Result;
use chrono::Utc;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, Duration};

use tokio::sync::Mutex;
use tracing::{error, info};

use crate::protocol::messages::Message;
use crate::{
    audit_log::AuditLog, connection_manager::ConnectionManager,
    hand_state::create_hand_state_message, table_manager::TableManager,
};
use game_engine::{Action, ActionKind, ServerConfig, Street};

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
        let mut pots = vec![(hand.pot.main, vec![0, 1])];
        for side_pot in &hand.pot.side_pots {
            pots.push((side_pot.amount, side_pot.eligible_seats.clone()));
        }
        let mut awards = [0u64, 0u64];
        for (amount, eligible_seats) in pots {
            let eligible_winners: Vec<_> =
                winners.iter().filter(|&seat| eligible_seats.contains(seat)).copied().collect();
            if eligible_winners.is_empty() {
                continue;
            }
            let share = amount / eligible_winners.len() as u64;
            let remainder = amount % eligible_winners.len() as u64;
            for (idx, &seat) in eligible_winners.iter().enumerate() {
                let mut award = share;
                if idx == 0 && remainder > 0 {
                    let mut sorted = eligible_winners.clone();
                    sorted.sort_by_key(|&s| if s == button { 0 } else { 1 });
                    if seat == sorted[0] {
                        award += remainder;
                    }
                }
                awards[seat as usize] += award;
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
        if winners.is_empty() {
            let mut tm = self.table_manager.lock().await;
            let table = tm.get_table_mut(table_id).expect("table must exist");
            table.current_hand = None;
            let table_state =
                Self::create_table_state_message(table_id, &table.seats, &table.config, None);
            drop(tm);
            self.connection_manager.broadcast_to_table(table_id, |_| table_state.clone()).await;
        } else {
            let awards = Self::award_pots_to_winners(hand, winners);
            let mut tm = self.table_manager.lock().await;
            let table = tm.get_table_mut(table_id).expect("table must exist");
            for (seat, award) in awards.iter().enumerate() {
                if let Some(player) = table.seats[seat].as_mut() {
                    player.stack += award;
                }
            }
            table.current_hand = None;
            let table_state =
                Self::create_table_state_message(table_id, &table.seats, &table.config, None);
            drop(tm);
            self.connection_manager.broadcast_to_table(table_id, |_| table_state.clone()).await;
        }
    }

    fn create_table_state_message(
        table_id: &game_engine::TableId,
        seats: &[Option<game_engine::Player>; 2],
        config: &game_engine::TableConfig,
        current_hand_id: Option<game_engine::HandId>,
    ) -> Message {
        Message::TableState {
            version: "1.0".to_string(),
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
                let acting_seat = button_position; // small blind acts first preflop
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
        let acting_seat = hand.betting.acting_seat(hand.button_position);
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
        // Log action
        {
            let mut audit_log = self.audit_log.lock().await;
            if let Err(e) = audit_log.log_action(hand.id, seat, action.kind, action.amount) {
                error!("failed to log auto-fold action: {}", e);
            }
        }
        // Clone hand for broadcasting and hand end processing
        let hand_clone = hand.clone();
        let config = table.config.clone();
        drop(tm); // release lock before broadcasting

        // Broadcast updated hand state (fold action applied)
        let next_acting_seat = hand_clone.betting.acting_seat(hand_clone.button_position);
        let time_remaining_ms = config.action_timeout_secs * 1000;
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
        let winners = hand_clone.evaluate_winner();
        // Log hand end to audit log (already done in manual fold block, but we need to do here)
        {
            let pot_total = hand_clone.pot.main
                + hand_clone.pot.side_pots.iter().map(|p| p.amount).sum::<u64>();
            let mut audit_log = self.audit_log.lock().await;
            if let Err(e) =
                audit_log.log_hand_end(hand_clone.id, &table_id, winners.clone(), pot_total)
            {
                error!("failed to log hand end: {}", e);
            }
        }
        self.finish_hand(&table_id, &hand_clone, &winners).await;
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

    /// Starts a background task that periodically checks for action timeouts
    /// and applies auto‑fold to players who have exceeded the timeout.
    pub fn start_timeout_checker(self) {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                // Check for timed‑out players
                let timed_out = {
                    let tm = self.table_manager.lock().await;
                    tm.check_action_timeouts()
                };
                for (table_id, seat) in timed_out {
                    self.apply_auto_fold(table_id, seat).await;
                }
            }
        });
    }

    pub async fn bind(&self) -> Result<TcpListener> {
        let listener = TcpListener::bind(&self.config.bind_address).await?;
        info!("server listening on {}", self.config.bind_address);
        Ok(listener)
    }

    pub async fn run(&self, listener: TcpListener) -> Result<()> {
        let server = self.clone();
        // Start background timeout checker
        self.clone().start_timeout_checker();
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
    let (version, _client_name, _client_version) = match client_hello {
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
                        break Ok(());
                    }
                    continue;
                }
                // Attempt to occupy seat and get table state
                let result: Result<_, String> = {
                    let mut tm = server.table_manager.lock().await;
                    // Get starting stack from table config (default to 1500 if table not found)
                    let starting_stack =
                        tm.get_table(&table_id).map(|t| t.config.starting_stack).unwrap_or(1500);
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
                            table_id: table_id.clone(),
                            seats,
                            config,
                            current_hand_id,
                        };
                        if tx.send(table_state).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        // If a hand is already in progress, send HandState to this player
                        let hand_state_msg = {
                            let tm = server.table_manager.lock().await;
                            if let Some(table) = tm.get_table(&table_id) {
                                if let Some(hand) = &table.current_hand {
                                    let time_remaining_ms = table.config.action_timeout_secs * 1000;
                                    let acting_seat =
                                        hand.betting.acting_seat(hand.button_position);
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
                            if tx.send(msg).is_err() {
                                server
                                    .cleanup_connection(current_table.as_ref(), current_seat)
                                    .await;
                                break Ok(());
                            }
                        }
                    }
                    Err(e) => {
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "seat_taken".to_string(),
                            message: e,
                            original_type: "join_table".to_string(),
                        };
                        if tx.send(error).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        // Unregister this connection if it was registered
                        if let (Some(table_id), Some(seat)) = (current_table.as_ref(), current_seat)
                        {
                            server.connection_manager.unregister(table_id, seat).await;
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
                    server.cleanup_connection(current_table.as_ref(), current_seat).await;
                    break Ok(());
                }
            }
            Message::Action { version: _, hand_id, kind, amount } => {
                // Validate seat and table
                let (table_id, seat) = match (current_table.as_ref(), current_seat) {
                    (Some(table_id), Some(seat)) => (table_id.clone(), seat),
                    _ => {
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "not_at_table".to_string(),
                            message: "must join a table before acting".to_string(),
                            original_type: "action".to_string(),
                        };
                        if tx.send(error).is_err() {
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
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "table_not_found".to_string(),
                            message: "table no longer exists".to_string(),
                            original_type: "action".to_string(),
                        };
                        if tx.send(error).is_err() {
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
                        let error = Message::Error {
                            version: "1.0".to_string(),
                            code: "hand_not_found".to_string(),
                            message: "hand not found or not active".to_string(),
                            original_type: "action".to_string(),
                        };
                        if tx.send(error).is_err() {
                            server.cleanup_connection(current_table.as_ref(), current_seat).await;
                            break Ok(());
                        }
                        continue;
                    }
                };
                // Validate it's player's turn
                let acting_seat = hand.betting.acting_seat(hand.button_position);
                if acting_seat != Some(seat) {
                    drop(tm);
                    let error = Message::Error {
                        version: "1.0".to_string(),
                        code: "not_your_turn".to_string(),
                        message: "it is not your turn to act".to_string(),
                        original_type: "action".to_string(),
                    };
                    if tx.send(error).is_err() {
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
                    let error = Message::Error {
                        version: "1.0".to_string(),
                        code: "illegal_action".to_string(),
                        message: e,
                        original_type: "action".to_string(),
                    };
                    if tx.send(error).is_err() {
                        server.cleanup_connection(current_table.as_ref(), current_seat).await;
                        break Ok(());
                    }
                    continue;
                }
                // Log action to audit log
                {
                    let mut audit_log = server.audit_log.lock().await;
                    if let Err(e) = audit_log.log_action(hand.id, seat, kind, amount) {
                        error!("failed to log action: {}", e);
                    }
                }
                // Success: broadcast updated hand state
                let config = table.config.clone();
                let hand_clone = hand.clone();
                drop(tm); // release lock before broadcasting

                // Determine acting seat for next player (or None if round complete)
                let next_acting_seat = hand_clone.betting.acting_seat(hand_clone.button_position);
                let time_remaining_ms = config.action_timeout_secs * 1000;
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
                    // Evaluate winner (should be the other player)
                    let winners = hand_clone.evaluate_winner();
                    // Log hand end to audit log
                    {
                        let pot_total = hand_clone.pot.main
                            + hand_clone.pot.side_pots.iter().map(|p| p.amount).sum::<u64>();
                        let mut audit_log = server.audit_log.lock().await;
                        if let Err(e) = audit_log.log_hand_end(
                            hand_clone.id,
                            &table_id,
                            winners.clone(),
                            pot_total,
                        ) {
                            error!("failed to log hand end: {}", e);
                        }
                    }
                    server.finish_hand(&table_id, &hand_clone, &winners).await;
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
                        let next_acting_seat =
                            hand_clone.betting.acting_seat(hand_clone.button_position);
                        let time_remaining_ms = config.action_timeout_secs * 1000;
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
                            // Evaluate winners
                            let winners = hand_clone.evaluate_winner();
                            // Log hand end to audit log
                            {
                                let pot_total = hand_clone.pot.main
                                    + hand_clone
                                        .pot
                                        .side_pots
                                        .iter()
                                        .map(|p| p.amount)
                                        .sum::<u64>();
                                let mut audit_log = server.audit_log.lock().await;
                                if let Err(e) = audit_log.log_hand_end(
                                    hand_clone.id,
                                    &table_id,
                                    winners.clone(),
                                    pot_total,
                                ) {
                                    error!("failed to log hand end: {}", e);
                                }
                            }
                            server.finish_hand(&table_id, &hand_clone, &winners).await;
                        }
                    }
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
                    server.cleanup_connection(current_table.as_ref(), current_seat).await;
                    break Ok(());
                }
            }
        }
    }
}
