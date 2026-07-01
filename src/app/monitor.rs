use super::HallintaApp;
use crate::core::{logging, save_monitor};
use crate::models::SessionStatus;
use crate::tasks::TaskResult;
use std::time::Instant;

impl HallintaApp {
    // ── Save Monitor ───────────────────────────────────────────────────

    pub fn start_save_monitor(&mut self) {
        let preset = self.selected_preset.clone();
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || save_monitor::list_paused_sessions(&preset))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionCheckComplete(result));
        });
    }

    pub fn start_new_monitor_session(&mut self) {
        let name = save_monitor::generate_session_name();
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
                session.status = SessionStatus::Active;
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

    pub fn end_monitor_session(&mut self) {
        if let Some(ref mut session) = self.save_monitor.current_session {
            session.status = SessionStatus::Ended;
            session.ended_at = Some(chrono::Utc::now().to_rfc3339());
            let _ = save_monitor::save_session(session);
        }
        let count = self.save_monitor.snapshot_count;
        self.save_monitor.running = false;
        self.save_monitor.current_session = None;
        self.save_monitor.pending_change_since = None;
        let _ = logging::log("INFO", "Monitor session ended", "SaveMonitor");
        logging::write_session_marker(&format!("MONITOR_STOP:snapshots={}", count));
    }

    pub fn stop_save_monitor(&mut self) {
        if let Some(ref mut session) = self.save_monitor.current_session {
            session.status = SessionStatus::Paused;
            session.snapshot_count = self.save_monitor.snapshot_count;
            let _ = save_monitor::save_session(session);
        }
        let count = self.save_monitor.snapshot_count;
        self.save_monitor.running = false;
        self.save_monitor.current_session = None;
        self.save_monitor.pending_change_since = None;
        let _ = logging::log("INFO", "Monitor session paused", "SaveMonitor");
        logging::write_session_marker(&format!("MONITOR_STOP:snapshots={}", count));
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
            if self.save_monitor.pending_change_since.is_none() {
                self.save_monitor.pending_change_since = Some(Instant::now());
                let _ = logging::log(
                    "DEBUG",
                    "Save file change detected, waiting for stability...",
                    "SaveMonitor",
                );
            }
        }
    }

    pub(super) fn take_monitor_snapshot(&mut self) {
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
