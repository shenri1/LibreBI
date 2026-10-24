use eframe::egui;

struct LibreBiApp;

impl eframe::App for LibreBiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("LibreBI");
        ui.label("Em construcao");
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "LibreBI",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(LibreBiApp))),
    )
}
