mod connection;
mod ui;
use ui::HupokerApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("hupoker", native_options, Box::new(|cc| Box::new(HupokerApp::new(cc))))
}
