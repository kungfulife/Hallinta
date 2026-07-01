use crate::app::HallintaApp;
use crate::models::{AppSettings, Modal};
use crate::ui::design::Design;
use eframe::egui;

/// Render a helper/description text with a subtle background pill for readability.
fn helper_text(ui: &mut egui::Ui, d: &Design, text: &str) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(6, 3))
        .corner_radius(3.0)
        .fill(d.helper_text_bg)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(d.font_body)
                    .italics()
                    .color(d.helper_text_color),
            );
        });
}

/// Wrap a text-edit field in a highlighted frame when it has focus.
fn focused_text_edit(
    ui: &mut egui::Ui,
    d: &Design,
    value: &mut String,
    desired_width: f32,
) -> egui::Response {
    // We need to render the text edit first to know if it has focus,
    // then paint the highlight behind it. egui draws back-to-front within
    // a frame, so we use a Frame that is conditionally styled.

    // Probe: does this field currently have focus? We use the upcoming ID.
    let id = ui.next_auto_id();
    let has_focus = ui.ctx().memory(|m| m.has_focus(id));

    let frame = if has_focus {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(2, 1))
            .corner_radius(4.0)
            .fill(d.settings_focus_bg)
            .stroke(egui::Stroke::new(1.5, d.settings_focus_border))
    } else {
        egui::Frame::NONE.inner_margin(egui::Margin::symmetric(2, 1))
    };

    let mut resp = None;
    frame.show(ui, |ui| {
        resp = Some(ui.add(egui::TextEdit::singleline(value).desired_width(desired_width)));
    });
    resp.unwrap()
}

fn render_appearance_settings(
    ui: &mut egui::Ui,
    d: &Design,
    settings: &mut AppSettings,
    scale_changed: &mut bool,
) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Appearance").strong().size(d.font_tab));
        ui.add_space(d.sm);

        ui.horizontal(|ui| {
            ui.label("UI Scale:");

            // Internal 1.25 is displayed as 1.0x for user-facing scale.
            let mut display_scale = settings.ui_scale - crate::ui::design::SCALE_OFFSET;
            let display_min =
                crate::ui::design::SCALE_INTERNAL_MIN - crate::ui::design::SCALE_OFFSET;
            let display_max =
                crate::ui::design::SCALE_INTERNAL_MAX - crate::ui::design::SCALE_OFFSET;

            let scale_resp = ui.add(
                egui::Slider::new(&mut display_scale, display_min..=display_max)
                    .step_by(0.05)
                    .text("×"),
            );

            let new_internal = display_scale + crate::ui::design::SCALE_OFFSET;

            // Applying zoom while dragging shifts the slider under the pointer.
            if scale_resp.drag_stopped() || (scale_resp.changed() && !scale_resp.dragged()) {
                settings.ui_scale = new_internal;
                *scale_changed = true;
            }

            if ui.small_button("Reset").clicked() {
                settings.ui_scale = crate::ui::design::SCALE_INTERNAL_DEFAULT;
                *scale_changed = true;
            }
        });
    });
}

