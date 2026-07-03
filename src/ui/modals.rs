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
            dismissable,
        } => {
            render_confirm(
                app,
                ctx,
                &message,
                &confirm_text,
                &cancel_text,
                action,
                cancel_action,
                dismissable,
            );
        }
        Modal::Input {
            title,
            mut value,
            hint,
            action,
        } => {
            render_input(app, ctx, &title, &mut value, &hint, action);
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
        Modal::ExternalModChanges { file_mods, summary } => {
            render_external_mod_changes(app, ctx, file_mods, &summary);
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
        Modal::RestoreManager {
            sessions,
            snapshots,
            selected_session,
        } => {
            render_restore_manager(app, ctx, sessions, snapshots, selected_session);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_confirm(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    message: &str,
    confirm_text: &str,
    cancel_text: &str,
    action: ConfirmAction,
    cancel_action: Option<ConfirmAction>,
    dismissable: bool,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut confirmed = false;
    let mut cancelled = false;
    let mut open = true;
    let mut dismissed = false;

    let paint_confirm = |ui: &mut egui::Ui| {
        if dismissable {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕").on_hover_text("Cancel").clicked() {
                        dismissed = true;
                    }
                });
            });
        }
        ui.label(message);
        ui.add_space(d.md);
        ui.horizontal(|ui| {
            if ui.button(confirm_text).clicked() {
                confirmed = true;
            }
            if ui.button(cancel_text).clicked() {
                cancelled = true;
            }
        });
    };

    let base_window = || {
        egui::Window::new("Confirm")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
    };

    if dismissable {
        base_window().open(&mut open).show(ctx, paint_confirm);
    } else {
        base_window().show(ctx, paint_confirm);
    }

    if confirmed {
        app.handle_confirm_action(action);
    } else if cancelled {
        if let Some(cancel_act) = cancel_action {
            app.handle_confirm_action(cancel_act);
        }
    } else if dismissed || (!open && dismissable) {
        app.handle_confirm_action(ConfirmAction::DismissConfirm);
    } else {
        app.active_modal = Some(Modal::Confirm {
            message: message.to_string(),
            confirm_text: confirm_text.to_string(),
            cancel_text: cancel_text.to_string(),
            action,
            cancel_action,
            dismissable,
        });
    }
}

fn render_input(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    title: &str,
    value: &mut String,
    hint: &str,
    action: InputAction,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            if !hint.is_empty() {
                let hint_color = ui.visuals().weak_text_color();
                ui.label(
                    egui::RichText::new(format!("Default: {}", hint))
                        .italics()
                        .color(hint_color),
                );
                ui.add_space(d.xs);
            }
            let response = ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .desired_width(280.0),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                confirmed = true;
            }
            ui.add_space(d.sm);
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
            hint: hint.to_string(),
            action,
        });
    }
}

fn render_checklist(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    title: &str,
    message: &str,
    items: &mut [ChecklistItem],
    action: ChecklistAction,
) {
    let _d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
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
            items: items.to_vec(),
            action,
        });
    }
}

fn render_info(app: &mut HallintaApp, ctx: &egui::Context, title: &str, message: &str) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut dismissed = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label(message);
            ui.add_space(d.sm);
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
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    if let Some(Modal::Progress {
        ref message,
        progress,
    }) = app.active_modal
    {
        egui::Window::new("Working...")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.label(message.as_str());
                ui.add_space(d.sm);
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
    let _d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut confirmed = false;
    let mut cancelled = false;

    egui::Window::new("Missing Workshop Mods")
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("The following mods are not installed:").strong());
            ui.add_space(8.0);
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    for (name, workshop_id) in mods {
                        ui.horizontal(|ui| {
                            ui.label(name);
                            if workshop_id != "0"
                                && !workshop_id.is_empty()
                                && ui.small_button("Subscribe").clicked()
                            {
                                let _ = crate::core::workshop::open_steam_subscribe(workshop_id);
                            }
                        });
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Subscribe All in Steam").clicked() {
                    for (_, workshop_id) in mods {
                        if !workshop_id.is_empty() && workshop_id != "0" {
                            let _ = crate::core::workshop::open_steam_subscribe(workshop_id);
                        }
                    }
                }
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

