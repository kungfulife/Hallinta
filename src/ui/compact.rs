use crate::app::HallintaApp;
use crate::models::Modal;
use crate::ui::design::Design;
use eframe::egui;

pub fn render_compact(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = Design::new(ui.ctx(), &app.settings);

    // Use vertical_centered with a max width so everything aligns nicely
    let available = ui.available_size();
    let top_pad = (available.y * 0.12).max(d.lg);

    ui.add_space(top_pad);

    ui.vertical_centered(|ui| {
        ui.set_max_width(260.0);

        // ── Preset selector ──────────────────────────────────────────────────
        ui.label(
            egui::RichText::new("Preset")
                .size(d.font_small)
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(d.xs);

        let is_locked = app.save_monitor.is_running();
        let prev_selected = app.selected_preset.clone();

        let mut preset_names: Vec<String> = app.presets.keys().cloned().collect();
        preset_names.sort_by(|a, b| {
            if a == "Default" {
                std::cmp::Ordering::Less
            } else if b == "Default" {
                std::cmp::Ordering::Greater
            } else {
                a.to_lowercase().cmp(&b.to_lowercase())
            }
        });

        ui.add_enabled_ui(!is_locked, |ui| {
            egui::ComboBox::from_id_salt("compact_preset_selector")
                .selected_text(&app.selected_preset)
                .width(250.0)
                .show_ui(ui, |ui| {
                    for name in &preset_names {
                        if ui
                            .selectable_label(*name == app.selected_preset, name)
                            .clicked()
                        {
                            app.selected_preset = name.clone();
                        }
                    }
                });
        });

        if app.selected_preset != prev_selected {
            app.switch_preset();
        }

        ui.add_space(d.sm);

        // Mod count summary
        let total = app.current_mods.len();
        let enabled_count = app.current_mods.iter().filter(|m| m.enabled).count();
        ui.label(
            egui::RichText::new(format!("{} / {} mods enabled", enabled_count, total))
                .size(d.font_body)
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(d.lg);
        ui.separator();
        ui.add_space(d.lg);

        let btn_w = 240.0;
        let btn_h = 28.0;
        let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

        // ── Monitor button ───────────────────────────────────────────────────
        if is_locked {
            ui.colored_label(
                d.status_ok,
                egui::RichText::new("● MONITOR ACTIVE")
                    .size(d.font_body)
                    .strong(),
            );
            ui.add_space(d.sm);
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Stop Monitor").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.stop_save_monitor();
            }
        } else {
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Start Monitor").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.start_save_monitor();
            }
        }

        ui.add_space(d.sm);

        // ── Secondary actions ────────────────────────────────────────────────
        ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Create Backup").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.start_backup_modal();
            }
        });

        ui.add_space(d.xs);

        if ui
            .add_sized(
                [btn_w, btn_h],
                egui::Button::new(
                    egui::RichText::new("View Snapshots").size(d.font_body),
                ),
            )
            .clicked()
        {
            let preset = app.selected_preset.clone();
            if let Some(ref session) = app.save_monitor.current_session {
                app.load_session_snapshots_async(preset.clone(), session.id.clone());
            }
            app.active_modal = Some(Modal::SnapshotManager {
                preset_name: preset,
            });
        }

        // ── Live snapshot count when running ─────────────────────────────────
        if is_locked {
            ui.add_space(d.md);
            ui.label(
                egui::RichText::new(format!(
                    "Snapshots taken: {}",
                    app.save_monitor.snapshot_count
                ))
                .size(d.font_small)
                .color(ui.visuals().weak_text_color()),
            );
        }
    });
}
