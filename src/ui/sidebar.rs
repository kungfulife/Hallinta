use crate::app::HallintaApp;
use crate::models::{ConfirmAction, Modal};
use eframe::egui;

pub fn render_sidebar(app: &mut HallintaApp, ctx: &egui::Context) {
    egui::SidePanel::right("sidebar_panel")
        .resizable(false)
        .default_width(160.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Actions").heading().strong());
            ui.add_space(8.0);

            let is_locked = app.save_monitor.is_running();
            let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

            // ── Mod Actions ────────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Mods").strong());
                ui.add_space(4.0);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.button("Import Mod List").clicked() {
                        app.import_mod_list();
                    }
                    if ui.button("Export Mod List").clicked() {
                        app.export_mod_list();
                    }
                    if ui.button("Open mod_config.xml").clicked() {
                        app.open_mod_config_file();
                    }
                });
            });

            ui.add_space(8.0);

            // ── Preset Actions ─────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Presets").strong());
                ui.add_space(4.0);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.button("Export Presets").clicked() {
                        app.start_export_presets();
                    }
                    if ui.button("Import Presets").clicked() {
                        app.import_presets();
                    }
                });
            });

            ui.add_space(8.0);

            // ── Backup & Restore ───────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Backup").strong());
                ui.add_space(4.0);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui.button("Create Backup").clicked() {
                        app.start_backup_modal();
                    }
                    if ui.button("Restore Backup").clicked() {
                        app.start_restore_modal();
                    }
                    if ui.button("Manage Backups").clicked() {
                        app.load_backup_list_async();
                        app.active_modal = Some(Modal::BackupManager);
                    }
                });
            });

            ui.add_space(8.0);

            // ── Save Monitor ───────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Save Monitor").strong());
                ui.add_space(4.0);

                if app.save_monitor.is_running() {
                    ui.colored_label(
                        egui::Color32::from_rgb(50, 200, 50),
                        egui::RichText::new("Running").strong(),
                    );
                    ui.label(format!(
                        "Snapshots: {}",
                        app.save_monitor.snapshot_count
                    ));
                    if ui.button("Stop Monitor").clicked() {
                        app.stop_save_monitor();
                    }
                } else {
                    ui.add_enabled_ui(!backup_busy, |ui| {
                        if ui.button("Start Monitor").clicked() {
                            app.start_save_monitor();
                        }
                    });
                }

                ui.add_space(4.0);

                // View snapshots for current preset
                if ui.button("View Snapshots").clicked() {
                    let preset = app.selected_preset.clone();
                    app.load_snapshot_list_async(preset.clone());
                    app.active_modal = Some(Modal::SnapshotManager {
                        preset_name: preset,
                    });
                }

                // Clear all monitor data
                if ui
                    .add_enabled(
                        !is_locked,
                        egui::Button::new("Clear All Snapshots"),
                    )
                    .clicked()
                {
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
}
