use super::HallintaApp;
use crate::core::{logging, save_monitor, settings};
use crate::models::{ConfirmAction, Modal};
use crate::tasks::TaskResult;

impl HallintaApp {
    // ── Task Result Handling ───────────────────────────────────────────

    pub(super) fn poll_task_results(&mut self) {
        while let Ok(result) = self.task_rx.try_recv() {
            match result {
                TaskResult::BackupComplete(res) => {
                    self.backup_state.in_progress = false;
                    let was_modal_progress =
                        matches!(self.active_modal, Some(Modal::Progress { .. }));
                    if was_modal_progress {
                        self.active_modal = None;
                    }
                    match res {
                        Ok(filename) => {
                            let size = settings::get_data_dir()
                                .ok()
                                .and_then(|d| {
                                    std::fs::metadata(d.join("backups").join(&filename)).ok()
                                })
                                .map(|m| m.len())
                                .unwrap_or(0);
                            let _ = logging::log(
                                "INFO",
                                &format!("Backup created: {} ({} MB)", filename, size / 1_048_576),
                                "Backup",
                            );
                            logging::write_session_marker(&format!("BACKUP_OK:{}", filename));
                            self.load_backup_list_async();
                            let backup_path = settings::get_data_dir()
                                .map(|d| {
                                    d.join("backups")
                                        .join(&filename)
                                        .to_string_lossy()
                                        .to_string()
                                })
                                .unwrap_or(filename.clone());
                            // Don't override with success modal if a modal is already open (e.g. another action)
                            if was_modal_progress {
                                self.active_modal = Some(Modal::Info {
                                    title: "Backup Created".to_string(),
                                    message: format!("Saved to:\n{}", backup_path),
                                });
                            }
                        }
                        Err(e) => {
                            let _ =
                                logging::log("ERROR", &format!("Backup failed: {}", e), "Backup");
                            logging::write_session_marker("BACKUP_FAILED");
                            if was_modal_progress {
                                self.active_modal = Some(Modal::Info {
                                    title: "Backup Failed".to_string(),
                                    message: e,
                                });
                            }
                        }
                    }
                }
                TaskResult::RestoreComplete(res) => {
                    self.backup_state.restoring = false;
                    self.active_modal = None;
                    match res {
                        Ok(()) => {
                            let _ = logging::log(
                                "INFO",
                                &format!(
                                    "Restore complete — reloading mod list (preset=\"{}\")",
                                    self.selected_preset
                                ),
                                "Backup",
                            );
                            logging::write_session_marker("RESTORE_COMPLETE");
                            self.reload_mods();
                            self.check_workshop_mods_async();
                            self.active_modal = Some(Modal::Info {
                                title: "Restore Complete".to_string(),
                                message: "Save data was restored from backup.".to_string(),
                            });
                        }
                        Err(e) => {
                            let _ =
                                logging::log("ERROR", &format!("Restore failed: {}", e), "Backup");
                            logging::write_session_marker("RESTORE_FAILED");
                            self.active_modal = Some(Modal::Info {
                                title: "Restore Failed".to_string(),
                                message: e,
                            });
                        }
                    }
                }
                TaskResult::SnapshotComplete(res) => {
                    self.save_monitor.snapshot_in_flight = false;
                    match res {
                        Ok(filename) => {
                            self.save_monitor.snapshot_count += 1;
                            if let Some(ref mut session) = self.save_monitor.current_session {
                                session.snapshot_count = self.save_monitor.snapshot_count;
                                let _ = save_monitor::save_session(session);
                            }
                            let _ = logging::log(
                                "INFO",
                                &format!("Snapshot created: {}", filename),
                                "SaveMonitor",
                            );
                            // Session-scoped cleanup
                            if let Some(ref session) = self.save_monitor.current_session {
                                let preset = session.preset_name.clone();
                                let sid = session.id.clone();
                                let keep = self
                                    .settings
                                    .save_monitor_settings
                                    .max_snapshots_per_session;
                                let cleanup_tx = self.task_tx.clone();
                                self.async_runtime.spawn(async move {
                                    let result = tokio::task::spawn_blocking(move || {
                                        save_monitor::cleanup_session_snapshots(&preset, &sid, keep)
                                    })
                                    .await
                                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
                                    let _ = cleanup_tx
                                        .send(TaskResult::SnapshotCleanupComplete(result));
                                });
                            }
                        }
                        Err(e) => {
                            let _ = logging::log(
                                "ERROR",
                                &format!("Snapshot failed: {}", e),
                                "SaveMonitor",
                            );
                        }
                    }
                }
                TaskResult::UpgradeBackupComplete(res) => {
                    if let Err(e) = res {
                        let _ = logging::log(
                            "ERROR",
                            &format!("Upgrade backup failed: {}", e),
                            "Settings",
                        );
                    }
                }
                TaskResult::BackupListLoaded(res) => {
                    if let Ok(list) = res {
                        self.backup_state.backup_list = list;
                    }
                }
                TaskResult::SessionCheckComplete(res) => match res {
                    Ok(paused) if !paused.is_empty() => {
                        self.active_modal = Some(Modal::Confirm {
                            message: format!(
                                "Found {} paused session(s). Resume the most recent one?",
                                paused.len()
                            ),
                            confirm_text: "Resume".to_string(),
                            cancel_text: "New Session".to_string(),
                            action: ConfirmAction::ContinueMonitorSession(paused[0].id.clone()),
                            cancel_action: Some(ConfirmAction::StartNewMonitorSession),
                        });
                    }
                    _ => {
                        self.start_new_monitor_session();
                    }
                },
                TaskResult::SessionListLoaded(res) => {
                    if let Ok(sessions) = res {
                        self.active_modal = Some(Modal::RestoreManager {
                            sessions,
                            snapshots: Vec::new(),
                            selected_session: None,
                        });
                    }
                }
                TaskResult::SessionSnapshotsLoaded(res) => {
                    if let Ok(list) = res {
                        // Update the RestoreManager modal if it's open
                        if let Some(Modal::RestoreManager {
                            sessions,
                            selected_session,
                            ..
                        }) = self.active_modal.take()
                        {
                            self.active_modal = Some(Modal::RestoreManager {
                                sessions,
                                snapshots: list,
                                selected_session,
                            });
                        } else {
                            self.backup_state.snapshot_list = list;
                        }
                    }
                }
                TaskResult::WorkshopModsChecked(res) => match res {
                    Ok(status) => {
                        let total = status.len();
                        let installed = status.iter().filter(|(_, ok)| *ok).count();
                        let missing = total - installed;
                        self.backup_state.workshop_status = status;
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Workshop check: {}/{} installed{}",
                                installed,
                                total,
                                if missing > 0 {
                                    format!(", {} missing", missing)
                                } else {
                                    String::new()
                                }
                            ),
                            "Workshop",
                        );
                    }
                    Err(e) => {
                        let _ = logging::log(
                            "WARN",
                            &format!("Workshop check failed: {}", e),
                            "Workshop",
                        );
                    }
                },
                TaskResult::SnapshotCleanupComplete(res) => {
                    if let Ok(count) = res
                        && count > 0
                    {
                        let _ = logging::log(
                            "INFO",
                            &format!("Cleaned up {} old snapshot(s)", count),
                            "SaveMonitor",
                        );
                    }
                }
                TaskResult::BackupDeleted(res) => match res {
                    Ok(filename) => {
                        let _ = logging::log(
                            "INFO",
                            &format!("Deleted backup: {}", filename),
                            "Backup",
                        );
                        self.load_backup_list_async();
                    }
                    Err(e) => {
                        self.active_modal = Some(Modal::Info {
                            title: "Delete Failed".to_string(),
                            message: e,
                        });
                    }
                },
                TaskResult::MonitorDataCleared(res) => match res {
                    Ok(()) => {
                        let _ = logging::log("INFO", "Monitor data cleared", "SaveMonitor");
                    }
                    Err(e) => {
                        let _ = logging::log(
                            "ERROR",
                            &format!("Failed to clear monitor data: {}", e),
                            "SaveMonitor",
                        );
                    }
                },
            }
        }
    }
}
