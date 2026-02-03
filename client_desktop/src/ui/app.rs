use crate::ui::connection_dialog::{ConnectionDialog, ConnectionStatus};
use crate::ui::TableView;

pub struct HupokerApp {
    connection_dialog: ConnectionDialog,
    table_view: Option<TableView>,
}

impl HupokerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let dialog = ConnectionDialog { open: true, ..Default::default() };
        Self { connection_dialog: dialog, table_view: None }
    }
}

impl eframe::App for HupokerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // If connected and have table state, show table view
        if let ConnectionStatus::Connected = self.connection_dialog.connection_status {
            if let Some(table_state) = &self.connection_dialog.table_state {
                // Ensure table view exists
                if self.table_view.is_none() {
                    let table_id = table_state.table_id.clone();
                    // TODO: Allow seat selection in UI. Currently defaults to seat 0.
                    let player_seat = 0;
                    let connection = self.connection_dialog.connection.take();
                    self.table_view = Some(TableView::new(player_seat, table_id, connection));
                } else {
                    // Table view already exists (reconnection case)
                    // Update its connection with the new one from dialog
                    if let Some(conn) = self.connection_dialog.connection.take() {
                        if let Some(table_view) = &mut self.table_view {
                            table_view.set_connection(conn);
                        }
                    }
                }
                // Update table view with hand state if any (from reconnection)
                if let Some(hand_state) = self.connection_dialog.hand_state.take() {
                    if let Some(table_view) = &mut self.table_view {
                        table_view.update_from_hand_state(&hand_state);
                    }
                }
                // Close connection dialog
                self.connection_dialog.open = false;
                // Show table view
                if let Some(table_view) = &mut self.table_view {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        table_view.show(ui);
                    });
                }
                return;
            }
        }

        // Otherwise show welcome panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hupoker client");
            ui.separator();
            ui.label("Welcome to heads-up NLHE poker.");
            ui.label("Connect to a server to start playing.");
        });

        self.connection_dialog.show(ctx);
    }
}
