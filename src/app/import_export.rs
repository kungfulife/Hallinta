use super::HallintaApp;
use crate::core::{gallery, logging, mods, platform, presets, settings, workshop};
use crate::models::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

struct FileSnapshot {
    path: PathBuf,
    contents: Option<Vec<u8>>,
}

pub(super) fn with_file_rollback<T>(
    paths: &[PathBuf],
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let snapshots: Vec<FileSnapshot> = paths
        .iter()
        .map(|path| {
            let contents = if path.exists() {
                Some(
                    std::fs::read(path)
                        .map_err(|e| format!("Failed to snapshot {}: {e}", path.display()))?,
                )
            } else {
                None
            };
            Ok(FileSnapshot {
                path: path.clone(),
                contents,
            })
        })
        .collect::<Result<_, String>>()?;

    match operation() {
        Ok(value) => Ok(value),
        Err(operation_error) => {
            let mut rollback_errors = Vec::new();
            for snapshot in snapshots.into_iter().rev() {
                let result = match snapshot.contents {
                    Some(contents) => std::fs::write(&snapshot.path, contents),
                    None if snapshot.path.exists() => std::fs::remove_file(&snapshot.path),
                    None => Ok(()),
                };
                if let Err(e) = result {
                    rollback_errors.push(format!("{}: {e}", snapshot.path.display()));
                }
            }

            if rollback_errors.is_empty() {
                Err(operation_error)
            } else {
                Err(format!(
                    "{operation_error}\nRollback incomplete: {}",
                    rollback_errors.join("; ")
                ))
            }
        }
    }
}

#[derive(Debug)]
struct PreparedModImport {
    new_mods: Vec<ModEntry>,
    missing: Vec<(String, String)>,
    matched_count: usize,
}

fn mod_import_identity(name: &str, workshop_id: &str) -> Result<String, String> {
    let workshop_id = workshop_id.trim();
    if !workshop_id.is_empty() && workshop_id != "0" {
        return Ok(format!("workshop:{workshop_id}"));
    }

    let name = name.trim();
    if name.is_empty() {
        return Err("Mod list contains a local mod with no name.".to_string());
    }
    Ok(format!("local:{name}"))
}

fn prepare_mod_import(current: &[ModEntry], content: &str) -> Result<PreparedModImport, String> {
    let imported: Vec<ModListEntry> =
        serde_json::from_str(content).map_err(|e| format!("Invalid mod list format: {e}"))?;
    if imported.is_empty() {
        return Err("The selected mod list contains no mods.".to_string());
    }
    if imported.len() > 500 {
        return Err("The selected mod list has too many mods (max 500).".to_string());
    }

    let mut current_by_identity = BTreeMap::new();
    for (index, mod_entry) in current.iter().enumerate() {
        let identity = mod_import_identity(&mod_entry.name, &mod_entry.workshop_id)?;
        current_by_identity.entry(identity).or_insert(index);
    }

    let mut imported_identities = BTreeSet::new();
    let mut matched_indices = Vec::new();
    let mut missing = Vec::new();
    for imported_mod in imported {
        let identity = mod_import_identity(&imported_mod.name, &imported_mod.workshop_id)?;
        if !imported_identities.insert(identity.clone()) {
            return Err(format!(
                "The selected mod list contains a duplicate mod: {}.",
                imported_mod.name
            ));
        }

        if let Some(index) = current_by_identity.get(&identity) {
            matched_indices.push(*index);
        } else {
            missing.push((imported_mod.name, imported_mod.workshop_id));
        }
    }

    let matched_set: BTreeSet<usize> = matched_indices.iter().copied().collect();
    let mut new_mods = Vec::with_capacity(current.len());
    for index in &matched_indices {
        let mut mod_entry = current[*index].clone();
        mod_entry.enabled = true;
        new_mods.push(mod_entry);
    }
    for (index, mod_entry) in current.iter().enumerate() {
        if !matched_set.contains(&index) {
            let mut mod_entry = mod_entry.clone();
            mod_entry.enabled = false;
            new_mods.push(mod_entry);
        }
    }

    Ok(PreparedModImport {
        new_mods,
        missing,
        matched_count: matched_indices.len(),
    })
}

impl HallintaApp {
    // ── Import / Export ────────────────────────────────────────────────

