use crate::app::HallintaApp;
use crate::models::{FilterMode, View};
use eframe::egui;

pub fn render_header(app: &mut HallintaApp, ctx: &egui::Context) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    egui::TopBottomPanel::top("header_panel").show(ctx, |ui| {
        ui.add_space(d.sm);

        // Row 1: Tab buttons + search + filter
        ui.horizontal(|ui| {
            // Left-side controls: hidden in compact mode
            if !app.compact_mode {
                let tab_font = d.font(d.font_tab);

                // Tab: Mod List
                let ml_selected = app.active_view == View::ModList;
                let ml_fill = if ml_selected { d.tab_bg_selected } else { d.tab_bg };
                let ml_color = if ml_selected { d.tab_text_selected } else { d.tab_text };
                let ml_text = egui::RichText::new("Mod List").font(tab_font).strong().color(ml_color);
                if ui.add(egui::Button::new(ml_text).fill(ml_fill).corner_radius(4.0)).clicked()
                    && !app.save_monitor.is_running() {
                        app.active_view = View::ModList;
                    }

                ui.separator();

                // Search box (only in mod list)
                if app.active_view == View::ModList {
                    ui.label(egui::RichText::new("Search:").strong());
                    ui.add(
                        egui::TextEdit::singleline(&mut app.search_query)
                            .desired_width(d.search_w)
                            .hint_text("Filter..."),
                    );
                }

                // Filter mode (mod list only)
                if app.active_view == View::ModList && !app.save_monitor.is_running() {
                    ui.separator();
                    for mode in [FilterMode::All, FilterMode::Enabled, FilterMode::Disabled] {
                        let selected = app.filter_mode == mode;
                        let fill = if selected { d.filter_bg_selected } else { d.filter_bg };
                        let color = if selected { d.tab_text_selected } else { d.tab_text };
                        let text = egui::RichText::new(mode.label()).strong().color(color);
                        if ui.add(egui::Button::new(text).fill(fill).corner_radius(4.0)).clicked() {
                            app.filter_mode = mode;
                        }
                    }
                }
            }

            // Right-aligned controls — always visible
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Settings button
                if ui.button("Settings").clicked() {
                    if app.active_view == View::Settings {
                        app.active_view = View::ModList;
                    } else {
                        app.active_view = View::Settings;
                    }
                }

                // Compact mode toggle
                let compact_label = if app.compact_mode { "Normal" } else { "Compact" };
                if ui.button(compact_label).clicked() {
                    app.toggle_compact_mode(ctx);
                }

                // Monitor indicator
                if app.save_monitor.is_running() {
                    ui.colored_label(
                        d.status_ok,
                        egui::RichText::new("MONITOR ACTIVE").strong(),
                    );
                }
            });
        });

        // Row 2: Preset bar (only in mod list view, not in compact mode, not when monitor running)
        if app.active_view == View::ModList && !app.compact_mode && !app.save_monitor.is_running() {
            crate::ui::preset_bar::render_preset_bar(app, ui);
        }

        ui.add_space(d.xs);
    });
}
