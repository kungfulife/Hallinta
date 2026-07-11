use super::HallintaApp;
use super::import_export::with_file_rollback;
use crate::core::{backup, logging, presets, save_monitor, settings};
use crate::models::*;
use crate::tasks::TaskResult;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
enum RestoreChecklistStep {
    ChooseBackup(String),
    RestoreBackup(String),
}

fn restore_checklist_step(
    action_filename: &str,
    selected: &[String],
) -> Option<RestoreChecklistStep> {
    if action_filename.is_empty() {
        selected
            .first()
            .cloned()
            .map(RestoreChecklistStep::ChooseBackup)
    } else {
        Some(RestoreChecklistStep::RestoreBackup(
            action_filename.to_string(),
        ))
    }
}

fn selected_has(selected: &[String], id: &str) -> bool {
    selected.iter().any(|item| item == id)
}

impl HallintaApp {
    // ── Modal Action Handlers ──────────────────────────────────────────

    pub fn handle_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DeletePreset => {
                if self.selected_preset != "Default" {
                    self.cancel_drag_if_active();
                    let deleted = self.selected_preset.clone();
                    let mod_count = self.presets.get(&deleted).map(|m| m.len()).unwrap_or(0);
                    self.presets.remove(&deleted);
                    self.selected_preset = "Default".to_string();
                    self.current_mods = self.presets.get("Default").cloned().unwrap_or_default();
                    self.save_mod_config_and_preset();
                    let _ = logging::log(
                        "INFO",
                        &format!(
                            "Deleted preset: {} ({} mods) — switched to Default",
                            deleted, mod_count
                        ),
                        "PresetManager",
                    );
                } else {
                    let _ =
                        logging::log("WARN", "Refused to delete Default preset", "PresetManager");
                }
            }
            ConfirmAction::DeleteMod(hint_idx, expected_name, expected_workshop) => {
                // Re-resolve by name + workshop_id so list mutations between menu open and
                // confirm do not delete the wrong mod.
                let resolved = if hint_idx < self.current_mods.len()
                    && self.current_mods[hint_idx].name == expected_name
                    && self.current_mods[hint_idx].workshop_id == expected_workshop
                {
                    Some(hint_idx)
                } else {
                    self.current_mods
                        .iter()
                        .position(|m| m.name == expected_name && m.workshop_id == expected_workshop)
                };
                match resolved {
                    Some(idx) => {
                        let removed = self.current_mods.remove(idx);
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Deleted mod: {} (workshop_id={}, was at position {})",
                                removed.name,
                                if removed.workshop_id.is_empty() {
                                    "none"
                                } else {
                                    &removed.workshop_id
                                },
                                idx + 1
                            ),
                            "ModManager",
                        );
                    }
                    None => {
                        let _ = logging::log(
                            "WARN",
                            &format!(
                                "Delete mod cancelled: \"{}\" no longer in list (workshop_id={})",
                                expected_name,
                                if expected_workshop.is_empty() {
                                    "none"
                                } else {
                                    &expected_workshop
                                }
                            ),
                            "ModManager",
                        );
                    }
                }
            }
            ConfirmAction::AcceptExternalChanges(file_mods) => {
                self.cancel_drag_if_active();
                let prev_count = self.current_mods.len();
                let new_count = file_mods.len();
                self.current_mods = file_mods;
                self.presets
                    .insert(self.selected_preset.clone(), self.current_mods.clone());
                let _ = presets::save_presets(&self.presets);
                let _ = logging::log(
                    "INFO",
                    &format!(
                        "Accepted external mod_config.xml change ({} -> {} mods)",
                        prev_count, new_count
                    ),
                    "ModManager",
                );
            }
            ConfirmAction::KeepCurrentPreset => {
                self.save_mod_config_and_preset();
            }
            ConfirmAction::OverwritePresetImport(import) => {
                self.do_import_presets(&import, true);
            }
            ConfirmAction::RenamePresetImport(import) => {
                self.do_import_presets(&import, false);
            }
            ConfirmAction::ChecksumMismatchContinue(import) => {
                let items: Vec<ChecklistItem> = import
                    .presets
                    .keys()
                    .map(|name| {
                        let count = import.presets.get(name).map_or(0, |m| m.len());
                        ChecklistItem {
                            id: name.clone(),
                            label: format!("{} ({} mods)", name, count),
                            checked: true,
                            required: false,
                        }
                    })
                    .collect();
                self.active_modal = Some(Modal::Checklist {
                    title: "Import Presets".to_string(),
                    message: "Select presets to import:".to_string(),
                    items,
                    action: ChecklistAction::ImportPresets(import),
                });
            }
            ConfirmAction::ExitWithSnapshot => {
                let _ = logging::log("INFO", "Exit chosen: save snapshot and close", "App");
                if !self.save_monitor.snapshot_in_flight && !self.can_start_monitor_snapshot() {
                    self.close_requested = false;
                    self.close_after_snapshot = false;
                    self.active_modal = Some(Modal::Info {
                        title: "Snapshot Not Saved".to_string(),
                        message: "Wait for the active backup or restore to finish, then try closing again."
                            .to_string(),
                    });
                    return;
                }
                self.close_requested = true;
                self.close_after_snapshot = true;
                if !self.save_monitor.snapshot_in_flight {
                    self.take_monitor_snapshot();
                }
                if !self.save_monitor.snapshot_in_flight {
                    self.close_requested = false;
                    self.close_after_snapshot = false;
                    self.active_modal = Some(Modal::Info {
                        title: "Snapshot Not Saved".to_string(),
                        message: "A snapshot could not be started. Check the configured save directory and active monitor session."
                            .to_string(),
                    });
                }
            }
            ConfirmAction::ExitWithoutSnapshot => {
                let _ = logging::log("INFO", "Exit chosen: close without snapshot", "App");
                self.close_requested = true;
                self.close_after_snapshot = false;
                self.stop_monitor_for_close();
            }
            ConfirmAction::DeleteBackup(filename) => {
                self.delete_backup_async(filename);
            }
            ConfirmAction::DeleteMonitorSession {
                preset_name,
                session_id,
                session_name,
            } => match save_monitor::delete_session_snapshots(&preset_name, &session_id) {
                Ok(()) => {
                    let _ = logging::log(
                        "INFO",
                        &format!("Deleted monitor session: {}", session_name),
                        "SaveMonitor",
                    );
                    self.load_sessions_async();
                }
                Err(e) => {
                    self.active_modal = Some(Modal::Info {
                        title: "Delete Failed".to_string(),
                        message: e,
                    });
                }
            },
            ConfirmAction::RestoreLatest(filename) => {
                self.apply_default_restore(filename);
            }
            ConfirmAction::ClearMonitorData => {
                self.clear_monitor_data_async();
            }
            ConfirmAction::ContinueMonitorSession(session_id) => {
                self.resume_monitor_session(&session_id);
            }
            ConfirmAction::StartNewMonitorSession => {
                self.prompt_new_monitor_session();
            }
            ConfirmAction::DismissConfirm => {}
        }
    }

    pub fn handle_input_action(&mut self, action: InputAction, value: String) {
        let value = value.trim().to_string();

        match action {
            InputAction::StartMonitorSession => {
                let name = if value.is_empty() {
                    None
                } else {
                    Some(value.as_str())
                };
                self.start_new_monitor_session(name);
                return;
            }
            InputAction::RenameMonitorSession {
                preset_name,
                session_id,
            } => {
                if value.is_empty() {
                    return;
                }
                match crate::core::save_monitor::rename_session(&preset_name, &session_id, &value) {
                    Ok(_) => self.load_sessions_async(),
                    Err(e) => {
                        self.active_modal = Some(Modal::Info {
                            title: "Rename Failed".to_string(),
                            message: e,
                        });
                    }
                }
                return;
            }
            _ => {}
        }

        if value.is_empty() {
            return;
        }

        match action {
            InputAction::CreatePreset => {
                if !self.presets.contains_key(&value) {
                    self.presets
                        .insert(value.clone(), self.current_mods.clone());
                    self.selected_preset = value.clone();
                    self.save_mod_config_and_preset();
                    let _ = logging::log(
                        "INFO",
                        &format!("Created preset: {}", value),
                        "PresetManager",
                    );
                }
            }
            InputAction::RenamePreset => {
                if self.selected_preset != "Default"
                    && !self.presets.contains_key(&value)
                    && value != self.selected_preset
                {
                    let old_name = self.selected_preset.clone();
                    if let Some(mods_list) = self.presets.remove(&old_name) {
                        self.presets.insert(value.clone(), mods_list);
                        self.selected_preset = value.clone();
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!("Renamed preset {} -> {}", old_name, value),
                            "PresetManager",
                        );
                    }
                }
            }
            InputAction::StartMonitorSession | InputAction::RenameMonitorSession { .. } => {}
            InputAction::MoveModToPosition(from_idx) => {
                if let Ok(target) = value.parse::<usize>() {
                    let target_idx = target.saturating_sub(1);
                    if from_idx < self.current_mods.len()
                        && target_idx < self.current_mods.len()
                        && from_idx != target_idx
                    {
                        let mod_name = self.current_mods[from_idx].name.clone();
                        let item = self.current_mods.remove(from_idx);
                        self.current_mods.insert(target_idx, item);
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Moved \"{}\" from position {} to {}",
                                mod_name,
                                from_idx + 1,
                                target_idx + 1
                            ),
                            "ModManager",
                        );
                    } else {
                        let _ = logging::log(
                            "WARN",
                            &format!(
                                "Move-to-position rejected (from={}, target={}, len={})",
                                from_idx + 1,
                                target_idx + 1,
                                self.current_mods.len()
                            ),
                            "ModManager",
                        );
                    }
                }
            }
        }
    }

    pub fn handle_checklist_action(&mut self, action: ChecklistAction, selected: Vec<String>) {
        match action {
            ChecklistAction::ExportPresets => {
                if selected.is_empty() {
                    return;
                }
                self.pending_preset_export = Some(selected);
            }
            ChecklistAction::ImportPresets(mut import) => {
                import.selected_names = selected;
                if import.selected_names.is_empty() {
                    return;
                }

                let conflicts: Vec<String> = import
                    .selected_names
                    .iter()
                    .filter(|n| self.presets.contains_key(*n))
                    .cloned()
                    .collect();

                if conflicts.is_empty() {
                    self.do_import_presets(&import, false);
                } else {
                    self.active_modal = Some(Modal::Confirm {
                        message: format!(
                            "{} preset(s) already exist: {}. Overwrite?",
                            conflicts.len(),
                            conflicts.join(", ")
                        ),
                        confirm_text: "Overwrite".to_string(),
                        cancel_text: "Rename".to_string(),
                        action: ConfirmAction::OverwritePresetImport(import.clone()),
                        cancel_action: Some(ConfirmAction::RenamePresetImport(import)),
                        dismissable: false,
                    });
                }
            }
            ChecklistAction::Backup => {
                if !self.can_start_manual_backup() {
                    self.active_modal = Some(Modal::Info {
                        title: "Backup Busy".to_string(),
                        message: "Wait for the current backup or monitor snapshot to finish before creating a manual backup.".to_string(),
                    });
                    return;
                }

                let include_save01 = selected.contains(&"save01".to_string());
                let include_presets = selected.contains(&"presets".to_string());
                let include_entangled = selected.contains(&"entangled".to_string());

                let noita_dir = PathBuf::from(self.settings.noita_dir.clone());
                let entangled_dir = if include_entangled {
                    self.configured_entangled_dir().map(PathBuf::from)
                } else {
                    None
                };
                let tx = self.task_tx.clone();

                self.backup_state.in_progress = true;
                self.active_modal = Some(Modal::Progress {
                    message: "Creating manual backup...".to_string(),
                    progress: 0.5,
                });

                logging::write_session_marker("MANUAL_BACKUP_START");
                self.async_runtime.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        backup::create_backup(
                            &noita_dir,
                            include_save01,
                            include_presets,
                            include_entangled,
                            entangled_dir.as_deref(),
                        )
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Backup task failed: {}", e)));
                    let _ = tx.send(TaskResult::BackupComplete(result));
                });
            }
            ChecklistAction::Restore(ref _filename) => {
                match restore_checklist_step(_filename, &selected) {
                    Some(RestoreChecklistStep::ChooseBackup(filename)) => {
                        if let Ok(info) = backup::get_backup_contents(&filename) {
                            let mut restore_items = Vec::new();
                            if info.contains_save00 {
                                restore_items.push(ChecklistItem {
                                    id: "save00".to_string(),
                                    label: "save00".to_string(),
                                    checked: true,
                                    required: false,
                                });
                            }
                            if info.contains_save01 {
                                restore_items.push(ChecklistItem {
                                    id: "save01".to_string(),
                                    label: "save01".to_string(),
                                    checked: true,
                                    required: false,
                                });
                            }
                            if info.contains_presets {
                                restore_items.push(ChecklistItem {
                                    id: "presets".to_string(),
                                    label: "presets.json".to_string(),
                                    checked: true,
                                    required: false,
                                });
                            }
                            if info.contains_entangled {
                                restore_items.push(ChecklistItem {
                                    id: "entangled".to_string(),
                                    label: "Entangled Worlds".to_string(),
                                    checked: true,
                                    required: false,
                                });
                            }

                            self.active_modal = Some(Modal::Checklist {
                                title: format!("Restore {}", filename),
                                message: "Select components to restore:".to_string(),
                                items: restore_items,
                                action: ChecklistAction::Restore(filename.clone()),
                            });
                        }
                    }
                    Some(RestoreChecklistStep::RestoreBackup(filename)) => {
                        let noita_dir = PathBuf::from(self.settings.noita_dir.clone());
                        let entangled_dir = if selected_has(&selected, "entangled") {
                            self.configured_entangled_dir().map(PathBuf::from)
                        } else {
                            None
                        };
                        let options = RestoreOptions {
                            restore_save00: selected_has(&selected, "save00"),
                            restore_save01: selected_has(&selected, "save01"),
                            restore_presets: selected_has(&selected, "presets"),
                            restore_entangled: selected_has(&selected, "entangled"),
                        };
                        let tx = self.task_tx.clone();

                        self.backup_state.restoring = true;
                        self.active_modal = Some(Modal::Progress {
                            message: "Restoring backup...".to_string(),
                            progress: 0.5,
                        });

                        logging::write_session_marker("RESTORE_START");
                        self.async_runtime.spawn(async move {
                            let result = tokio::task::spawn_blocking(move || {
                                backup::restore_backup(
                                    &filename,
                                    &noita_dir,
                                    &options,
                                    entangled_dir.as_deref(),
                                )
                            })
                            .await
                            .unwrap_or_else(|e| Err(format!("Restore task failed: {}", e)));
                            let _ = tx.send(TaskResult::RestoreComplete(result));
                        });
                    }
                    None => {}
                }
            }
            ChecklistAction::RestoreSnapshot(zip_path) => {
                let noita_dir = PathBuf::from(self.settings.noita_dir.clone());
                let entangled_dir = if selected_has(&selected, "entangled") {
                    self.configured_entangled_dir().map(PathBuf::from)
                } else {
                    None
                };
                let options = RestoreOptions {
                    restore_save00: selected_has(&selected, "save00"),
                    restore_save01: selected_has(&selected, "save01"),
                    restore_presets: false,
                    restore_entangled: selected_has(&selected, "entangled"),
                };
                let tx = self.task_tx.clone();

                self.backup_state.restoring = true;
                self.active_modal = Some(Modal::Progress {
                    message: "Restoring snapshot...".to_string(),
                    progress: 0.5,
                });

                logging::write_session_marker("SNAPSHOT_RESTORE_START");
                self.async_runtime.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        backup::restore_from_path(
                            &zip_path,
                            &noita_dir,
                            &options,
                            entangled_dir.as_deref(),
                        )
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Restore failed: {}", e)));
                    let _ = tx.send(TaskResult::RestoreComplete(result));
                });
            }
        }
    }

    pub fn handle_missing_mods_action(&mut self, action: MissingModsAction) {
        match action {
            MissingModsAction::ModImport(new_mods) => {
                let matched_count = new_mods
                    .iter()
                    .filter(|mod_entry| mod_entry.enabled)
                    .count();
                self.apply_mod_import(new_mods, matched_count);
            }
            MissingModsAction::PresetImport(import) => {
                // Show the preset selection checklist after acknowledging missing mods
                let items = self.build_preset_import_checklist(&import.presets);
                self.active_modal = Some(Modal::Checklist {
                    title: "Import Presets".to_string(),
                    message: "Select presets to import:".to_string(),
                    items,
                    action: ChecklistAction::ImportPresets(import),
                });
            }
        }
    }

    fn do_import_presets(&mut self, import: &PresetImportData, overwrite: bool) {
        let previous_presets = self.presets.clone();
        let mut imported = 0;
        for name in &import.selected_names {
            if let Some(mods_list) = import.presets.get(name) {
                let mut target_name = name.clone();
                if !overwrite {
                    target_name = self.unique_preset_name(name);
                }
                self.presets.insert(target_name, mods_list.clone());
                imported += 1;
            }
        }

        let result = settings::get_data_dir().and_then(|data_dir| {
            with_file_rollback(&[data_dir.join("presets.json")], || {
                presets::save_presets(&self.presets)
            })
        });
        if result.is_err() {
            self.presets = previous_presets;
        }
        self.finish_preset_import(result, imported);
    }

    fn finish_preset_import(&mut self, result: Result<(), String>, imported: usize) {
        match result {
            Ok(()) => {
                let _ = logging::log(
                    "INFO",
                    &format!("Imported {} preset(s)", imported),
                    "PresetManager",
                );
            }
            Err(e) => {
                let _ = logging::log(
                    "ERROR",
                    &format!("Preset import failed: {e}"),
                    "PresetManager",
                );
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Failed to save imported presets: {e}"),
                });
            }
        }
    }

    // ── Helpers ────────────────────────────────────────────────────────

    /// Generate a conflict-free preset name by appending " (imported)" / " (imported N)".
    fn unique_preset_name(&self, base_name: &str) -> String {
        let mut target = base_name.to_string();
        if self.presets.contains_key(&target) {
            target = format!("{} (imported)", base_name);
            let mut counter = 2;
            while self.presets.contains_key(&target) {
                target = format!("{} (imported {})", base_name, counter);
                counter += 1;
            }
        }
        target
    }

    pub(super) fn build_preset_import_checklist(
        &self,
        presets: &BTreeMap<String, Vec<ModEntry>>,
    ) -> Vec<ChecklistItem> {
        presets
            .keys()
            .map(|name| {
                let count = presets.get(name).map_or(0, |m| m.len());
                let exists = self.presets.contains_key(name);
                ChecklistItem {
                    id: name.clone(),
                    label: format!(
                        "{} ({} mods){}",
                        name,
                        count,
                        if exists { " - EXISTS" } else { "" }
                    ),
                    checked: true,
                    required: false,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn restore_checklist_chooses_backup_on_first_step() {
        assert_eq!(
            restore_checklist_step("", &ids(&["backup.zip"])),
            Some(RestoreChecklistStep::ChooseBackup("backup.zip".to_string()))
        );
    }

    #[test]
    fn restore_checklist_restores_components_when_filename_is_already_known() {
        assert_eq!(
            restore_checklist_step("backup.zip", &ids(&["save00", "presets"])),
            Some(RestoreChecklistStep::RestoreBackup(
                "backup.zip".to_string()
            ))
        );
    }

    #[test]
    fn exit_without_snapshot_stops_monitor_and_requests_close() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;

        app.handle_confirm_action(ConfirmAction::ExitWithoutSnapshot);

        assert!(!app.save_monitor.running);
        assert!(app.close_requested);
    }

    #[test]
    fn exit_with_snapshot_does_not_close_when_snapshot_cannot_start() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.backup_state.in_progress = true;

        app.handle_confirm_action(ConfirmAction::ExitWithSnapshot);

        assert!(app.save_monitor.running);
        assert!(!app.close_requested);
        assert!(!app.close_after_snapshot);
        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, .. }) if title == "Snapshot Not Saved"
        ));
    }

    #[test]
    fn preset_export_checklist_defers_native_file_dialog() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.presets.insert("Default".to_string(), Vec::new());

        app.handle_checklist_action(ChecklistAction::ExportPresets, ids(&["Default"]));

        assert_eq!(app.pending_preset_export, Some(ids(&["Default"])));
    }

    #[test]
    fn backup_checklist_waits_for_monitor_snapshot_in_flight() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.snapshot_in_flight = true;

        app.handle_checklist_action(ChecklistAction::Backup, ids(&["save01", "presets"]));

        assert!(!app.backup_state.in_progress);
        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, .. }) if title == "Backup Busy"
        ));
    }

    #[test]
    fn preset_import_save_failure_is_shown_to_user() {
        let (_runtime, mut app) = test_app(Vec::new());

        app.finish_preset_import(Err("disk full".to_string()), 2);

        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, ref message })
                if title == "Import Failed" && message.contains("disk full")
        ));
    }

    #[test]
    fn acknowledged_missing_mod_import_still_reports_write_failure() {
        let original = vec![super::super::test_support::mod_entry("Original", true, "1")];
        let (_runtime, mut app) = test_app(original.clone());
        app.settings.noita_dir = std::env::temp_dir()
            .join(format!(
                "hallinta-missing-modal-import-{}",
                std::process::id()
            ))
            .to_string_lossy()
            .to_string();

        app.handle_missing_mods_action(MissingModsAction::ModImport(vec![
            super::super::test_support::mod_entry("Imported", true, "2"),
        ]));

        assert_eq!(app.current_mods[0].name, "Original");
        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, .. }) if title == "Import Failed"
        ));
    }
}
