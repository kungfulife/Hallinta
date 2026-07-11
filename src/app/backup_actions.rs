use super::HallintaApp;
use crate::core::{backup, logging};
use crate::models::*;
use crate::tasks::TaskResult;
use std::path::PathBuf;

fn backup_has_restorable_components(info: &BackupInfo) -> bool {
    info.contains_save00 || info.contains_save01 || info.contains_presets || info.contains_entangled
}

impl HallintaApp {
    // ── Backup ─────────────────────────────────────────────────────────

    pub(crate) fn can_start_manual_backup(&self) -> bool {
        !self.backup_state.in_progress
            && !self.backup_state.restoring
            && !self.save_monitor.snapshot_in_flight
    }

    pub fn start_backup_modal(&mut self) {
        self.active_modal = Some(Modal::ManualBackup {
            name: default_manual_backup_name(),
            items: self.manual_backup_items(),
            error: None,
        });
    }

    pub fn open_backup_manager(&mut self) {
        self.load_backup_list_async();
        self.active_modal = Some(Modal::BackupManager);
    }

    pub fn open_sessions_manager(&mut self) {
        self.load_sessions_async();
    }

    fn manual_backup_items(&self) -> Vec<ChecklistItem> {
        let mut items = vec![
            ChecklistItem {
                id: "save00".to_string(),
                label: "save00 (always included)".to_string(),
                checked: true,
                required: true,
            },
            ChecklistItem {
                id: "save01".to_string(),
                label: "save01".to_string(),
                checked: true,
                required: false,
            },
            ChecklistItem {
                id: "presets".to_string(),
                label: "presets.json".to_string(),
                checked: true,
                required: false,
            },
        ];

        if self.configured_entangled_dir().is_some() {
            items.push(ChecklistItem {
                id: "entangled".to_string(),
                label: "Entangled Worlds".to_string(),
                checked: true,
                required: false,
            });
        }

        items
    }

    pub fn submit_manual_backup(&mut self, name: String, selected: Vec<String>) {
        let fallback = default_manual_backup_name();
        let name = match backup::normalize_manual_backup_name(&name, &fallback) {
            Ok(name) => name,
            Err(error) => {
                let mut items = self.manual_backup_items();
                for item in &mut items {
                    if !item.required {
                        item.checked = selected.contains(&item.id);
                    }
                }
                self.active_modal = Some(Modal::ManualBackup {
                    name,
                    items,
                    error: Some(error),
                });
                return;
            }
        };

        if !self.can_start_manual_backup() {
            self.active_modal = Some(Modal::Info {
                title: "Backup Busy".to_string(),
                message: "Wait for the current backup or monitor snapshot to finish before creating a manual backup.".to_string(),
            });
            return;
        }

        let include_save01 = selected.iter().any(|id| id == "save01");
        let include_presets = selected.iter().any(|id| id == "presets");
        let include_entangled = selected.iter().any(|id| id == "entangled");
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

        logging::write_session_marker(&format!("MANUAL_BACKUP_START:name={name}"));
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                backup::create_backup(
                    &noita_dir,
                    include_save01,
                    include_presets,
                    include_entangled,
                    entangled_dir.as_deref(),
                    &name,
                )
            })
            .await
            .unwrap_or_else(|error| Err(format!("Backup task failed: {error}")));
            let _ = tx.send(TaskResult::BackupComplete(result));
        });
    }

    /// Restore the most recent backup with default options (one-click).
    pub fn restore_last_backup(&mut self) {
        match backup::list_backups() {
            Ok(list) if !list.is_empty() => {
                let latest = &list[0];
                let _ = logging::log(
                    "INFO",
                    &format!("Restore-last triggered: {}", latest.filename),
                    "Backup",
                );
                self.active_modal = Some(Modal::Confirm {
                    message: format!(
                        "Restore latest backup:\n{}\n({} MB)",
                        latest.filename,
                        latest.size_bytes / 1_048_576,
                    ),
                    confirm_text: "Restore".to_string(),
                    cancel_text: "Cancel".to_string(),
                    action: ConfirmAction::RestoreLatest(latest.filename.clone()),
                    cancel_action: None,
                    dismissable: false,
                });
            }
            Ok(_) => {
                self.active_modal = Some(Modal::Info {
                    title: "Restore".to_string(),
                    message: "No backups found.".to_string(),
                });
            }
            Err(e) => {
                let _ = logging::log(
                    "ERROR",
                    &format!("Restore-last list failed: {}", e),
                    "Backup",
                );
            }
        }
    }

    /// Apply a restore using default options (used by restore-last).
    pub fn apply_default_restore(&mut self, filename: String) {
        let info = match backup::get_backup_contents(&filename) {
            Ok(i) => i,
            Err(e) => {
                let _ = logging::log("ERROR", &format!("Restore peek failed: {}", e), "Backup");
                return;
            }
        };
        if !backup_has_restorable_components(&info) {
            self.active_modal = Some(Modal::Info {
                title: "Restore".to_string(),
                message: format!("Backup {filename} does not contain any restorable components."),
            });
            return;
        }
        let options = RestoreOptions {
            restore_save00: info.contains_save00,
            restore_save01: info.contains_save01,
            restore_presets: info.contains_presets,
            restore_entangled: info.contains_entangled,
        };
        let noita_dir = PathBuf::from(self.settings.noita_dir.clone());
        let entangled_dir = if options.restore_entangled {
            self.configured_entangled_dir().map(PathBuf::from)
        } else {
            None
        };
        let tx = self.task_tx.clone();
        self.backup_state.restoring = true;
        self.active_modal = Some(Modal::Progress {
            message: "Restoring backup...".to_string(),
            progress: 0.5,
        });
        logging::write_session_marker(&format!("RESTORE_START:auto={}", filename));
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                backup::restore_backup(&filename, &noita_dir, &options, entangled_dir.as_deref())
            })
            .await
            .unwrap_or_else(|e| Err(format!("Restore task failed: {}", e)));
            let _ = tx.send(TaskResult::RestoreComplete(result));
        });
    }

    pub fn start_restore_components(&mut self, filename: String) {
        let info = match backup::get_backup_contents(&filename) {
            Ok(info) => info,
            Err(error) => {
                self.active_modal = Some(Modal::Info {
                    title: "Restore".to_string(),
                    message: format!("Failed to inspect backup: {error}"),
                });
                return;
            }
        };

        let mut items = Vec::new();
        for (id, label, present) in [
            ("save00", "save00", info.contains_save00),
            ("save01", "save01", info.contains_save01),
            ("presets", "presets.json", info.contains_presets),
            ("entangled", "Entangled Worlds", info.contains_entangled),
        ] {
            if present {
                items.push(ChecklistItem {
                    id: id.to_string(),
                    label: label.to_string(),
                    checked: true,
                    required: false,
                });
            }
        }

        if items.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Restore".to_string(),
                message: format!("Backup {filename} does not contain any restorable components."),
            });
            return;
        }

        self.active_modal = Some(Modal::Checklist {
            title: format!("Restore {filename}"),
            message: "Select components to restore:".to_string(),
            items,
            action: ChecklistAction::Restore(filename),
        });
    }
}