    pub fn import_mod_list(&mut self) {
        if !self.can_import_mod_list() {
            let _ = logging::log(
                "INFO",
                "Mod list import skipped while monitor is running",
                "ModManager",
            );
            return;
        }

        let path = rfd::FileDialog::new()
            .set_title("Import Mod List")
            .add_filter("JSON", &["json"])
            .pick_file();

        let path = match path {
            Some(p) => p,
            None => return,
        };

        let content = match mods::read_file(&path) {
            Ok(c) => c,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };

        let prepared = match prepare_mod_import(&self.current_mods, &content) {
            Ok(prepared) => prepared,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };

        if !prepared.missing.is_empty() {
            self.active_modal = Some(Modal::MissingMods {
                mods: prepared.missing,
                action: MissingModsAction::ModImport(prepared.new_mods),
            });
        } else {
            self.apply_mod_import(prepared.new_mods, prepared.matched_count);
        }
    }

    fn can_import_mod_list(&self) -> bool {
        !self.save_monitor.is_running()
    }

    pub(super) fn apply_mod_import(&mut self, new_mods: Vec<ModEntry>, matched_count: usize) {
        let previous_mods = self.current_mods.clone();
        let previous_preset = self.presets.get(&self.selected_preset).cloned();
        let previous_settings = self.settings.clone();
        let data_dir = match settings::get_data_dir() {
            Ok(data_dir) => data_dir,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };
        let mut persistence_paths = vec![
            data_dir.join("presets.json"),
            data_dir.join("settings.json"),
        ];
        if !self.settings.noita_dir.is_empty() {
            persistence_paths.push(PathBuf::from(&self.settings.noita_dir).join("mod_config.xml"));
        }

        self.current_mods = new_mods;
        let result =
            with_file_rollback(&persistence_paths, || self.try_save_mod_config_and_preset());
        match result {
            Ok(()) => {
                let _ = logging::log(
                    "INFO",
                    &format!("Imported mod list ({} mods matched)", matched_count),
                    "ModManager",
                );
            }
            Err(e) => {
                self.current_mods = previous_mods;
                self.settings = previous_settings;
                if let Some(previous_preset) = previous_preset {
                    self.presets
                        .insert(self.selected_preset.clone(), previous_preset);
                } else {
                    self.presets.remove(&self.selected_preset);
                }
                let _ = logging::log("ERROR", &format!("Import failed: {e}"), "ModManager");
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
            }
        }
    }