pub fn render_settings(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);

    // Snapshot values that trigger side-effects so we can detect changes
    let prev_noita_dir = app.settings.noita_dir.clone();
    let prev_entangled_dir = app.settings.entangled_dir.clone();
    let prev_dark_mode = app.settings.dark_mode;
    let prev_compact_mode = app.settings.compact_mode;
    let prev_ui_scale = app.settings.ui_scale;

    // Track whether any "simple" setting changed (no special side-effects, just save)
    let mut needs_save = false;
    // Track text fields that need validation before saving
    let mut noita_dir_lost_focus = false;
    let mut entangled_dir_lost_focus = false;
    let mut show_noita_warning = false;
    let mut scale_changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Settings");
        ui.add_space(d.md);

        // ── Directory Settings ──────────���──────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Directories").strong().size(d.font_tab));
            ui.add_space(d.sm);

            // Noita save directory
            ui.label("Noita Save Directory:");
            ui.horizontal(|ui| {
                let resp = focused_text_edit(
                    ui,
                    &d,
                    &mut app.settings.noita_dir,
                    ui.available_width() - 250.0,
                );
                if resp.lost_focus() && app.settings.noita_dir != prev_noita_dir {
                    noita_dir_lost_focus = true;
                }
                if ui.button("Browse").clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select Noita Save Directory")
                        .pick_folder()
                {
                    app.settings.noita_dir = folder.to_string_lossy().to_string();
                    noita_dir_lost_focus = true;
                }
                if ui.button("Auto-detect").clicked()
                    && let Ok(path) = crate::core::platform::get_noita_save_path()
                {
                    app.settings.noita_dir = path.to_string_lossy().to_string();
                    noita_dir_lost_focus = true;
                }
                if !app.settings.noita_dir.is_empty() && ui.button("Open").clicked() {
                    let _ = crate::core::platform::open_directory(std::path::Path::new(
                        &app.settings.noita_dir,
                    ));
                }
            });

            ui.add_space(d.sm);

            // Entangled Worlds directory
            ui.label("Entangled Worlds Save Directory:");
            ui.horizontal(|ui| {
                let resp = focused_text_edit(
                    ui,
                    &d,
                    &mut app.settings.entangled_dir,
                    ui.available_width() - 250.0,
                );
                if resp.lost_focus() && app.settings.entangled_dir != prev_entangled_dir {
                    entangled_dir_lost_focus = true;
                }
                if ui.button("Browse").clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select Entangled Worlds Directory")
                        .pick_folder()
                {
                    app.settings.entangled_dir = folder.to_string_lossy().to_string();
                    entangled_dir_lost_focus = true;
                }
                if ui.button("Auto-detect").clicked()
                    && let Ok(path) = crate::core::platform::get_entangled_worlds_save_path()
                {
                    app.settings.entangled_dir = path.to_string_lossy().to_string();
                    entangled_dir_lost_focus = true;
                }
                if !app.settings.entangled_dir.is_empty() && ui.button("Open").clicked() {
                    let _ = crate::core::platform::open_directory(std::path::Path::new(
                        &app.settings.entangled_dir,
                    ));
                }
            });

            // Debug app data directory (settings, logs, backups, presets)
            if cfg!(debug_assertions) {
                ui.add_space(d.sm);
                let dev_dir = crate::core::settings::get_data_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                helper_text(ui, &d, &format!("Debug App Data: {}", dev_dir));
                helper_text(
                    ui,
                    &d,
                    "Debug build: Noita and Entangled Worlds paths above are used directly.",
                );
            }
        });

        ui.add_space(d.md);

        // ── Appearance ───────────��──────────────────────────────────────
        render_appearance_settings(ui, &d, &mut app.settings, &mut scale_changed);

        ui.add_space(d.md);

        // ── Logging Settings ───────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Logging").strong().size(d.font_tab));
            ui.add_space(d.sm);

            ui.horizontal(|ui| {
                ui.label("Max Log Files:");
                if ui
                    .add(
                        egui::DragValue::new(&mut app.settings.log_settings.max_log_files)
                            .range(1..=500),
                    )
                    .changed()
                {
                    needs_save = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Log Level:");
                let prev_level = app.settings.log_settings.log_level.clone();
                egui::ComboBox::from_id_salt("log_level")
                    .selected_text(&app.settings.log_settings.log_level)
                    .show_ui(ui, |ui| {
                        for level in &["DEBUG", "INFO", "WARN", "ERROR"] {
                            ui.selectable_value(
                                &mut app.settings.log_settings.log_level,
                                level.to_string(),
                                *level,
                            );
                        }
                    });
                if app.settings.log_settings.log_level != prev_level {
                    needs_save = true;
                }
            });
            if ui
                .checkbox(
                    &mut app.settings.log_settings.collect_system_info,
                    "Log detailed system info on startup",
                )
                .changed()
            {
                needs_save = true;
            }
        });

        ui.add_space(d.md);

        // ── Save Monitor Settings ──────────────────────────────────────
        ui.group(|ui| {
            ui.label(
                egui::RichText::new("Save Monitor")
                    .strong()
                    .size(d.font_tab),
            );
            ui.add_space(d.sm);

            ui.horizontal(|ui| {
                ui.label("Max snapshots per session:");
                if ui
                    .add(
                        egui::DragValue::new(
                            &mut app.settings.save_monitor_settings.max_snapshots_per_session,
                        )
                        .range(1..=100),
                    )
                    .changed()
                {
                    needs_save = true;
                }
            });
            helper_text(
                ui,
                &d,
                "Oldest snapshots are removed when the limit is reached.",
            );
            if ui
                .checkbox(
                    &mut app.settings.save_monitor_settings.include_save01,
                    "Include save01 in snapshots",
                )
                .changed()
            {
                needs_save = true;
            }
            if ui
                .checkbox(
                    &mut app.settings.save_monitor_settings.include_entangled,
                    "Include Entangled Worlds in snapshots",
                )
                .changed()
            {
                needs_save = true;
            }
            if ui
                .checkbox(
                    &mut app.settings.save_monitor_settings.start_in_monitor_mode,
                    "Start Save Monitor on launch",
                )
                .changed()
            {
                needs_save = true;
            }
        });

        ui.add_space(d.md);

        // ── Workshop Settings ────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Workshop").strong().size(d.font_tab));
            ui.add_space(d.sm);

            ui.label("Steam Path:");
            ui.horizontal(|ui| {
                let steam_prev = app.settings.steam_path.clone();
                let resp = focused_text_edit(
                    ui,
                    &d,
                    &mut app.settings.steam_path,
                    ui.available_width() - 250.0,
                );
                if resp.lost_focus() && app.settings.steam_path != steam_prev {
                    needs_save = true;
                }
                if ui.button("Browse").clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select Steam Directory")
                        .pick_folder()
                {
                    app.settings.steam_path = folder.to_string_lossy().to_string();
                    needs_save = true;
                }
                if ui.button("Auto-detect").clicked()
                    && let Ok(path) = crate::core::workshop::detect_steam_path()
                {
                    app.settings.steam_path = path.to_string_lossy().to_string();
                    needs_save = true;
                }
                if !app.settings.steam_path.is_empty() && ui.button("Open").clicked() {
                    let _ = crate::core::platform::open_directory(std::path::Path::new(
                        &app.settings.steam_path,
                    ));
                }
            });
        });

        ui.add_space(d.lg);

        // ── Action Buttons ───────────────────────��─────────────────────
        ui.horizontal(|ui| {
            if ui.button("Reset to Defaults").clicked() {
                let mut defaults = default_settings();
                // Auto-detect directories so the user sees populated paths
                if let Ok(path) = crate::core::platform::get_noita_save_path() {
                    defaults.noita_dir = path.to_string_lossy().to_string();
                }
                if let Ok(path) = crate::core::platform::get_entangled_worlds_save_path() {
                    defaults.entangled_dir = path.to_string_lossy().to_string();
                }
                if let Ok(path) = crate::core::workshop::detect_steam_path() {
                    defaults.steam_path = path.to_string_lossy().to_string();
                }
                app.settings = defaults;
                // All side-effects (theme, compact, scale) are picked up below
                // via the prev_ snapshot comparisons and scale_changed flag
                if app.settings.ui_scale != prev_ui_scale {
                    scale_changed = true;
                }
                noita_dir_lost_focus = true;
                needs_save = true;
            }
        });

        ui.add_space(d.lg);

        // ── Info Panels ──────────────��────────────────────────���────────
        ui.horizontal(|ui| {
            if ui.button("System Information").clicked() {
                app.active_modal = Some(Modal::SystemInfo);
            }
            if ui.button("Open Source Libraries").clicked() {
                app.active_modal = Some(Modal::OpenSourceLibraries);
            }
            if ui.button("Open Settings Folder").clicked()
                && let Ok(dir) = crate::core::settings::get_data_dir()
            {
                let _ = crate::core::platform::open_directory(&dir);
            }
        });
    });

    // ── Handle side-effects after the scroll area ──────────────────────

    // Noita directory changed (via blur, Browse, or Auto-detect)
    if noita_dir_lost_focus {
        // Validate: check for mod_config.xml
        if !app.settings.noita_dir.is_empty() {
            let noita_path = std::path::PathBuf::from(&app.settings.noita_dir);
            if !noita_path.join("mod_config.xml").exists() {
                show_noita_warning = true;
            }
        }
        if !show_noita_warning {
            app.on_noita_dir_changed();
        }
    }

    if show_noita_warning {
        app.active_modal = Some(Modal::Info {
            title: "Warning".to_string(),
            message: "The selected Noita directory does not contain mod_config.xml.".to_string(),
        });
        // Still save — user might know what they're doing
        app.save_current_settings();
    }

    // Entangled dir changed
    if entangled_dir_lost_focus {
        app.save_current_settings();
    }

    // Dark mode changed indirectly (for example, Reset to Defaults)
    if app.settings.dark_mode != prev_dark_mode {
        app.on_dark_mode_changed(ui.ctx());
    }

    // Compact mode changed indirectly (for example, Reset to Defaults)
    if app.settings.compact_mode != prev_compact_mode {
        app.on_compact_mode_changed(ui.ctx());
    }

    // UI scale changed — resize window proportionally
    if scale_changed {
        app.on_ui_scale_changed(ui.ctx(), prev_ui_scale);
    }

    // Generic save for anything else that changed
    if needs_save {
        app.save_current_settings();
    }
}

