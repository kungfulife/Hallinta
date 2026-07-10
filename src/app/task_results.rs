use super::HallintaApp;
use crate::core::{logging, save_monitor, settings};
use crate::models::{ConfirmAction, Modal, WorkshopInstallState};
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
                                &format!(
                                    "Manual backup created: {} ({} MB)",
                                    filename,
                                    size / 1_048_576
                                ),
                                "Backup",
                            );
                            logging::write_session_marker(&format!(
                                "MANUAL_BACKUP_OK:{}",
                                filename
                            ));
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
                                    title: "Manual Backup Created".to_string(),
                                    message: format!("Saved to:\n{}", backup_path),
                                });
                            }
                        }
                        Err(e) => {
                            let _ = logging::log(
                                "ERROR",
                                &format!("Manual backup failed: {}", e),
                                "Backup",
                            );
                            logging::write_session_marker("MANUAL_BACKUP_FAILED");
                            if was_modal_progress {
                                self.active_modal = Some(Modal::Info {
                                    title: "Manual Backup Failed".to_string(),
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
                            self.sync_restore_manager_after_snapshot();
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
                    if self.close_after_snapshot {
                        self.close_after_snapshot = false;
                        self.stop_monitor_for_close();
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
                    Ok(stopped) if !stopped.is_empty() => {
                        self.active_modal = Some(Modal::Confirm {
                            message: format!(
                                "Found {} stopped session(s). Resume the most recent one?",
                                stopped.len()
                            ),
                            confirm_text: "Resume".to_string(),
                            cancel_text: "New Session".to_string(),
                            action: ConfirmAction::ContinueMonitorSession(stopped[0].id.clone()),
                            cancel_action: Some(ConfirmAction::StartNewMonitorSession),
                            dismissable: true,
                        });
                    }
                    _ => {
                        self.prompt_new_monitor_session();
                    }
                },
                TaskResult::SessionListLoaded {
                    result,
                    open_if_missing,
                } => {
                    if let Ok(sessions) = result {
                        match self.active_modal.take() {
                            Some(Modal::RestoreManager {
                                selected_session,
                                snapshots,
                                ..
                            }) => {
                                let selected = selected_session
                                    .filter(|(id, _)| sessions.iter().any(|s| s.id == *id));
                                let snapshots =
                                    if selected.is_some() { snapshots } else { Vec::new() };
                                self.active_modal = Some(Modal::RestoreManager {
                                    sessions,
                                    snapshots,
                                    selected_session: selected,
                                });
                            }
                            None if open_if_missing => {
                                self.active_modal = Some(Modal::RestoreManager {
                                    sessions,
                                    snapshots: Vec::new(),
                                    selected_session: None,
                                });
                            }
                            other => {
                                // Keep whatever modal is open; drop stale list refresh.
                                self.active_modal = other;
                            }
                        }
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
                TaskResult::WorkshopModsChecked { generation, result } => {
                    if generation != self.backup_state.workshop_check_generation {
                        continue;
                    }
                    self.backup_state.workshop_check_in_flight = false;
                    match result {
                        Ok(report) => {
                            let total = report.statuses.len();
                            let installed = report
                                .statuses
                                .iter()
                                .filter(|(_, state)| *state == WorkshopInstallState::Installed)
                                .count();
                            let missing = report
                                .statuses
                                .iter()
                                .filter(|(_, state)| *state == WorkshopInstallState::Missing)
                                .count();
                            let unknown = total.saturating_sub(installed + missing);
                            self.backup_state.workshop_status = report.statuses;
                            self.backup_state.workshop_diagnostic = report.diagnostic.clone();
                            let _ = logging::log(
                                "INFO",
                                &format!(
                                    "Workshop check: {}/{} installed{}{} (libraries={}, content_roots={})",
                                    installed,
                                    total,
                                    if missing > 0 {
                                        format!(", {} missing", missing)
                                    } else {
                                        String::new()
                                    },
                                    if unknown > 0 {
                                        format!(", {} unknown", unknown)
                                    } else {
                                        String::new()
                                    },
                                    report.libraries_checked.len(),
                                    report.content_roots_found
                                ),
                                "Workshop",
                            );
                            if let Some(diagnostic) = report.diagnostic {
                                let _ = logging::log("WARN", &diagnostic, "Workshop");
                            }
                        }
                        Err(e) => {
                            self.backup_state.workshop_diagnostic = Some(e.clone());
                            let _ = logging::log(
                                "WARN",
                                &format!("Workshop check failed: {}", e),
                                "Workshop",
                            );
                        }
                    }
                }
                TaskResult::SnapshotCleanupComplete(res) => {
                    if let Ok(count) = res
                        && count > 0
                    {
                        let _ = logging::log(
                            "INFO",
                            &format!("Cleaned up {} old snapshot(s)", count),
                            "SaveMonitor",
                        );
                        // File list may have dropped oldest zips — refresh open view.
                        self.refresh_restore_manager_if_open();
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

    /// Keep open "View Sessions" UI in sync when a new monitor snapshot lands.
    fn sync_restore_manager_after_snapshot(&mut self) {
        let Some(live) = self.save_monitor.current_session.clone() else {
            return;
        };

        let (found_in_list, viewing_live) = match &mut self.active_modal {
            Some(Modal::RestoreManager {
                sessions,
                selected_session,
                ..
            }) => {
                let found = if let Some(entry) = sessions.iter_mut().find(|s| s.id == live.id) {
                    entry.snapshot_count = live.snapshot_count;
                    entry.status = live.status.clone();
                    true
                } else {
                    false
                };
                let viewing = selected_session
                    .as_ref()
                    .is_some_and(|(id, _)| id == &live.id);
                (found, viewing)
            }
            _ => return,
        };

        if !found_in_list {
            // Live session not in the cached list yet — full refresh.
            self.refresh_restore_manager_if_open();
            return;
        }

        if viewing_live {
            self.load_session_snapshots_async(live.preset_name, live.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use crate::models::{
        Modal, SessionInfo, SessionStatus, WorkshopCheckReport, WorkshopInstallState,
    };
    use crate::tasks::TaskResult;

    fn workshop_report(id: &str, state: WorkshopInstallState) -> WorkshopCheckReport {
        WorkshopCheckReport {
            statuses: vec![(id.to_string(), state)],
            libraries_checked: vec!["C:/Steam".to_string()],
            content_roots_found: 1,
            diagnostic: None,
        }
    }

    #[test]
    fn snapshot_completion_finishes_pending_monitor_close() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.save_monitor.snapshot_in_flight = true;
        app.close_after_snapshot = true;
        app.close_requested = true;

        app.task_tx
            .send(TaskResult::SnapshotComplete(Ok("snapshot.zip".to_string())))
            .expect("test task result should send");

        app.poll_task_results();

        assert!(!app.save_monitor.running);
        assert!(!app.save_monitor.snapshot_in_flight);
        assert!(!app.close_after_snapshot);
        assert!(app.close_requested);
    }

    #[test]
    fn stale_workshop_check_result_is_ignored() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.backup_state.workshop_check_generation = 2;
        app.backup_state.workshop_check_in_flight = true;

        app.task_tx
            .send(TaskResult::WorkshopModsChecked {
                generation: 1,
                result: Ok(workshop_report("123", WorkshopInstallState::Installed)),
            })
            .expect("test task result should send");

        app.poll_task_results();

        assert!(app.backup_state.workshop_status.is_empty());
        assert!(app.backup_state.workshop_check_in_flight);
    }

    #[test]
    fn current_workshop_check_result_updates_status() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.backup_state.workshop_check_generation = 2;
        app.backup_state.workshop_check_in_flight = true;

        app.task_tx
            .send(TaskResult::WorkshopModsChecked {
                generation: 2,
                result: Ok(workshop_report("123", WorkshopInstallState::Missing)),
            })
            .expect("test task result should send");

        app.poll_task_results();

        assert_eq!(
            app.backup_state.workshop_status,
            vec![("123".to_string(), WorkshopInstallState::Missing)]
        );
        assert!(!app.backup_state.workshop_check_in_flight);
    }

    fn sample_session(id: &str, count: u32) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            name: format!("Session {id}"),
            preset_name: "Default".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: None,
            status: SessionStatus::Monitoring,
            snapshot_count: count,
            locked_mods: Vec::new(),
            folder_name: id.to_string(),
        }
    }

    #[test]
    fn session_list_refresh_preserves_selected_session() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.active_modal = Some(Modal::RestoreManager {
            sessions: vec![sample_session("a", 1)],
            snapshots: vec![],
            selected_session: Some(("a".to_string(), "Session a".to_string())),
        });

        app.task_tx
            .send(TaskResult::SessionListLoaded {
                result: Ok(vec![sample_session("a", 3), sample_session("b", 0)]),
                open_if_missing: false,
            })
            .expect("test task result should send");

        app.poll_task_results();

        match app.active_modal {
            Some(Modal::RestoreManager {
                ref sessions,
                ref selected_session,
                ..
            }) => {
                assert_eq!(sessions.len(), 2);
                assert_eq!(sessions[0].snapshot_count, 3);
                assert_eq!(
                    selected_session.as_ref().map(|(id, _)| id.as_str()),
                    Some("a")
                );
            }
            other => panic!("expected RestoreManager, got {other:?}"),
        }
    }

    #[test]
    fn session_list_refresh_does_not_reopen_closed_modal() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.active_modal = None;

        app.task_tx
            .send(TaskResult::SessionListLoaded {
                result: Ok(vec![sample_session("a", 1)]),
                open_if_missing: false,
            })
            .expect("test task result should send");

        app.poll_task_results();

        assert!(app.active_modal.is_none());
    }

    #[test]
    fn snapshot_complete_updates_open_session_list_count() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.save_monitor.snapshot_count = 2;
        app.save_monitor.current_session = Some(sample_session("live", 2));
        app.active_modal = Some(Modal::RestoreManager {
            sessions: vec![sample_session("live", 2), sample_session("old", 5)],
            snapshots: Vec::new(),
            selected_session: None,
        });

        app.task_tx
            .send(TaskResult::SnapshotComplete(Ok(
                "snapshot_20260101_000000.zip".to_string()
            )))
            .expect("test task result should send");

        app.poll_task_results();

        match app.active_modal {
            Some(Modal::RestoreManager { ref sessions, .. }) => {
                let live = sessions
                    .iter()
                    .find(|s| s.id == "live")
                    .expect("live session should remain listed");
                assert_eq!(live.snapshot_count, 3);
                assert_eq!(app.save_monitor.snapshot_count, 3);
            }
            other => panic!("expected RestoreManager, got {other:?}"),
        }
    }
}
