use crate::app::HallintaApp;
use eframe::egui;

pub fn render_compact(_app: &mut HallintaApp, ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.label("Compact mode");
    });
}