fn default_settings() -> AppSettings {
    AppSettings {
        noita_dir: String::new(),
        entangled_dir: String::new(),
        dark_mode: false,
        selected_preset: "Default".to_string(),
        version: crate::core::platform::get_version(),
        log_settings: Default::default(),
        backup_settings: Default::default(),
        save_monitor_settings: Default::default(),
        steam_path: String::new(),
        compact_mode: false,
        ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
        last_filter_mode: String::new(),
        last_sort_mode: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered_appearance_labels(settings: &mut AppSettings) -> Vec<String> {
        let ctx = egui::Context::default();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let d = Design::new(ui.ctx(), settings);
                let mut scale_changed = false;
                render_appearance_settings(ui, &d, settings, &mut scale_changed);
            });
        });

        output
            .shapes
            .iter()
            .flat_map(|shape| text_from_shape(&shape.shape))
            .collect()
    }

    fn text_from_shape(shape: &egui::epaint::Shape) -> Vec<String> {
        match shape {
            egui::epaint::Shape::Text(text) => vec![text.galley.text().to_string()],
            egui::epaint::Shape::Vec(shapes) => shapes.iter().flat_map(text_from_shape).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn appearance_settings_do_not_duplicate_top_bar_toggles() {
        let mut settings = default_settings();

        let labels = rendered_appearance_labels(&mut settings);

        assert!(
            !labels.iter().any(|label| label == "Dark Mode"),
            "dark mode is already available in the top bar"
        );
        assert!(
            !labels.iter().any(|label| label == "Compact Mode"),
            "compact mode is already available in the top bar"
        );
        assert!(
            labels.iter().any(|label| label == "UI Scale:"),
            "appearance settings should still expose UI scale"
        );
    }
}
