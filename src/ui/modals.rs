use crate::app::HallintaApp;
use crate::models::*;
use eframe::egui;

/// Render the active modal (if any).
pub fn render_modals(app: &mut HallintaApp, ctx: &egui::Context) {
    let modal = match app.active_modal.take() {
        Some(m) => m,
        None => return,
    };

    // Dim background BEHIND the modal (use Background order so modal renders on top)
    let screen_rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("modal_dimmer"),
    ));
    painter.rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(80));

    match modal {
        Modal::Confirm {
            message,
            confirm_text,
            cancel_text,
            action,
            cancel_action,
        } => {
            render_confirm(app, ctx, &message, &confirm_text, &cancel_text, action, cancel_action);
        }
        Modal::Input {
            title,
            mut value,
            action,
        } => {
            render_input(app, ctx, &title, &mut value, action);
        }
        Modal::Checklist {
            title,
            message,
            mut items,
            action,
        } => {
            render_checklist(app, ctx, &title, &message, &mut items, action);
        }
        Modal::Info { title, message } => {
            render_info(app, ctx, &title, &message);
        }
        Modal::Progress { message, progress } => {
            app.active_modal = Some(Modal::Progress { message, progress });
            render_progress(app, ctx);
        }
        Modal::MissingMods { mods, action } => {
            render_missing_mods(app, ctx, &mods, action);
        }
        Modal::SystemInfo => {
            render_system_info(app, ctx);
        }
        Modal::OpenSourceLibraries => {
            render_open_source(app, ctx);
        }
        Modal::BackupManager => {
            render_backup_manager(app, ctx);
        }
        Modal::SnapshotManager { preset_name } => {
            render_snapshot_manager(app, ctx, &preset_name);
        }
    }
}

fn render_confirm(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    message: &str,
    confirm_text: &str,
    cancel_text: &str,
    action: ConfirmAction,
    cancel_action: Option<ConfirmAction>,
) {
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new("Confirm")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(confirm_text).clicked() {
                    confirmed = true;
                }
                if ui.button(cancel_text).clicked() {
                    cancelled = true;
                }
            });
        });

    if confirmed {
        app.handle_confirm_action(action);
    } else if cancelled {
        if let Some(cancel_act) = cancel_action {
            app.handle_confirm_action(cancel_act);
        }
        // Otherwise just close
    } else {
        // Still open
        app.active_modal = Some(Modal::Confirm {
            message: message.to_string(),
            confirm_text: confirm_text.to_string(),
            cancel_text: cancel_text.to_string(),
            action,
            cancel_action,
        });
    }
}

fn render_input(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    title: &str,
    value: &mut String,
    action: InputAction,
) {
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            let response = ui.text_edit_singleline(value);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    confirmed = true;
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
        });

    if confirmed {
        app.handle_input_action(action, value.clone());
    } else if !cancelled {
        app.active_modal = Some(Modal::Input {
            title: title.to_string(),
            value: value.clone(),
            action,
        });
    }
}

fn render_checklist(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    title: &str,
    message: &str,
    items: &mut Vec<ChecklistItem>,
    action: ChecklistAction,
) {
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for item in items.iter_mut() {
                        ui.checkbox(&mut item.checked, &item.label);
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    confirmed = true;
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
        });

    if confirmed {
        let selected: Vec<String> = items
            .iter()
            .filter(|i| i.checked)
            .map(|i| i.id.clone())
            .collect();
        app.handle_checklist_action(action, selected);
    } else if !cancelled {
        app.active_modal = Some(Modal::Checklist {
            title: title.to_string(),
            message: message.to_string(),
            items: items.clone(),
            action,
        });
    }
}

fn render_info(app: &mut HallintaApp, ctx: &egui::Context, title: &str, message: &str) {
    let mut dismissed = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(8.0);
            if ui.button("OK").clicked() {
                dismissed = true;
            }
        });

    if !dismissed {
        app.active_modal = Some(Modal::Info {
            title: title.to_string(),
            message: message.to_string(),
        });
    }
}

fn render_progress(app: &mut HallintaApp, ctx: &egui::Context) {
    if let Some(Modal::Progress { ref message, progress }) = app.active_modal {
        egui::Window::new("Working...")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(message.as_str());
                ui.add_space(8.0);
                ui.add(egui::ProgressBar::new(progress).show_percentage());
            });
    }
}

fn render_missing_mods(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    mods: &[(String, String)],
    action: MissingModsAction,
) {
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new("Missing Workshop Mods")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("The following mods are not installed:")
                    .strong(),
            );
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for (name, workshop_id) in mods {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            if workshop_id != "0" && !workshop_id.is_empty() {
                                if ui.small_button("Subscribe").clicked() {
                                    let _ =
                                        crate::core::workshop::open_steam_subscribe(workshop_id);
                                }
                            }
                        });
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Continue Anyway").clicked() {
                    confirmed = true;
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });
        });

    if confirmed {
        app.handle_missing_mods_action(action);
    } else if !cancelled {
        app.active_modal = Some(Modal::MissingMods {
            mods: mods.to_vec(),
            action,
        });
    }
}