    pub fn export_mod_list(&mut self) {
        let enabled: Vec<ModListEntry> = self
            .current_mods
            .iter()
            .filter(|m| m.enabled)
            .map(|m| ModListEntry {
                name: m.name.clone(),
                workshop_id: m.workshop_id.clone(),
            })
            .collect();

        if enabled.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Export".to_string(),
                message: "No enabled mods to export.".to_string(),
            });
            return;
        }

        self.pending_mod_list_export = Some((self.selected_preset.clone(), enabled));
    }

    fn export_selected_mod_list(&mut self, preset_name: String, enabled: Vec<ModListEntry>) {
        let path = rfd::FileDialog::new()
            .set_title("Export Enabled Mods")
            .set_file_name(format!("{}-mod-list.json", preset_name))
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            let result = serde_json::to_string_pretty(&enabled)
                .map_err(|e| format!("Serialization failed: {e}"))
                .and_then(|content| mods::write_file(&path, &content));
            self.finish_export(
                result,
                format!("Exported {} mods", enabled.len()),
                "ModManager",
            );
        }
    }

    fn finish_export(&mut self, result: Result<(), String>, success: String, module: &str) {
        match result {
            Ok(()) => {
                let _ = logging::log("INFO", &success, module);
            }
            Err(e) => {
                let _ = logging::log("ERROR", &format!("Export failed: {e}"), module);
                self.active_modal = Some(Modal::Info {
                    title: "Export Failed".to_string(),
                    message: e,
                });
            }
        }
    }

    pub fn start_export_presets(&mut self) {
        if !self.can_export_presets() {
            return;
        }

        let preset_names: Vec<String> = self.presets.keys().cloned().collect();
        if preset_names.is_empty() {
            return;
        }

        let items: Vec<ChecklistItem> = preset_names
            .iter()
            .map(|name| {
                let count = self.presets.get(name).map_or(0, |m| m.len());
                ChecklistItem {
                    id: name.clone(),
                    label: format!("{} ({} mods)", name, count),
                    checked: true,
                    required: false,
                }
            })
            .collect();

        self.active_modal = Some(Modal::Checklist {
            title: "Export Presets".to_string(),
            message: "Select presets to export:".to_string(),
            items,
            action: ChecklistAction::ExportPresets,
        });
    }

    pub(crate) fn can_export_presets(&self) -> bool {
        !self.backup_state.in_progress && !self.backup_state.restoring
    }

    pub(super) fn run_deferred_file_dialogs(&mut self) {
        if self.active_modal.is_some() {
            return;
        }

        if let Some((preset_name, enabled)) = self.pending_mod_list_export.take() {
            self.export_selected_mod_list(preset_name, enabled);
            return;
        }

        let Some(selected) = self.pending_preset_export.take() else {
            return;
        };

        self.export_selected_presets(selected);
    }

    fn export_selected_presets(&mut self, selected: Vec<String>) {
        let mut export_presets = BTreeMap::new();
        for name in &selected {
            if let Some(mods_list) = self.presets.get(name) {
                export_presets.insert(name.clone(), mods_list.clone());
            }
        }

        let checksum = serde_json::to_string(&export_presets)
            .ok()
            .map(|s| gallery::compute_checksum(&s));

        let export = PresetExportFile {
            hallinta_export: "presets".to_string(),
            version: platform::get_version(),
            presets: export_presets,
            checksum,
        };

        let path = rfd::FileDialog::new()
            .set_title("Export Presets")
            .set_file_name("hallinta-presets.json")
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            let result = serde_json::to_string_pretty(&export)
                .map_err(|e| format!("Serialization failed: {e}"))
                .and_then(|content| mods::write_file(&path, &content));
            self.finish_export(
                result,
                format!("Exported {} preset(s)", selected.len()),
                "PresetManager",
            );
        }
    }

    pub fn import_presets(&mut self) {
        let path = rfd::FileDialog::new()
            .set_title("Import Presets")
            .add_filter("JSON", &["json"])
            .pick_file();

        let path = match path {
            Some(p) => p,
            None => return,
        };

        let content = match mods::read_file(&path) {
            Ok(c) => c,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };

        if let Err(e) = presets::validate_preset_file(&content) {
            self.active_modal = Some(Modal::Info {
                title: "Import Rejected".to_string(),
                message: e,
            });
            return;
        }

        let import_data: PresetExportFile = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Invalid preset file: {}", e),
                });
                return;
            }
        };

        if import_data.hallinta_export != "presets" || import_data.presets.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Import Failed".to_string(),
                message: "Invalid preset file format.".to_string(),
            });
            return;
        }

        // Checksum verification
        if let Some(ref checksum) = import_data.checksum
            && let Ok(canonical) = serde_json::to_string(&import_data.presets)
            && !gallery::verify_checksum(&canonical, checksum)
        {
            let import = PresetImportData {
                presets: import_data.presets.clone(),
                selected_names: import_data.presets.keys().cloned().collect(),
            };
            self.active_modal = Some(Modal::Confirm {
                message: "Checksum mismatch: the preset file may have been modified. Continue?"
                    .to_string(),
                confirm_text: "Continue".to_string(),
                cancel_text: "Cancel".to_string(),
                action: ConfirmAction::ChecksumMismatchContinue(import),
                cancel_action: None,
                dismissable: false,
            });
            return;
        }

        // Check for missing workshop mods across all presets
        let steam_path = &self.settings.steam_path;
        if !steam_path.is_empty() {
            let all_workshop_ids: Vec<String> = import_data
                .presets
                .values()
                .flatten()
                .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
                .map(|m| m.workshop_id.clone())
                .collect();

            if !all_workshop_ids.is_empty()
                && let Ok(report) =
                    workshop::check_workshop_mods_installed(&all_workshop_ids, steam_path)
            {
                let missing: Vec<(String, String)> = import_data
                    .presets
                    .values()
                    .flatten()
                    .filter(|m| {
                        report.statuses.iter().any(|(id, state)| {
                            id == &m.workshop_id && *state == WorkshopInstallState::Missing
                        })
                    })
                    .map(|m| (m.name.clone(), m.workshop_id.clone()))
                    .collect();

                if !missing.is_empty() {
                    let import = PresetImportData {
                        presets: import_data.presets,
                        selected_names: Vec::new(),
                    };
                    self.active_modal = Some(Modal::MissingMods {
                        mods: missing,
                        action: MissingModsAction::PresetImport(import),
                    });
                    return;
                }
            }
        }

        // Show checklist for which presets to import
        let items = self.build_preset_import_checklist(&import_data.presets);

        self.active_modal = Some(Modal::Checklist {
            title: "Import Presets".to_string(),
            message: "Select presets to import:".to_string(),
            items,
            action: ChecklistAction::ImportPresets(PresetImportData {
                presets: import_data.presets,
                selected_names: Vec::new(),
            }),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use super::*;

    fn current_mods() -> Vec<ModEntry> {
        vec![
            super::super::test_support::mod_entry("Local", false, "0"),
            super::super::test_support::mod_entry("Workshop", false, "123"),
            super::super::test_support::mod_entry("Other", true, "456"),
        ]
    }

    #[test]
    fn empty_mod_list_import_is_rejected() {
        let err = prepare_mod_import(&current_mods(), "[]")
            .expect_err("empty import must not disable every mod");

        assert!(err.contains("no mods"));
    }

    #[test]
    fn duplicate_mod_list_identity_is_rejected() {
        let content = r#"[
            {"name":"Workshop","workshop_id":"123"},
            {"name":"Duplicate label","workshop_id":"123"}
        ]"#;

        let err = prepare_mod_import(&current_mods(), content)
            .expect_err("duplicate identity must not duplicate mod_config entries");

        assert!(err.contains("duplicate"));
    }

    #[test]
    fn local_numeric_name_does_not_match_workshop_id() {
        let current = vec![
            super::super::test_support::mod_entry("123", false, "0"),
            super::super::test_support::mod_entry("Remote", false, "123"),
        ];

        let prepared = prepare_mod_import(&current, r#"[{"name":" 123 ","workshop_id":"0"}]"#)
            .expect("local import should prepare");

        assert_eq!(prepared.new_mods[0].name, "123");
        assert!(prepared.new_mods[0].enabled);
        assert_eq!(prepared.new_mods[1].name, "Remote");
        assert!(!prepared.new_mods[1].enabled);
    }

    #[test]
    fn import_preparation_orders_matches_then_disables_remainder() {
        let content = r#"[
            {"name":"Workshop","workshop_id":"123"},
            {"name":"Missing","workshop_id":"999"},
            {"name":"Local","workshop_id":"0"}
        ]"#;

        let prepared = prepare_mod_import(&current_mods(), content).expect("import should prepare");

        assert_eq!(prepared.matched_count, 2);
        assert_eq!(
            prepared.missing,
            vec![("Missing".to_string(), "999".to_string())]
        );
        assert_eq!(
            prepared
                .new_mods
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Workshop", "Local", "Other"]
        );
        assert!(prepared.new_mods[0].enabled);
        assert!(prepared.new_mods[1].enabled);
        assert!(!prepared.new_mods[2].enabled);
    }

    #[test]
    fn mod_import_write_failure_keeps_previous_state_and_shows_error() {
        let original = current_mods();
        let (_runtime, mut app) = test_app(original.clone());
        app.settings.noita_dir = std::env::temp_dir()
            .join(format!(
                "hallinta-missing-import-dir-{}",
                std::process::id()
            ))
            .to_string_lossy()
            .to_string();
        let imported = vec![super::super::test_support::mod_entry(
            "Workshop", true, "123",
        )];

        app.apply_mod_import(imported, 1);

        assert_eq!(
            app.current_mods
                .iter()
                .map(|m| m.name.as_str())
                .collect::<Vec<_>>(),
            original.iter().map(|m| m.name.as_str()).collect::<Vec<_>>()
        );
        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, .. }) if title == "Import Failed"
        ));
    }

    #[test]
    fn failed_import_operation_restores_previous_disk_bytes() {
        let root = std::env::temp_dir().join(format!(
            "hallinta-import-rollback-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should follow epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("rollback test directory should exist");
        let path = root.join("state.json");
        std::fs::write(&path, b"before").expect("original state should write");

        let result = with_file_rollback(std::slice::from_ref(&path), || {
            std::fs::write(&path, b"after").map_err(|e| e.to_string())?;
            Err::<(), _>("later sink failed".to_string())
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).expect("state should read"), b"before");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn export_failure_is_shown_to_user() {
        let (_runtime, mut app) = test_app(Vec::new());

        app.finish_export(
            Err("disk full".to_string()),
            "unused success".to_string(),
            "ModManager",
        );

        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref title, ref message })
                if title == "Export Failed" && message.contains("disk full")
        ));
    }

    #[test]
    fn import_mod_list_has_app_layer_monitor_guard() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;

        assert!(!app.can_import_mod_list());
    }

    #[test]
    fn preset_export_is_allowed_while_monitoring() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;

        assert!(app.can_export_presets());
    }

    #[test]
    fn mod_list_export_defers_native_file_dialog() {
        let (_runtime, mut app) = test_app(vec![super::super::test_support::mod_entry(
            "Alpha", true, "1",
        )]);

        app.export_mod_list();

        assert!(app.pending_mod_list_export.is_some());
    }
}