fn render_external_mod_changes(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    file_mods: Vec<ModEntry>,
    summary: &ExternalModChangeSummary,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut use_disk = false;
    let mut keep_current = false;

    egui::Window::new("Mod List Changed")
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label("Changes were noticed while monitoring.");
            ui.add_space(d.sm);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(d.sm as i8))
                .show(ui, |ui| {
                    ui.label(format!(
                        "Current: {} mods, {} enabled",
                        summary.current_total, summary.current_enabled
                    ));
                    ui.label(format!(
                        "Disk: {} mods, {} enabled",
                        summary.disk_total, summary.disk_enabled
                    ));
                    ui.separator();
                    ui.label(format!("Added: {}", summary.added));
                    ui.label(format!("Removed: {}", summary.removed));
                    ui.label(format!("Enabled changed: {}", summary.enabled_changed));
                    ui.label(format!(
                        "Order changed: {}",
                        if summary.order_changed { "Yes" } else { "No" }
                    ));
                });
            ui.add_space(d.md);
            ui.horizontal(|ui| {
                if ui.button("Use Disk List").clicked() {
                    use_disk = true;
                }
                if ui.button("Keep Current").clicked() {
                    keep_current = true;
                }
            });
        });

    if use_disk {
        app.handle_confirm_action(ConfirmAction::AcceptExternalChanges(file_mods));
    } else if keep_current {
        app.handle_confirm_action(ConfirmAction::KeepCurrentPreset);
    } else {
        app.active_modal = Some(Modal::ExternalModChanges {
            file_mods,
            summary: summary.clone(),
        });
    }
}

