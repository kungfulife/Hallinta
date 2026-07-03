use crate::app::HallintaApp;
use crate::models::{FilterMode, SortMode, View};
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
                let ml_text = egui::RichText::new("Mod List")
                    .font(tab_font)
                    .strong()
                    .color(ml_color);
                if ui
                    .add(egui::Button::new(ml_text).fill(ml_fill).corner_radius(4.0))
                    .clicked()
                {
                    app.active_view = View::ModList;
                }

                ui.separator();

                // Search box (only in mod list)
                if app.active_view == View::ModList {
                    ui.label(egui::RichText::new("Search:").strong());
                    let search_id = egui::Id::new("hallinta_search");
                    let search_resp = ui.add(
                        egui::TextEdit::singleline(&mut app.search_query)
                            .id(search_id)
                            .desired_width(d.search_w)
                            .hint_text("Name or workshop ID..."),
                    );
                    search_resp.on_hover_text("Search by mod name or workshop ID (Ctrl+F)");
                    if app.focus_search_requested {
                        app.focus_search_requested = false;
                        ui.ctx().memory_mut(|m| m.request_focus(search_id));
                    }

                    // Quick reload mod_config.xml
                    if ui.button("⟳")
                        .on_hover_text("Reload mod_config.xml from disk (F5)")
                        .clicked()
                    {
                        app.reload_mods_explicit();
                    }
                }

                // Filter mode (mod list only)
                if app.active_view == View::ModList {
                    ui.separator();
                    for mode in [FilterMode::All, FilterMode::Enabled, FilterMode::Disabled] {
                        let selected = app.filter_mode == mode;
                        let fill = if selected { d.filter_bg_selected } else { d.filter_bg };
                        let color = if selected { d.tab_text_selected } else { d.tab_text };
                        let text = egui::RichText::new(mode.label()).strong().color(color);
                        if ui
                            .add(egui::Button::new(text).fill(fill).corner_radius(4.0))
                            .clicked()
                        {
                            app.set_filter_mode(mode);
                        }
                    }

                    // Sort dropdown
                    ui.separator();
                    let sort_label = format!("Sort: {}", app.sort_mode.label());
                    egui::ComboBox::from_id_salt("hallinta_sort")
                        .selected_text(sort_label)
                        .show_ui(ui, |ui| {
                            for mode in [
                                SortMode::Default,
                                SortMode::NameAsc,
                                SortMode::NameDesc,
                                SortMode::EnabledFirst,
                                SortMode::DisabledFirst,
                            ] {
                                if ui
                                    .selectable_label(app.sort_mode == mode, mode.label())
                                    .clicked()
                                {
                                    app.set_sort_mode(mode);
                                }
                            }
                        })
                        .response
                        .on_hover_text("Sort the visible mod list (does not change file order unless modified)");
                }
            }

            // Right-aligned controls — always visible
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Settings button
                if ui.button("Settings")
                    .on_hover_text("Open / close Settings (Ctrl+,)")
                    .clicked()
                {
                    if app.active_view == View::Settings {
                        app.active_view = View::ModList;
                    } else {
                        app.active_view = View::Settings;
                    }
                }

                // Compact mode toggle
                let compact_label = if app.compact_mode { "Normal" } else { "Compact" };
                if ui.button(compact_label)
                    .on_hover_text("Toggle compact / normal window")
                    .clicked()
                {
                    app.toggle_compact_mode(ctx);
                }

                // Quick dark-mode toggle
                let theme_icon = if app.settings.dark_mode { "☀" } else { "🌙" };
                if ui.button(theme_icon)
                    .on_hover_text("Toggle dark / light theme")
                    .clicked()
                {
                    app.settings.dark_mode = !app.settings.dark_mode;
                    app.on_dark_mode_changed(ctx);
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
