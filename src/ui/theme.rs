use eframe::egui;

pub fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    // Set both the theme preference (so egui doesn't override with OS theme each
    // frame) and the custom visuals for each mode.
    if dark_mode {
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals_of(egui::Theme::Dark, dark_visuals());
    } else {
        ctx.set_theme(egui::ThemePreference::Light);
        ctx.set_visuals_of(egui::Theme::Light, light_visuals());
    }
}

fn dark_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 120, 200);
    visuals.selection.bg_fill = egui::Color32::from_rgb(60, 120, 200);
    visuals
}

fn light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(40, 100, 180);
    visuals.selection.bg_fill = egui::Color32::from_rgb(40, 100, 180);
    visuals
}