fn render_system_info(app: &mut HallintaApp, ctx: &egui::Context) {
    let _d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut open = true;
    egui::Window::new("System Information")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, |ui| {
            if let Ok(info) = crate::core::platform::get_system_info() {
                egui::Grid::new("sysinfo_grid")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(120.0)
                    .show(ui, |ui| {
                        sysinfo_row(ui, "Version", &info.app_version);
                        sysinfo_row(ui, "Git Hash", &info.git_hash);
                        sysinfo_row(ui, "Build Profile", &info.build_profile);
                        sysinfo_row(
                            ui,
                            "Dev Build",
                            &format!("{}", crate::core::platform::is_dev_build()),
                        );
                        sysinfo_row(ui, "Rust Version", &info.rust_version);
                        sysinfo_row(ui, "Cargo Version", &info.cargo_version);
                        sysinfo_row(ui, "Build Target", &info.build_target);
                        sysinfo_row(ui, "GUI Framework", &info.gui_framework);
                        sysinfo_row(ui, "OS", &info.os);
                        sysinfo_row(ui, "OS Family", &info.os_family);
                        sysinfo_row(ui, "Architecture", &info.arch);
                        sysinfo_row(ui, "Logical CPU Cores", &info.logical_cpu_cores.to_string());
                        sysinfo_row(ui, "Local Time", &info.local_time);
                        sysinfo_row(ui, "UTC Time", &info.utc_time);
                        sysinfo_row(ui, "Executable Dir", &info.executable_dir);
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
    let _d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut open = true;
    egui::Window::new("Open Source Libraries")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, |ui| {
            let libs = crate::core::platform::get_open_source_libraries();
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    for lib in &libs {
                        ui.horizontal(|ui| {
                            ui.strong(format!("{} v{}", lib.name, lib.version));
                            ui.label(format!("- {}", lib.purpose));
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
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut open = true;
    let mut delete_filename: Option<String> = None;

    egui::Window::new("Manage Backups")
        .collapsible(false)
        .resizable(false)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, |ui| {
            if app.backup_state.backup_list.is_empty() {
                ui.label("No backups found.");
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "{} backup(s)",
                        app.backup_state.backup_list.len()
                    ))
                    .strong(),
                );
                ui.add_space(d.sm);

                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        // Clone for iteration
                        let backups = app.backup_state.backup_list.clone();
                        for backup in &backups {
                            egui::Frame::group(ui.style())
                                .inner_margin(egui::Margin::same(d.sm as i8))
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
                                            if backup.contains_save00 {
                                                contents.push("save00");
                                            }
                                            if backup.contains_save01 {
                                                contents.push("save01");
                                            }
                                            if backup.contains_presets {
                                                contents.push("presets");
                                            }
                                            if backup.contains_entangled {
                                                contents.push("entangled");
                                            }
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Contains: {}",
                                                    contents.join(", ")
                                                ))
                                                .small()
                                                .color(ui.visuals().weak_text_color()),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(egui::RichText::new("Delete").color(
                                                        egui::Color32::from_rgb(220, 60, 60),
                                                    ))
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
            dismissable: false,
        });
    } else if open {
        app.active_modal = Some(Modal::BackupManager);
    }
}

fn render_restore_manager(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    sessions: Vec<SessionInfo>,
    snapshots: Vec<SnapshotEntry>,
    selected_session: Option<(String, String)>,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut open = true;
    let mut view_session: Option<(String, String)> = None;
    let mut back_to_list = false;
    let mut delete_session_id: Option<(String, String)> = None;
    let mut open_session_dir: Option<(String, String)> = None;
    let mut restore_snap: Option<SnapshotEntry> = None;

    let title = if let Some((_, ref name)) = selected_session {
        format!("Session: {}", name)
    } else {
        "Monitor Sessions".to_string()
    };

    let viewport = ctx.content_rect();
    let scale = app.settings.ui_scale;
    let modal_width = (360.0 * scale).min((viewport.width() * 0.92).max(240.0));
    let scroll_height = (viewport.height() * 0.55).clamp(180.0, 420.0);

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .default_width(modal_width)
        .max_width(modal_width)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.set_width(modal_width - 24.0);
            if let Some((ref _session_id, ref _session_name)) = selected_session {
                // ── Snapshot list view ──
                ui.horizontal(|ui| {
                    if ui.button("\u{2190} Back to Sessions").clicked() {
                        back_to_list = true;
                    }
                    if let Some((ref session_id, _)) = selected_session
                        && ui.button("Open Folder").clicked()
                    {
                        let preset = sessions
                            .iter()
                            .find(|session| session.id == *session_id)
                            .map(|session| session.preset_name.clone())
                            .unwrap_or_else(|| app.selected_preset.clone());
                        open_session_dir = Some((preset, session_id.clone()));
                    }
                });
                ui.add_space(d.sm);

                if snapshots.is_empty() {
                    ui.label("No snapshots in this session.");
                } else {
                    ui.label(egui::RichText::new("Available snapshots").strong());
                    ui.add_space(d.sm);

                    egui::ScrollArea::vertical()
                        .max_height(scroll_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let snaps = snapshots.clone();
                            for snap in &snaps {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(d.sm as i8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&snap.filename).strong(),
                                                );
                                                ui.label(format!(
                                                    "{:.1} MB | {}",
                                                    snap.size_bytes as f64 / 1_048_576.0,
                                                    &snap.timestamp[..19.min(snap.timestamp.len())]
                                                ));
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("Restore").clicked() {
                                                        restore_snap = Some(snap.clone());
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(2.0);
                            }
                        });
                }
            } else {
                // ── Session list view ──
                if sessions.is_empty() {
                    ui.label("No monitor sessions found for this preset.");
                } else {
                    ui.label(
                        egui::RichText::new(format!("{} session(s)", sessions.len())).strong(),
                    );
                    ui.add_space(d.sm);

                    egui::ScrollArea::vertical()
                        .max_height(scroll_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let sess = sessions.clone();
                            for session in &sess {
                                let is_live = app.save_monitor.is_running()
                                    && app
                                        .save_monitor
                                        .current_session
                                        .as_ref()
                                        .is_some_and(|current| current.id == session.id);

                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::same(d.sm as i8))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.set_max_width(ui.available_width() * 0.55);
                                                ui.label(
                                                    egui::RichText::new(&session.name).strong(),
                                                );
                                                if is_live {
                                                    ui.colored_label(
                                                        d.status_ok,
                                                        egui::RichText::new("Monitoring").strong(),
                                                    );
                                                }
                                                ui.label(format!(
                                                    "Started: {}",
                                                    &session.started_at
                                                        [..19.min(session.started_at.len())]
                                                ));
                                                ui.label(format!(
                                                    "{} snapshot(s)",
                                                    session.snapshot_count
                                                ));
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("View").clicked() {
                                                        view_session = Some((
                                                            session.id.clone(),
                                                            session.name.clone(),
                                                        ));
                                                    }
                                                    if ui.button("Open Folder").clicked() {
                                                        open_session_dir = Some((
                                                            session.preset_name.clone(),
                                                            session.id.clone(),
                                                        ));
                                                    }
                                                    if !is_live && ui.button("Rename").clicked() {
                                                        app.active_modal = Some(Modal::Input {
                                                            title: "Rename session".to_string(),
                                                            value: session.name.clone(),
                                                            hint: String::new(),
                                                            action:
                                                                InputAction::RenameMonitorSession {
                                                                    preset_name: session
                                                                        .preset_name
                                                                        .clone(),
                                                                    session_id: session.id.clone(),
                                                                },
                                                        });
                                                    }
                                                    if !is_live
                                                        && ui
                                                            .button(
                                                                egui::RichText::new("Delete")
                                                                    .color(
                                                                        egui::Color32::from_rgb(
                                                                            220, 60, 60,
                                                                        ),
                                                                    ),
                                                            )
                                                            .clicked()
                                                    {
                                                        delete_session_id = Some((
                                                            session.id.clone(),
                                                            session.preset_name.clone(),
                                                        ));
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(2.0);
                            }
                        });
                }
            }
        });

    // Handle actions
    if let Some((sid, sname)) = view_session {
        // Load snapshots for this session and switch to snapshot view
        let preset = app.selected_preset.clone();
        app.load_session_snapshots_async(preset, sid.clone());
        app.active_modal = Some(Modal::RestoreManager {
            sessions,
            snapshots: Vec::new(),
            selected_session: Some((sid, sname)),
        });
    } else if back_to_list {
        app.active_modal = Some(Modal::RestoreManager {
            sessions,
            snapshots: Vec::new(),
            selected_session: None,
        });
    } else if let Some((sid, preset)) = delete_session_id {
        let _ = crate::core::save_monitor::delete_session_snapshots(&preset, &sid);
        app.load_sessions_async();
    } else if let Some((preset, sid)) = open_session_dir {
        if let Ok(dir) = crate::core::save_monitor::get_session_dir_by_id(&preset, &sid) {
            let _ = crate::core::platform::open_directory(&dir);
        }
        app.active_modal = Some(Modal::RestoreManager {
            sessions,
            snapshots,
            selected_session,
        });
    } else if let Some(snap) = restore_snap {
        // Build restore checklist for this snapshot
        if let Ok(zip_path) = crate::core::save_monitor::get_snapshot_path(
            &app.selected_preset,
            &snap.session_id,
            &snap.filename,
        ) {
            let mut restore_items = vec![crate::models::ChecklistItem {
                id: "save00".to_string(),
                label: "save00".to_string(),
                checked: true,
            }];
            if app.settings.save_monitor_settings.include_save01 {
                restore_items.push(crate::models::ChecklistItem {
                    id: "save01".to_string(),
                    label: "save01".to_string(),
                    checked: true,
                });
            }
            if app.settings.save_monitor_settings.include_entangled {
                restore_items.push(crate::models::ChecklistItem {
                    id: "entangled".to_string(),
                    label: "Entangled Worlds".to_string(),
                    checked: true,
                });
            }
            app.active_modal = Some(Modal::Checklist {
                title: format!("Restore {}", snap.filename),
                message: "Select components to restore:".to_string(),
                items: restore_items,
                action: crate::models::ChecklistAction::RestoreSnapshot(zip_path),
            });
        }
    } else if open {
        app.active_modal = Some(Modal::RestoreManager {
            sessions,
            snapshots,
            selected_session,
        });
    }
}

#[cfg(test)]
mod tests {
    const RESIZABLE_TRUE: &str = concat!(".resizable", "(true)");

    #[test]
    fn modal_windows_are_not_user_resizable() {
        let source = include_str!("modals.rs");
        assert!(
            !source.contains(RESIZABLE_TRUE),
            "modal windows should not show user resize handles"
        );
    }
}
