use super::HallintaApp;
use crate::core::{logging, save_monitor};
use crate::models::{InputAction, Modal, SessionStatus};
use crate::tasks::TaskResult;
use std::time::Instant;

impl HallintaApp {
    // ── Save Monitor ───────────────────────────────────────────────────

    pub fn start_save_monitor(&mut self) {
        let preset = self.selected_preset.clone();
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || save_monitor::list_stopped_sessions(&preset))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionCheckComplete(result));
        });
    }

    pub fn prompt_new_monitor_session(&mut self) {
        let default_name = save_monitor::generate_session_name();
        self.active_modal = Some(Modal::Input {
            title: "Name this monitor session".to_string(),
            value: String::new(),
            hint: default_name,
            action: InputAction::StartMonitorSession,
        });
    }

    pub fn start_new_monitor_session(&mut self, name: Option<&str>) {
        let name = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(save_monitor::generate_session_name);
        let preset = self.selected_preset.clone();
        let mods = self.current_mods.clone();
        match save_monitor::create_session(&preset, &name, &mods) {
            Ok(session) => {
                self.save_monitor.running = true;
                self.save_monitor.snapshot_count = 0;
                self.save_monitor.current_session = Some(session);
                let noita_dir = self.settings.noita_dir.clone();
                let include_save01 = self.settings.save_monitor_settings.include_save01;
                let entangled = if self.settings.save_monitor_settings.include_entangled {
                    self.configured_entangled_dir()
                } else {
                    None
                };
                self.save_monitor.last_known_mtime = save_monitor::scan_save_dirs_mtime(
                    &noita_dir,
                    include_save01,
                    entangled.as_deref(),
                );
                let _ = logging::log(
                    "INFO",
                    &format!("Monitor session started: {}", name),
                    "SaveMonitor",
                );
                logging::write_session_marker(&format!(
                    "MONITOR_START:preset={},session={}",
                    preset, name
                ));
                self.take_monitor_snapshot();
            }
            Err(e) => {
                let _ = logging::log(
                    "ERROR",
                    &format!("Failed to create session: {}", e),
                    "SaveMonitor",
                );
            }
        }
    }

    pub fn resume_monitor_session(&mut self, session_id: &str) {
        let preset = self.selected_preset.clone();
        match save_monitor::load_session(&preset, session_id) {
            Ok(mut session) => {
                session.status = SessionStatus::Monitoring;
                let _ = save_monitor::save_session(&session);
                self.save_monitor.running = true;
                self.save_monitor.snapshot_count = session.snapshot_count;
                self.save_monitor.current_session = Some(session);
                let noita_dir = self.settings.noita_dir.clone();
                let include_save01 = self.settings.save_monitor_settings.include_save01;
                let entangled = if self.settings.save_monitor_settings.include_entangled {
                    self.configured_entangled_dir()
                } else {
                    None
                };
                self.save_monitor.last_known_mtime = save_monitor::scan_save_dirs_mtime(
                    &noita_dir,
                    include_save01,
                    entangled.as_deref(),
                );
                let _ = logging::log("INFO", "Monitor session resumed", "SaveMonitor");
            }
            Err(e) => {
                let _ = logging::log(
                    "ERROR",
                    &format!("Failed to resume session: {}", e),
                    "SaveMonitor",
                );
            }
        }
    }

    pub fn stop_save_monitor(&mut self) {
        self.stop_save_monitor_inner(true);
    }

    fn stop_save_monitor_inner(&mut self, show_pending_external_mods: bool) {
        if let Some(ref mut session) = self.save_monitor.current_session {
            session.status = SessionStatus::Paused;
            session.snapshot_count = self.save_monitor.snapshot_count;
            let _ = save_monitor::save_session(session);
        }
        let count = self.save_monitor.snapshot_count;
        self.save_monitor.running = false;
        self.save_monitor.current_session = None;
        self.save_monitor.pending_change_since = None;
        let _ = logging::log("INFO", "Monitor session stopped", "SaveMonitor");
        logging::write_session_marker(&format!("MONITOR_STOP:snapshots={}", count));
        if show_pending_external_mods {
            self.show_pending_external_mods_after_monitor();
        }
    }

    pub(super) fn stop_monitor_for_close(&mut self) {
        self.stop_save_monitor_inner(false);
    }

    pub(crate) fn can_start_monitor_snapshot(&self) -> bool {
        !self.save_monitor.snapshot_in_flight
            && !self.backup_state.in_progress
            && !self.backup_state.restoring
    }

    pub(super) fn check_save_monitor_changes(&mut self) {
        let noita_dir = self.settings.noita_dir.clone();
        if noita_dir.is_empty() {
            return;
        }
        let include_save01 = self.settings.save_monitor_settings.include_save01;
        let entangled_dir = if self.settings.save_monitor_settings.include_entangled {
            self.configured_entangled_dir()
        } else {
            None
        };
        let current_mtime = save_monitor::scan_save_dirs_mtime(
            &noita_dir,
            include_save01,
            entangled_dir.as_deref(),
        );
        if current_mtime > self.save_monitor.last_known_mtime {
            self.save_monitor.last_known_mtime = current_mtime;
            let was_pending = self.save_monitor.pending_change_since.is_some();
            self.save_monitor.pending_change_since = Some(Instant::now());
            if !was_pending {
                let _ = logging::log(
                    "DEBUG",
                    "Save file change detected, waiting for stability...",
                    "SaveMonitor",
                );
            } else {
                let _ = logging::log(
                    "DEBUG",
                    "Save file changed again, resetting stability timer...",
                    "SaveMonitor",
                );
            }
        }
    }

    pub(super) fn take_monitor_snapshot(&mut self) {
        if !self.can_start_monitor_snapshot() {
            return;
        }

        let noita_dir = self.settings.noita_dir.clone();
        if noita_dir.is_empty() {
            return;
        }
        let session_id = match &self.save_monitor.current_session {
            Some(s) => s.id.clone(),
            None => return,
        };
        let preset_name = self.selected_preset.clone();
        let include_save01 = self.settings.save_monitor_settings.include_save01;
        let include_entangled = self.settings.save_monitor_settings.include_entangled;
        let entangled_dir = if include_entangled {
            self.configured_entangled_dir()
        } else {
            None
        };
        let tx = self.task_tx.clone();
        self.save_monitor.snapshot_in_flight = true;

        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                save_monitor::create_snapshot_in_session(
                    &noita_dir,
                    &preset_name,
                    &session_id,
                    include_save01,
                    include_entangled,
                    entangled_dir.as_deref(),
                )
            })
            .await
            .unwrap_or_else(|e| Err(format!("Snapshot task failed: {}", e)));
            let _ = tx.send(TaskResult::SnapshotComplete(result));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use crate::models::{SessionInfo, SessionStatus};
    use std::time::{Duration, Instant};

    fn test_session() -> SessionInfo {
        SessionInfo {
            id: "session-id".to_string(),
            name: "Session".to_string(),
            preset_name: "Default".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: None,
            status: SessionStatus::Monitoring,
            snapshot_count: 0,
            locked_mods: Vec::new(),
            folder_name: "session-id".to_string(),
        }
    }

    #[test]
    fn take_monitor_snapshot_waits_for_manual_backup_in_progress() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(test_session());
        app.backup_state.in_progress = true;

        app.take_monitor_snapshot();

        assert!(!app.save_monitor.snapshot_in_flight);
    }

    #[test]
    fn save_monitor_resets_pending_timer_when_save_changes_again() {
        let dir =
            std::env::temp_dir().join(format!("hallinta_monitor_debounce_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test save dir should be created");

        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.save_monitor.running = true;
        app.save_monitor.last_known_mtime = 0;
        let old_pending = Instant::now() - Duration::from_secs(180);
        app.save_monitor.pending_change_since = Some(old_pending);

        app.check_save_monitor_changes();

        let reset_pending = app
            .save_monitor
            .pending_change_since
            .expect("pending change should remain set");
        assert!(
            reset_pending > old_pending,
            "new save writes should reset the quiet-period timer"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
