use crate::ui::connection_dialog::{ConnectionDialog, ConnectionStatus};
use crate::ui::TableView;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct HupokerApp {
    connection_dialog: ConnectionDialog,
    table_view: Option<TableView>,
    runtime: Arc<Runtime>,
}

impl HupokerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
        let dialog = ConnectionDialog {
            open: true,
            runtime: Some(Arc::clone(&runtime)),
            ..Default::default()
        };
        Self { connection_dialog: dialog, table_view: None, runtime }
    }
}

impl eframe::App for HupokerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let ConnectionStatus::Connected = self.connection_dialog.connection_status {
            if let Some(table_state) = &self.connection_dialog.table_state {
                if self.table_view.is_none() {
                    let table_id = table_state.table_id.clone();
                    let player_seat = 0;
                    let connection = self.connection_dialog.connection.take();
                    self.table_view = Some(TableView::new(
                        player_seat,
                        table_id,
                        connection,
                        Arc::clone(&self.runtime),
                    ));
                } else if let Some(conn) = self.connection_dialog.connection.take() {
                    if let Some(table_view) = &mut self.table_view {
                        table_view.set_connection(conn);
                    }
                }
                if let Some(hand_state) = self.connection_dialog.hand_state.take() {
                    if let Some(table_view) = &mut self.table_view {
                        table_view.update_from_hand_state(&hand_state);
                    }
                }
                self.connection_dialog.open = false;
                if let Some(table_view) = &mut self.table_view {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        table_view.show(ui);
                    });
                }
                return;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hupoker client");
            ui.separator();
            ui.label("Welcome to heads-up NLHE poker.");
            ui.label("Connect to a server to start playing.");
        });

        self.connection_dialog.show(ctx);
    }
}
