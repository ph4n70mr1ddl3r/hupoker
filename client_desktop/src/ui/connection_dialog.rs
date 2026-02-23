use crate::connection::Connection;
use anyhow::Result;
use server::protocol::messages::{HandState, TableState};
use std::sync::Arc;
use std::time::Instant;
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct ConnectionDialog {
    pub open: bool,
    pub server_address: String,
    pub connection_status: ConnectionStatus,
    pub error_message: Option<String>,
    pub connection: Option<Connection>,
    pub table_state: Option<TableState>,
    pub hand_state: Option<HandState>,
    pub reconnect_table_id: Option<game_engine::TableId>,
    pub reconnect_seat: Option<u8>,
    pub reconnect_attempts: u32,
    pub last_reconnect_attempt: Option<Instant>,
    pub runtime: Option<Arc<Runtime>>,
}

#[derive(Default, PartialEq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl ConnectionDialog {
    pub fn show(&mut self, ctx: &egui::Context) {
        let mut open = self.open;
        egui::Window::new("Connect to Server")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Server address:");
                ui.text_edit_singleline(&mut self.server_address);
                ui.add_space(10.0);

                match self.connection_status {
                    ConnectionStatus::Disconnected => {
                        if ui.button("Connect").clicked() {
                            self.attempt_connection();
                        }
                    }
                    ConnectionStatus::Connecting => {
                        ui.spinner();
                        ui.label("Connecting...");
                    }

                    ConnectionStatus::Connected => {
                        ui.colored_label(egui::Color32::GREEN, "Connected!");
                        if ui.button("Disconnect").clicked() {
                            self.disconnect();
                        }
                        if let Some(table_state) = &self.table_state {
                            ui.add_space(10.0);
                            ui.separator();
                            ui.heading(format!("Table: {}", table_state.table_id.as_str()));
                            ui.label(format!(
                                "Small blind: {} | Big blind: {}",
                                table_state.config.small_blind, table_state.config.big_blind
                            ));
                            ui.label(format!(
                                "Starting stack: {}",
                                table_state.config.starting_stack
                            ));
                            ui.add_space(5.0);
                            ui.label("Seats:");
                            for seat in &table_state.seats {
                                let player_text = if let Some(player) = &seat.player {
                                    format!("Seat {}: Player (stack: {})", seat.seat, player.stack)
                                } else {
                                    format!("Seat {}: Empty", seat.seat)
                                };
                                ui.label(player_text);
                            }
                        }
                    }
                    ConnectionStatus::Reconnecting => {
                        ui.spinner();
                        ui.label("Reconnecting...");
                        self.attempt_reconnection();
                    }
                }

                if let Some(err) = &self.error_message {
                    ui.add_space(10.0);
                    ui.colored_label(egui::Color32::RED, err);
                }
            });
        self.open = open;
    }

    fn attempt_connection(&mut self) {
        self.connection_status = ConnectionStatus::Connecting;
        self.error_message = None;
        self.table_state = None;
        self.hand_state = None;

        let address = self.server_address.clone();
        let rt = self.runtime.get_or_insert_with(|| {
            Arc::new(Runtime::new().expect("failed to create tokio runtime"))
        });
        let rt = Arc::clone(rt);
        let result: Result<(Connection, TableState, Vec<HandState>), anyhow::Error> =
            rt.block_on(async {
                // Connect TCP
                let mut conn = Connection::connect(&address).await?;
                // Perform handshake
                conn.handshake("hupoker-client", "0.1.0").await?;
                // Join default table (table-0) seat 0
                let table_id = game_engine::TableId::new("table-0".to_string());
                let (table_state, hand_states) = conn.join_table(table_id, 0).await?;
                Ok((conn, table_state, hand_states))
            });
        match result {
            Ok((conn, table_state, hand_states)) => {
                let table_id = table_state.table_id.clone();
                self.connection = Some(conn);
                self.table_state = Some(table_state);
                self.hand_state = hand_states.into_iter().next(); // store first hand state, if any
                self.connection_status = ConnectionStatus::Connected;
                self.reconnect_table_id = Some(table_id);
                // TODO: Support seat selection. Currently hardcoded to seat 0.
                self.reconnect_seat = Some(0);
                self.reconnect_attempts = 0;
                self.last_reconnect_attempt = None;
            }
            Err(e) => {
                // Only start reconnecting if we have reconnect info (i.e., we were previously connected)
                if self.reconnect_table_id.is_some() && self.reconnect_seat.is_some() {
                    self.start_reconnecting();
                } else {
                    self.connection_status = ConnectionStatus::Disconnected;
                }
                self.error_message = Some(format!("Connection failed: {}", e));
            }
        }
    }

    fn disconnect(&mut self) {
        self.connection = None;
        self.table_state = None;
        self.hand_state = None;
        self.connection_status = ConnectionStatus::Disconnected;
    }

    fn start_reconnecting(&mut self) {
        if let Some(table_state) = &self.table_state {
            self.reconnect_table_id = Some(table_state.table_id.clone());
            // TODO: Track which seat we were occupying. Currently hardcoded to seat 0.
            self.reconnect_seat = Some(0);
        }
        self.connection_status = ConnectionStatus::Reconnecting;
        self.reconnect_attempts = 0;
        self.last_reconnect_attempt = None;
    }

    fn attempt_reconnection(&mut self) {
        self.hand_state = None;
        const RECONNECT_DELAY_SECS: u64 = 5;
        let now = Instant::now();
        if let Some(last) = self.last_reconnect_attempt {
            if now.duration_since(last).as_secs() < RECONNECT_DELAY_SECS {
                return;
            }
        }
        self.last_reconnect_attempt = Some(now);
        self.reconnect_attempts += 1;

        let address = self.server_address.clone();
        let Some(table_id) = self.reconnect_table_id.clone() else {
            self.error_message = Some("Cannot reconnect: missing table".to_string());
            self.connection_status = ConnectionStatus::Disconnected;
            return;
        };
        let Some(seat) = self.reconnect_seat else {
            self.error_message = Some("Cannot reconnect: missing seat".to_string());
            self.connection_status = ConnectionStatus::Disconnected;
            return;
        };

        let rt = self.runtime.get_or_insert_with(|| {
            Arc::new(Runtime::new().expect("failed to create tokio runtime"))
        });
        let rt = Arc::clone(rt);
        let result: Result<(Connection, TableState, Vec<HandState>), anyhow::Error> =
            rt.block_on(async {
                let mut conn = Connection::connect(&address).await?;
                conn.handshake("hupoker-client", "0.1.0").await?;
                let (table_state, hand_states) = conn.join_table(table_id, seat).await?;
                Ok((conn, table_state, hand_states))
            });

        match result {
            Ok((conn, table_state, hand_states)) => {
                self.connection = Some(conn);
                self.table_state = Some(table_state);
                self.hand_state = hand_states.into_iter().next(); // store first hand state, if any
                self.connection_status = ConnectionStatus::Connected;
                self.error_message = None;
                self.reconnect_attempts = 0;
            }
            Err(e) => {
                self.error_message = Some(format!(
                    "Reconnection failed (attempt {}): {}",
                    self.reconnect_attempts, e
                ));
                // Stay in Reconnecting state; will retry later
            }
        }
    }
}