fn default_manual_backup_name() -> String {
    chrono::Local::now()
        .format("Backup %Y-%m-%d %H-%M")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use super::*;

    fn empty_backup_info() -> BackupInfo {
        BackupInfo {
            filename: "empty.zip".to_string(),
            timestamp: String::new(),
            size_bytes: 0,
            contains_save00: false,
            contains_save01: false,
            contains_presets: false,
            contains_entangled: false,
        }
    }

    #[test]
    fn default_restore_requires_a_restorable_component() {
        assert!(!backup_has_restorable_components(&empty_backup_info()));

        let mut with_save = empty_backup_info();
        with_save.contains_save00 = true;
        assert!(backup_has_restorable_components(&with_save));
    }

    #[test]
    fn backup_modal_requests_name_and_defaults_optional_components_checked() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.entangled_dir = "C:/Entangled".to_string();

        app.start_backup_modal();

        let Some(Modal::ManualBackup {
            name, items, error, ..
        }) = app.active_modal
        else {
            panic!("expected named manual-backup modal");
        };
        assert!(!name.is_empty());
        assert!(error.is_none());
        let save00 = items
            .iter()
            .find(|item| item.id == "save00")
            .expect("save00 item should be present");
        assert!(save00.checked);
        assert!(save00.required);
        assert!(items.iter().any(|item| item.id == "save01" && item.checked));
        assert!(
            items
                .iter()
                .any(|item| item.id == "entangled" && item.checked)
        );
    }

    #[test]
    fn invalid_manual_backup_name_keeps_dialog_open_with_error() {
        let (_runtime, mut app) = test_app(Vec::new());

        app.submit_manual_backup("bad/name".to_string(), vec!["save00".to_string()]);

        assert!(matches!(
            app.active_modal,
            Some(Modal::ManualBackup {
                error: Some(ref message),
                ..
            }) if message.contains("invalid")
        ));
        assert!(!app.backup_state.in_progress);
    }

    #[test]
    fn empty_backup_archive_is_not_offered_as_a_successful_restore() {
        let (_runtime, mut app) = test_app(Vec::new());
        let backups_dir = crate::core::settings::get_data_dir()
            .expect("test data dir")
            .join("backups");
        std::fs::create_dir_all(&backups_dir).expect("create backups dir");
        let filename = format!("empty_restore_{}.zip", std::process::id());
        let path = backups_dir.join(&filename);
        let file = std::fs::File::create(&path).expect("create empty zip");
        zip::ZipWriter::new(file)
            .finish()
            .expect("finish empty zip");

        app.start_restore_components(filename);

        std::fs::remove_file(path).ok();
        assert!(matches!(
            app.active_modal,
            Some(Modal::Info { ref message, .. })
                if message.contains("does not contain any restorable components")
        ));
    }

    #[test]
    fn manual_backup_can_start_while_monitoring() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;

        assert!(app.can_start_manual_backup());
    }

    #[test]
    fn manual_backup_waits_for_monitor_snapshot_in_flight() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.save_monitor.snapshot_in_flight = true;

        assert!(!app.can_start_manual_backup());
    }
}
