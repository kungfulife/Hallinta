use super::HallintaApp;
use crate::core::{gallery, logging, mods, presets, workshop};
use crate::models::*;

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

        let imported: Vec<ModListEntry> = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Invalid mod list format: {}", e),
                });
                return;
            }
        };

        let mut found_in_order = Vec::new();
        let mut missing = Vec::new();

        for imp in &imported {
            let key = if imp.workshop_id != "0" && !imp.workshop_id.is_empty() {
                &imp.workshop_id
            } else {
                &imp.name
            };

            if let Some(pos) = self.current_mods.iter().position(|m| {
                if m.workshop_id != "0" && !m.workshop_id.is_empty() {
                    &m.workshop_id == key
                } else {
                    &m.name == key
                }
            }) {
                found_in_order.push(pos);
            } else {
                missing.push((imp.name.clone(), imp.workshop_id.clone()));
            }
        }

        if !missing.is_empty() {
            let mut new_mods = Vec::new();
            for &idx in &found_in_order {
                let mut m = self.current_mods[idx].clone();
                m.enabled = true;
                new_mods.push(m);
            }
            let found_set: std::collections::HashSet<usize> =
                found_in_order.iter().copied().collect();
            for (i, m) in self.current_mods.iter().enumerate() {
                if !found_set.contains(&i) {
                    let mut m = m.clone();
                    m.enabled = false;
                    new_mods.push(m);
                }
            }

            self.active_modal = Some(Modal::MissingMods {
                mods: missing,
                action: MissingModsAction::ModImport(new_mods),
            });
        } else {
            self.apply_mod_import(&found_in_order);
        }
    }

    fn can_import_mod_list(&self) -> bool {
        !self.save_monitor.is_running()
    }

    fn apply_mod_import(&mut self, found_indices: &[usize]) {
        let found_set: std::collections::HashSet<usize> = found_indices.iter().copied().collect();
        let mut new_mods = Vec::new();
        for &idx in found_indices {
            let mut m = self.current_mods[idx].clone();
            m.enabled = true;
            new_mods.push(m);
        }
        for (i, m) in self.current_mods.iter().enumerate() {
            if !found_set.contains(&i) {
                let mut m = m.clone();
                m.enabled = false;
                new_mods.push(m);
            }
        }
        self.current_mods = new_mods;
        self.save_mod_config_and_preset();
        let _ = logging::log(
            "INFO",
            &format!("Imported mod list ({} mods matched)", found_indices.len()),
            "ModManager",
        );
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

        let path = rfd::FileDialog::new()
            .set_title("Export Enabled Mods")
            .set_file_name(format!("{}-mod-list.json", self.selected_preset))
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            match serde_json::to_string_pretty(&enabled) {
                Ok(content) => {
                    if let Err(e) = mods::write_file(&path, &content) {
                        let _ =
                            logging::log("ERROR", &format!("Export failed: {}", e), "ModManager");
                    } else {
                        let _ = logging::log(
                            "INFO",
                            &format!("Exported {} mods", enabled.len()),
                            "ModManager",
                        );
                    }
                }
                Err(e) => {
                    let _ = logging::log(
                        "ERROR",
                        &format!("Serialization failed: {}", e),
                        "ModManager",
                    );
                }
            }
        }
    }

    pub fn start_export_presets(&mut self) {
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
            let raw_presets_str = serde_json::to_string(&import_data.presets).unwrap_or_default();
            if !gallery::verify_checksum(&raw_presets_str, checksum) {
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
                });
                return;
            }
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
                && let Ok(statuses) =
                    workshop::check_workshop_mods_installed(&all_workshop_ids, steam_path)
            {
                let missing: Vec<(String, String)> = import_data
                    .presets
                    .values()
                    .flatten()
                    .filter(|m| {
                        statuses
                            .iter()
                            .any(|(id, installed)| id == &m.workshop_id && !installed)
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

    #[test]
    fn import_mod_list_has_app_layer_monitor_guard() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;

        assert!(!app.can_import_mod_list());
    }
}
