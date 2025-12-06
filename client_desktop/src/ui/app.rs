use crate::ui::connection_dialog::ConnectionDialog;

pub struct HupokerApp {
    connection_dialog: ConnectionDialog,
}

impl HupokerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut dialog = ConnectionDialog::default();
        dialog.open = true;
        Self { connection_dialog: dialog }
    }
}

impl eframe::App for HupokerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("hupoker client");
            ui.separator();
            ui.label("Welcome to heads-up NLHE poker.");
            ui.label("Connect to a server to start playing.");
        });

        self.connection_dialog.show(ctx);
    }
}
