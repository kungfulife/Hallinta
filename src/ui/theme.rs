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
    // Purple accent — Noita magical energy
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(110, 60, 190);
    visuals.selection.bg_fill = egui::Color32::from_rgb(110, 60, 190);
    // Slightly brighter base text for readability
    visuals.override_text_color = Some(egui::Color32::from_rgb(220, 218, 225));
    visuals
}

fn light_visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::light();
    // Muted purple accent — readable on light backgrounds
    let accent = egui::Color32::from_rgb(100, 65, 160);
    visuals.widgets.active.bg_fill = accent;
    visuals.selection.bg_fill = egui::Color32::from_rgba_premultiplied(100, 65, 160, 60);
    // Selected tab text: dark purple instead of blinding fill
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 35, 130));
    visuals
}
