use crate::app::HallintaApp;
use crate::models::{AppSettings, Modal};
use eframe::egui;

enum SettingsAction {
    None,
    Save,
    Cancel,
    Reset,
    ShowWarning,
}

pub fn render_settings(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let mut settings = match app.pending_settings.take() {
        Some(s) => s,
        None => app.settings.clone(),
    };

    let mut action = SettingsAction::None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Settings");
        ui.add_space(8.0);

        // ── Directory Settings ─────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Directories").strong().size(14.0));
            ui.add_space(4.0);

            // Noita save directory
            ui.label("Noita Save Directory:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut settings.noita_dir)
                        .desired_width(ui.available_width() - 180.0),
                );
                if ui.button("Browse").clicked() {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select Noita Save Directory")
                        .pick_folder()
                    {
                        settings.noita_dir = folder.to_string_lossy().to_string();
                    }
                }
                if ui.button("Auto-detect").clicked() {
                    if let Ok(path) = crate::core::platform::get_noita_save_path() {
                        settings.noita_dir = path.to_string_lossy().to_string();
                    }
                }
            });

            ui.add_space(4.0);

            // Entangled Worlds directory
            ui.label("Entangled Worlds Save Directory:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut settings.entangled_dir)
                        .desired_width(ui.available_width() - 180.0),
                );
                if ui.button("Browse").clicked() {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select Entangled Worlds Directory")
                        .pick_folder()
                    {
                        settings.entangled_dir = folder.to_string_lossy().to_string();
                    }
                }
                if ui.button("Auto-detect").clicked() {
                    if let Ok(path) = crate::core::platform::get_entangled_worlds_save_path() {
                        settings.entangled_dir = path.to_string_lossy().to_string();
                    }
                }
            });

            // Dev data directory (debug only)
            if cfg!(debug_assertions) {
                ui.add_space(4.0);
                let dev_dir = crate::core::settings::get_data_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                ui.label(
                    egui::RichText::new(format!("Dev Data: {}", dev_dir))
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );
            }
        });

        ui.add_space(8.0);

        // ── Appearance ──────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Appearance").strong().size(14.0));
            ui.add_space(4.0);

            ui.checkbox(&mut settings.dark_mode, "Dark Mode");
            ui.checkbox(&mut settings.compact_mode, "Compact Mode");
        });

        ui.add_space(8.0);

        // ── Logging Settings ───────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Logging").strong().size(14.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Max Log Files:");
                ui.add(egui::DragValue::new(&mut settings.log_settings.max_log_files).range(1..=500));
            });
            ui.horizontal(|ui| {
                ui.label("Max Log Size (MB):");
                ui.add(egui::DragValue::new(&mut settings.log_settings.max_log_size_mb).range(1..=100));
            });
            ui.horizontal(|ui| {
                ui.label("Log Level:");
                egui::ComboBox::from_id_salt("log_level")
                    .selected_text(&settings.log_settings.log_level)
                    .show_ui(ui, |ui| {
                        for level in &["DEBUG", "INFO", "WARN", "ERROR"] {
                            ui.selectable_value(
                                &mut settings.log_settings.log_level,
                                level.to_string(),
                                *level,
                            );
                        }
                    });
            });
            ui.checkbox(
                &mut settings.log_settings.collect_system_info,
                "Log detailed system info on startup",
            );
        });

        ui.add_space(8.0);

        // ── Backup Settings ────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Backup").strong().size(14.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Auto-delete backups older than (days, 0=never):");
                ui.add(
                    egui::DragValue::new(&mut settings.backup_settings.auto_delete_days)
                        .range(0..=365),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Auto-backup interval (minutes, 0=disabled):");
                ui.add(
                    egui::DragValue::new(&mut settings.backup_settings.backup_interval_minutes)
                        .range(0..=1440),
                );
            });
        });

        ui.add_space(8.0);

        // ── Save Monitor Settings ──────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Save Monitor").strong().size(14.0));
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Snapshot interval (minutes):");
                ui.add(
                    egui::DragValue::new(&mut settings.save_monitor_settings.interval_minutes)
                        .range(1..=60),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Max snapshots per preset:");
                ui.add(
                    egui::DragValue::new(
                        &mut settings.save_monitor_settings.max_snapshots_per_preset,
                    )
                    .range(1..=100),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Keep every Nth snapshot (protected from cleanup):");
                ui.add(
                    egui::DragValue::new(&mut settings.save_monitor_settings.keep_every_nth)
                        .range(0..=50),
                );
            });
            ui.label(
                egui::RichText::new(
                    "  0 = no protection. 5 = every 5th oldest snapshot is kept during cleanup."
                )
                .small()
                .color(ui.visuals().weak_text_color()),
            );
            ui.checkbox(
                &mut settings.save_monitor_settings.include_entangled,
                "Include Entangled Worlds in snapshots",
            );
            ui.checkbox(
                &mut settings.save_monitor_settings.start_in_monitor_mode,
                "Start Save Monitor on launch",
            );
        });

        ui.add_space(8.0);

        // ── Gallery Settings ───────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("Preset Vault").strong().size(14.0));
            ui.add_space(4.0);

            ui.label("Catalog URL:");
            ui.add(
                egui::TextEdit::singleline(&mut settings.gallery_settings.catalog_url)
                    .desired_width(ui.available_width()),
            );
            ui.add_space(4.0);
            ui.label("Steam Path:");
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut settings.gallery_settings.steam_path)
                        .desired_width(ui.available_width() - 100.0),
                );
                if ui.button("Auto-detect").clicked() {
                    if let Ok(path) = crate::core::workshop::detect_steam_path() {
                        settings.gallery_settings.steam_path =
                            path.to_string_lossy().to_string();
                    }
                }
            });
        });

        ui.add_space(16.0);

        // ── Action Buttons ─────────────────────────────────────────────
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("Save & Close").strong())
                .clicked()
            {
                if !settings.noita_dir.is_empty() {
                    let noita_path = std::path::PathBuf::from(&settings.noita_dir);
                    if !noita_path.join("mod_config.xml").exists() {
                        action = SettingsAction::ShowWarning;
                        return;
                    }
                }
                action = SettingsAction::Save;
            }
            if ui.button("Reset to Defaults").clicked() {
                action = SettingsAction::Reset;
            }
            if ui.button("Cancel").clicked() {
                action = SettingsAction::Cancel;
            }
        });

        ui.add_space(16.0);

        // ── Info Panels ────────────────────────────────────────────────
        ui.horizontal(|ui| {
            if ui.button("System Information").clicked() {
                app.active_modal = Some(Modal::SystemInfo);
            }
            if ui.button("Open Source Libraries").clicked() {
                app.active_modal = Some(Modal::OpenSourceLibraries);
            }
            if ui.button("Open Settings Folder").clicked() {
                if let Ok(dir) = crate::core::settings::get_data_dir() {
                    let _ = crate::core::platform::open_directory(&dir);
                }
            }
        });
    });

    // Handle deferred actions after the ScrollArea closure
    match action {
        SettingsAction::Save => {
            // Apply theme change
            crate::ui::theme::apply_theme(ui.ctx(), settings.dark_mode);
            app.apply_settings(settings);
            app.active_view = crate::models::View::ModList;
        }
        SettingsAction::Cancel => {
            app.active_view = crate::models::View::ModList;
        }
        SettingsAction::Reset => {
            app.pending_settings = Some(default_settings());
        }
        SettingsAction::ShowWarning => {
            app.active_modal = Some(Modal::Info {
                title: "Warning".to_string(),
                message: "The selected Noita directory does not contain mod_config.xml."
                    .to_string(),
            });
            app.pending_settings = Some(settings);
        }
        SettingsAction::None => {
            app.pending_settings = Some(settings);
        }
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
        gallery_settings: Default::default(),
        compact_mode: false,
    }
}
