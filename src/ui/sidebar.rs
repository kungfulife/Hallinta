use crate::app::HallintaApp;
use crate::models::{ConfirmAction, Modal};
use eframe::egui;

pub fn render_sidebar(app: &mut HallintaApp, ctx: &egui::Context) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    egui::SidePanel::right("sidebar_panel")
        .resizable(false)
        .default_width(d.sidebar_w)
        .show(ctx, |ui| {
            ui.add_space(d.md);
            let btn_width = d.sidebar_w - d.md * 2.0;
            ui.set_min_width(d.sidebar_w);
            ui.label(egui::RichText::new("Actions").size(d.font_heading).strong());
            ui.add_space(d.md);

            let is_locked = app.save_monitor.is_running();
            let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

            // ── Mod Actions ────────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Mods").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Import Mod List")).clicked() {
                        app.import_mod_list();
                    }
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Export Mod List")).clicked() {
                        app.export_mod_list();
                    }
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Open mod_config.xml")).clicked() {
                        app.open_mod_config_file();
                    }
                });
            });

            ui.add_space(d.md);

            // ── Preset Actions ─────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Presets").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Export Presets")).clicked() {
                        app.start_export_presets();
                    }
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Import Presets")).clicked() {
                        app.import_presets();
                    }
                });
            });

            ui.add_space(d.md);

            // ── Backup & Restore ───────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Backup").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Create Backup")).clicked() {
                        app.start_backup_modal();
                    }
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Restore Backup")).clicked() {
                        app.start_restore_modal();
                    }
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Manage Backups")).clicked() {
                        app.load_backup_list_async();
                        app.active_modal = Some(Modal::BackupManager);
                    }
                });
            });

            ui.add_space(d.md);

            // ── Save Monitor ───────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Save Monitor").strong());
                ui.add_space(d.sm);

                if app.save_monitor.is_running() {
                    ui.colored_label(
                        d.status_ok,
                        egui::RichText::new("Running").strong(),
                    );
                    ui.label(format!(
                        "Snapshots: {}",
                        app.save_monitor.snapshot_count
                    ));
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Stop Monitor")).clicked() {
                        app.stop_save_monitor();
                    }
                } else {
                    ui.add_enabled_ui(!backup_busy, |ui| {
                        if ui.add_sized([btn_width, 0.0], egui::Button::new("Start Monitor")).clicked() {
                            app.start_save_monitor();
                        }
                    });
                }

                ui.add_space(d.sm);

                // View snapshots for current preset
                if ui.add_sized([btn_width, 0.0], egui::Button::new("View Snapshots")).clicked() {
                    let preset = app.selected_preset.clone();
                    app.load_snapshot_list_async(preset.clone());
                    app.active_modal = Some(Modal::SnapshotManager {
                        preset_name: preset,
                    });
                }

                // Clear all monitor data
                ui.add_enabled_ui(!is_locked, |ui| {
                    if ui.add_sized([btn_width, 0.0], egui::Button::new("Clear All Snapshots")).clicked() {
                        app.active_modal = Some(Modal::Confirm {
                            message: "Delete ALL monitor snapshots for ALL presets?".to_string(),
                            confirm_text: "Delete All".to_string(),
                            cancel_text: "Cancel".to_string(),
                            action: ConfirmAction::ClearMonitorData,
                            cancel_action: None,
                        });
                    }
                });
            });
        });
}
