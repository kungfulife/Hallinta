use crate::app::HallintaApp;
use crate::models::{FilterMode, View};
use eframe::egui;

pub fn render_header(app: &mut HallintaApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
        ui.add_space(4.0);

        // Row 1: Tab buttons + search + filter
        ui.horizontal(|ui| {
            // View tabs (bold, slightly larger)
            let tab_font = egui::FontId::proportional(15.0);

            let mod_list_text = egui::RichText::new("Mod List").font(tab_font.clone());
            if ui
                .selectable_label(app.active_view == View::ModList, mod_list_text)
                .clicked()
            {
                if !app.save_monitor.is_running() {
                    app.active_view = View::ModList;
                }
            }

            let vault_text = egui::RichText::new("Modpacks").font(tab_font);
            if ui
                .selectable_label(app.active_view == View::PresetVault, vault_text)
                .clicked()
            {
                if !app.save_monitor.is_running() {
                    app.active_view = View::PresetVault;
                }
            }

            ui.separator();

            // Search box (only in mod list or modpacks)
            if app.active_view == View::ModList || app.active_view == View::PresetVault {
                ui.label(egui::RichText::new("Search:").strong());
                ui.add(
                    egui::TextEdit::singleline(&mut app.search_query)
                        .desired_width(150.0)
                        .hint_text("Filter..."),
                );
            }

            // Filter mode (mod list only)
            if app.active_view == View::ModList && !app.save_monitor.is_running() {
                ui.separator();
                for mode in [FilterMode::All, FilterMode::Enabled, FilterMode::Disabled] {
                    if ui
                        .selectable_label(app.filter_mode == mode, mode.label())
                        .clicked()
                    {
                        app.filter_mode = mode;
                    }
                }
            }

            // Right-aligned controls
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Settings button
                if ui.button("Settings").clicked() {
                    if app.active_view == View::Settings {
                        app.active_view = View::ModList;
                    } else {
                        app.active_view = View::Settings;
                        app.pending_settings = Some(app.settings.clone());
                    }
                }

                // Compact mode toggle
                let compact_label = if app.compact_mode {
                    "Normal"
                } else {
                    "Compact"
                };
                if ui.button(compact_label).clicked() {
                    app.toggle_compact_mode(ctx);
                }

                // Monitor indicator
                if app.save_monitor.is_running() {
                    ui.colored_label(
                        egui::Color32::from_rgb(50, 200, 50),
                        egui::RichText::new("MONITOR ACTIVE").strong(),
                    );
                }
            });
        });

        // Row 2: Preset bar (only in mod list view, not in compact mode, not when monitor running)
        if app.active_view == View::ModList && !app.compact_mode && !app.save_monitor.is_running() {
            crate::ui::preset_bar::render_preset_bar(app, ui);
        }

        ui.add_space(2.0);
    });
}
