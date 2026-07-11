use crate::app::HallintaApp;
use crate::models::{ConfirmAction, Modal};
use eframe::egui;

pub fn render_sidebar(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
    egui::Panel::right("sidebar_panel")
        .resizable(false)
        .default_size(d.sidebar_w)
        .min_size(d.sidebar_w)
        .max_size(d.sidebar_w)
        .show(ui, |ui| {
            ui.add_space(d.md);
            let btn_width = (ui.available_width() - d.md * 2.0).max(1.0);
            ui.label(egui::RichText::new("Actions").size(d.font_heading).strong());
            ui.add_space(d.md);

            let is_locked = app.save_monitor.is_running();
            let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

            // ── Mod Actions ────────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Mods").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Import Mod List"))
                        .on_hover_text("Load a saved mod list (.json) and apply it")
                        .clicked()
                    {
                        app.import_mod_list();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Export Mod List"))
                        .on_hover_text("Save the currently enabled mods to a .json file")
                        .clicked()
                    {
                        app.export_mod_list();
                    }
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Open mod_config.xml"))
                        .on_hover_text("Open the live mod_config.xml in your default editor")
                        .clicked()
                    {
                        app.open_mod_config_file();
                    }
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Reload from disk"))
                        .on_hover_text("Re-read mod_config.xml (F5)")
                        .clicked()
                    {
                        app.reload_mods_explicit();
                    }
                });
            });

            ui.add_space(d.md);

            // ── Preset Actions ─────────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Presets").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(app.can_export_presets(), |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Export Presets"))
                        .on_hover_text("Bundle one or more presets into a shareable .json")
                        .clicked()
                    {
                        app.start_export_presets();
                    }
                });
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Import Presets"))
                        .on_hover_text("Load presets from a .json file")
                        .clicked()
                    {
                        app.import_presets();
                    }
                });
            });

            ui.add_space(d.md);

            // ── Backup & Restore ───────────────────────────────────
            ui.group(|ui| {
                ui.label(egui::RichText::new("Backup").strong());
                ui.add_space(d.sm);

                ui.add_enabled_ui(app.can_start_manual_backup(), |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Create Manual Backup"))
                        .on_hover_text(
                            "Bundle save00, presets, and optional save dirs into a manual .zip (Ctrl+B)",
                        )
                        .clicked()
                    {
                        app.start_backup_modal();
                    }
                });
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Restore Latest"))
                        .on_hover_text("Restore the most recent backup with default options")
                        .clicked()
                    {
                        app.restore_last_backup();
                    }
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Restore Backup"))
                        .on_hover_text("Pick from the list of saved backups")
                        .clicked()
                    {
                        app.start_restore_modal();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Manage Backups"))
                        .on_hover_text("Browse, inspect, and delete backups")
                        .clicked()
                    {
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
                    ui.colored_label(d.status_ok, egui::RichText::new("Running").strong());
                    if let Some(ref session) = app.save_monitor.current_session {
                        ui.label(format!("Session: {}", session.name));
                    }

                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Stop Monitor"))
                        .on_hover_text("Stop the monitor - session is saved and can be resumed later")
                        .clicked()
                    {
                        app.stop_save_monitor();
                    }
                } else {
                    ui.add_enabled_ui(!backup_busy, |ui| {
                        if ui
                            .add_sized([btn_width, 0.0], egui::Button::new("Start Monitor"))
                            .on_hover_text("Start auto-snapshotting saves while you play")
                            .clicked()
                        {
                            app.start_save_monitor();
                        }
                    });
                }

                ui.add_space(d.sm);

                if ui
                    .add_sized([btn_width, 0.0], egui::Button::new("View Sessions"))
                    .on_hover_text("Browse and restore from past monitor sessions")
                    .clicked()
                {
                    app.load_sessions_async();
                }

                ui.add_enabled_ui(!is_locked, |ui| {
                    if ui
                        .add_sized([btn_width, 0.0], egui::Button::new("Clear All Data"))
                        .on_hover_text(
                            "DESTRUCTIVE: delete every session and snapshot for every preset",
                        )
                        .clicked()
                    {
                        app.active_modal = Some(Modal::Confirm {
                            message: "Delete ALL monitor sessions and snapshots for ALL presets?"
                                .to_string(),
                            confirm_text: "Delete All".to_string(),
                            cancel_text: "Cancel".to_string(),
                            action: ConfirmAction::ClearMonitorData,
                            cancel_action: None,
                            dismissable: false,
                        });
                    }
                });
            });
        });
}
