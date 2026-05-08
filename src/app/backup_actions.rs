use super::HallintaApp;
use crate::core::{backup, logging};
use crate::models::*;
use crate::tasks::TaskResult;
use std::path::PathBuf;

impl HallintaApp {
    // ── Backup ─────────────────────────────────────────────────────────

    pub fn start_backup_modal(&mut self) {
        let mut items = vec![
            ChecklistItem {
                id: "save00".to_string(),
                label: "save00 (always included)".to_string(),
                checked: true,
            },
            ChecklistItem {
                id: "save01".to_string(),
                label: "save01".to_string(),
                checked: false,
            },
            ChecklistItem {
                id: "presets".to_string(),
                label: "presets.json".to_string(),
                checked: true,
            },
        ];

        if self.get_active_entangled_dir().is_some() {
            items.push(ChecklistItem {
                id: "entangled".to_string(),
                label: "Entangled Worlds".to_string(),
                checked: false,
            });
        }

        self.active_modal = Some(Modal::Checklist {
            title: "Create Backup".to_string(),
            message: "Select components to include:".to_string(),
            items,
            action: ChecklistAction::Backup,
        });
    }

    /// Auto-backup: silent quick backup (save00 + presets, no entangled, no save01).
    pub fn start_auto_backup(&mut self) {
        let noita_dir = PathBuf::from(self.get_active_noita_dir());
        if noita_dir.as_os_str().is_empty() {
            return;
        }
        let tx = self.task_tx.clone();
        self.backup_state.in_progress = true;
        let _ = logging::log("INFO", "Auto-backup triggered", "Backup");
        logging::write_session_marker(&format!(
            "AUTO_BACKUP_START:interval={}min",
            self.settings.backup_settings.backup_interval_minutes
        ));
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                backup::create_backup(&noita_dir, false, true, false, None)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Auto-backup task failed: {}", e)));
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
        let options = RestoreOptions {
            restore_save00: info.contains_save00,
            restore_save01: info.contains_save01,
            restore_presets: info.contains_presets,
            restore_entangled: info.contains_entangled,
        };
        let noita_dir = PathBuf::from(self.get_active_noita_dir());
        let entangled_dir = if options.restore_entangled {
            self.get_active_entangled_dir().map(PathBuf::from)
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

    pub fn start_restore_modal(&mut self) {
        let backups = match backup::list_backups() {
            Ok(b) => b,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Restore".to_string(),
                    message: format!("Failed to list backups: {}", e),
                });
                return;
            }
        };

        if backups.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Restore".to_string(),
                message: "No backups found.".to_string(),
            });
            return;
        }

        let items: Vec<ChecklistItem> = backups
            .iter()
            .map(|b| ChecklistItem {
                id: b.filename.clone(),
                label: format!(
                    "{} ({:.1} MB)",
                    b.filename,
                    b.size_bytes as f64 / 1_048_576.0
                ),
                checked: false,
            })
            .collect();

        self.active_modal = Some(Modal::Checklist {
            title: "Restore Backup".to_string(),
            message: "Select a backup to restore:".to_string(),
            items,
            action: ChecklistAction::Restore(String::new()),
        });
    }
}