fn render_system_info(app: &mut HallintaApp, ctx: &egui::Context) {
    let mut open = true;
    egui::Window::new("System Information")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            if let Ok(info) = crate::core::platform::get_system_info() {
                egui::Grid::new("sysinfo_grid")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        sysinfo_row(ui, "Version", &info.app_version);
                        sysinfo_row(ui, "Build Profile", &info.build_profile);
                        sysinfo_row(ui, "Dev Build", &format!("{}", crate::core::platform::is_dev_build()));
                        sysinfo_row(ui, "Rust Version", &info.rust_version);
                        sysinfo_row(ui, "Cargo Version", &info.cargo_version);
                        sysinfo_row(ui, "Build Target", &info.build_target);
                        sysinfo_row(ui, "GUI Framework", &info.gui_framework);
                        sysinfo_row(ui, "OS", &info.os);
                        sysinfo_row(ui, "OS Family", &info.os_family);
                        sysinfo_row(ui, "Architecture", &info.arch);
                        sysinfo_row(ui, "CPU Cores", &info.logical_cpu_cores.to_string());
                        sysinfo_row(ui, "Local Time", &info.local_time);
                        sysinfo_row(ui, "UTC Time", &info.utc_time);
                        sysinfo_row(ui, "Executable Dir", &info.executable_dir);
                        sysinfo_row(ui, "App Data Dir", &info.app_data_dir);
                    });

                ui.add_space(8.0);
                ui.label(egui::RichText::new("Detected Paths").strong());
                ui.add_space(4.0);
                egui::Grid::new("paths_grid")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        let noita_path = crate::core::platform::get_noita_save_path()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|e| format!("Not found: {}", e));
                        sysinfo_row(ui, "Noita Save", &noita_path);

                        let steam_path = crate::core::workshop::detect_steam_path()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|e| format!("Not found: {}", e));
                        sysinfo_row(ui, "Steam", &steam_path);

                        let ew_path = crate::core::platform::get_entangled_worlds_save_path()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|e| format!("Not found: {}", e));
                        sysinfo_row(ui, "Entangled Worlds", &ew_path);
                    });
            }
        });

    if open {
        app.active_modal = Some(Modal::SystemInfo);
    }
}

fn sysinfo_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(egui::RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

fn render_open_source(app: &mut HallintaApp, ctx: &egui::Context) {
    let mut open = true;
    egui::Window::new("Open Source Libraries")
        .collapsible(false)
        .resizable(true)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            let libs = crate::core::platform::get_open_source_libraries();
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for lib in &libs {
                        ui.horizontal(|ui| {
                            ui.strong(&format!("{} v{}", lib.name, lib.version));
                            ui.label(&format!("- {}", lib.purpose));
                        });
                        if ui.small_button(&lib.homepage).clicked() {
                            let _ = crate::core::platform::open_url(&lib.homepage);
                        }
                        ui.add_space(4.0);
                    }
                });
        });

    if open {
        app.active_modal = Some(Modal::OpenSourceLibraries);
    }
}

fn render_backup_manager(app: &mut HallintaApp, ctx: &egui::Context) {
    let mut open = true;
    let mut delete_filename: Option<String> = None;

    egui::Window::new("Manage Backups")
        .collapsible(false)
        .resizable(true)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            if app.backup_state.backup_list.is_empty() {
                ui.label("No backups found.");
            } else {
                ui.label(egui::RichText::new(format!(
                    "{} backup(s)",
                    app.backup_state.backup_list.len()
                )).strong());
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        // Clone for iteration
                        let backups = app.backup_state.backup_list.clone();
                        for backup in &backups {
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.label(
                                                egui::RichText::new(&backup.filename).strong(),
                                            );
                                            ui.label(format!(
                                                "{:.1} MB | {}",
                                                backup.size_bytes as f64 / 1_048_576.0,
                                                &backup.timestamp[..19.min(backup.timestamp.len())]
                                            ));
                                            let mut contents = Vec::new();
                                            if backup.contains_save00 { contents.push("save00"); }
                                            if backup.contains_save01 { contents.push("save01"); }
                                            if backup.contains_presets { contents.push("presets"); }
                                            if backup.contains_entangled { contents.push("entangled"); }
                                            ui.label(
                                                egui::RichText::new(
                                                    format!("Contains: {}", contents.join(", "))
                                                )
                                                .small()
                                                .color(ui.visuals().weak_text_color()),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(
                                                        egui::RichText::new("Delete")
                                                            .color(egui::Color32::from_rgb(220, 60, 60)),
                                                    )
                                                    .clicked()
                                                {
                                                    delete_filename = Some(backup.filename.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                            ui.add_space(2.0);
                        }
                    });
            }
        });

    if let Some(filename) = delete_filename {
        app.active_modal = Some(Modal::Confirm {
            message: format!("Delete backup \"{}\"?", filename),
            confirm_text: "Delete".to_string(),
            cancel_text: "Cancel".to_string(),
            action: ConfirmAction::DeleteBackup(filename),
            cancel_action: None,
        });
    } else if open {
        app.active_modal = Some(Modal::BackupManager);
    }
}

fn render_snapshot_manager(app: &mut HallintaApp, ctx: &egui::Context, preset_name: &str) {
    let mut open = true;

    egui::Window::new(format!("Snapshots: {}", preset_name))
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .open(&mut open)
        .show(ctx, |ui| {
            if app.backup_state.snapshot_list.is_empty() {
                ui.label("No snapshots found for this preset.");
            } else {
                ui.label(egui::RichText::new(format!(
                    "{} snapshot(s)",
                    app.backup_state.snapshot_list.len()
                )).strong());
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(350.0)
                    .show(ui, |ui| {
                        let snapshots = app.backup_state.snapshot_list.clone();
                        for snapshot in &snapshots {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&snapshot.filename).strong());
                                ui.label(format!(
                                    "{:.1} MB",
                                    snapshot.size_bytes as f64 / 1_048_576.0
                                ));
                                ui.label(
                                    egui::RichText::new(
                                        &snapshot.timestamp[..19.min(snapshot.timestamp.len())]
                                    )
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                                );
                            });
                        }
                    });
            }
        });

    if open {
        app.active_modal = Some(Modal::SnapshotManager {
            preset_name: preset_name.to_string(),
        });
    }
}
