use crate::connection::Connection;
use anyhow::Result;
#[allow(clippy::single_component_path_imports)]
use game_engine;
use server::protocol::messages::TableState;
use tokio::runtime::Runtime;

#[derive(Default)]
pub struct ConnectionDialog {
    pub open: bool,
    pub server_address: String,
    pub connection_status: ConnectionStatus,
    pub error_message: Option<String>,
    pub connection: Option<Connection>,
    pub table_state: Option<TableState>,
}

#[derive(Default, PartialEq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
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

        let address = self.server_address.clone();
        let rt = Runtime::new().unwrap();
        let result: Result<(Connection, TableState), anyhow::Error> = rt.block_on(async {
            // Connect TCP
            let mut conn = Connection::connect(&address).await?;
            // Perform handshake
            conn.handshake("hupoker-client", "0.1.0").await?;
            // Join default table (table-0) seat 0
            let table_id = game_engine::TableId::new("table-0".to_string());
            let table_state = conn.join_table(table_id, 0).await?;
            Ok((conn, table_state))
        });
        match result {
            Ok((conn, table_state)) => {
                self.connection = Some(conn);
                self.table_state = Some(table_state);
                self.connection_status = ConnectionStatus::Connected;
            }
            Err(e) => {
                self.connection_status = ConnectionStatus::Disconnected;
                self.error_message = Some(format!("Connection failed: {}", e));
            }
        }
    }

    fn disconnect(&mut self) {
        self.connection = None;
        self.table_state = None;
        self.connection_status = ConnectionStatus::Disconnected;
    }
}
