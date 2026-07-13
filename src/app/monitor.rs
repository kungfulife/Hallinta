use super::HallintaApp;
use crate::core::{logging, platform, process_probe, save_monitor};
use crate::models::{AutoMonitorMode, InputAction, Modal, SessionStatus};
use crate::tasks::TaskResult;
use std::time::{Duration, Instant};

const AUTO_MONITOR_PROBE_INTERVAL: Duration = Duration::from_secs(3);

impl HallintaApp {
    // ── Save Monitor ───────────────────────────────────────────────────

    pub(crate) fn can_start_save_monitor(&self) -> bool {
        self.is_noita_sync_live()
            && !self.backup_state.in_progress
            && !self.backup_state.restoring
            && !self.save_monitor.snapshot_in_flight
    }

    /// Begin the manual flow that offers stopped-session resume before naming a new session.
    pub fn offer_save_monitor_start(&mut self) {
        if !self.can_start_save_monitor() {
            self.active_modal = Some(Modal::Info {
                title: "Monitor Unavailable".to_string(),
                message: "Resolve the Noita configuration before starting Save Monitor."
                    .to_string(),
            });
            return;
        }
        // Latch process auto-monitor so launch-start / manual start do not double-prompt.
        self.save_monitor.auto_monitor_match_active = true;
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

    /// Rising-edge process detection for optional auto-start only.
    /// Never stops a session. Never prompts or starts while a session is already running.
    /// Independent of "Start Save Monitor on Hallinta launch".
    pub(super) fn poll_auto_monitor_from_processes(&mut self, now: Instant) {
        // Fold legacy continuous-Ask into Off + one-time intro when still pending.
        if self.settings.save_monitor_settings.auto_monitor_mode == AutoMonitorMode::Ask {
            self.settings.save_monitor_settings.auto_monitor_mode = AutoMonitorMode::Off;
        }

        // Active session owns monitoring — no intro, no auto-start, no auto-stop.
        if self.save_monitor.is_running() {
            self.save_monitor.auto_monitor_match_active = true;
            return;
        }

        let intro_pending = self
            .settings
            .save_monitor_settings
            .auto_monitor_intro_pending;
        let mode = self.settings.save_monitor_settings.auto_monitor_mode;
        if !intro_pending && mode != AutoMonitorMode::Always {
            self.save_monitor.auto_monitor_match_active = false;
            return;
        }

        let should_probe = self
            .save_monitor
            .last_process_probe
            .is_none_or(|t| now.duration_since(t) >= AUTO_MONITOR_PROBE_INTERVAL);
        if !should_probe {
            return;
        }
        self.save_monitor.last_process_probe = Some(now);

        let when = self.settings.save_monitor_settings.auto_monitor_when;
        let presence = process_probe::detect_game_processes();
        let is_match = presence.matches(when);
        if !is_match {
            self.save_monitor.auto_monitor_match_active = false;
            return;
        }

        // Already handled this continuous process presence (e.g. user declined, or stopped mid-run).
        if self.save_monitor.auto_monitor_match_active {
            return;
        }
        if !self.is_noita_sync_live() {
            return;
        }
        // Wait for a clear UI; do not latch yet so the offer still arrives after.
        if self.active_modal.is_some() {
            return;
        }

        self.save_monitor.auto_monitor_match_active = true;

        if intro_pending {
            self.active_modal = Some(Modal::AutoMonitorIntro {
                detection: presence.describe(when),
            });
            return;
        }

        if mode == AutoMonitorMode::Always {
            let _ = logging::log(
                "INFO",
                "Auto-starting Save Monitor after game process detection",
                "SaveMonitor",
            );
            self.start_new_monitor_session(None);
        }
    }

    pub fn finish_auto_monitor_intro(&mut self, enable_always: bool) {
        self.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = false;
        self.settings.save_monitor_settings.auto_monitor_mode = if enable_always {
            AutoMonitorMode::Always
        } else {
            AutoMonitorMode::Off
        };
        if let Err(error) = crate::core::settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Could not save auto-monitor intro choice: {error}"),
                "SaveMonitor",
            );
        }
        // Never stack a second start on top of an already-running session.
        if enable_always && !self.save_monitor.is_running() {
            self.start_new_monitor_session(None);
        }
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
        if !self.can_start_save_monitor() {
            self.active_modal = Some(Modal::Info {
                title: "Monitor Unavailable".to_string(),
                message: "Resolve the Noita configuration before starting Save Monitor."
                    .to_string(),
            });
            return;
        }
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
                self.refresh_restore_manager_if_open();
                let _ = self.take_monitor_snapshot();
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
        self.resume_monitor_session_for(&preset, session_id);
    }

    pub fn resume_monitor_session_for(&mut self, preset: &str, session_id: &str) {
        if !self.can_start_save_monitor() {
            self.active_modal = Some(Modal::Info {
                title: "Monitor Unavailable".to_string(),
                message: "Resolve the Noita configuration before resuming Save Monitor."
                    .to_string(),
            });
            return;
        }
        let preset = preset.to_string();
        match save_monitor::load_session(&preset, session_id) {
            Ok(mut session) => {
                self.selected_preset = preset.clone();
                self.settings.selected_preset = preset.clone();
                self.save_current_settings();
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
        self.save_monitor.last_write_at = None;
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
        self.can_start_monitor_snapshot_inner(false)
    }

    fn can_start_monitor_snapshot_inner(&self, bypass_update_freeze: bool) -> bool {
        !self.save_monitor.snapshot_in_flight
            && !self.backup_state.in_progress
            && !self.backup_state.restoring
            && (bypass_update_freeze || !self.update_state.is_locked())
    }

    pub(super) fn check_save_monitor_changes(&mut self) {
        let noita_dir = self.settings.noita_dir.clone();
        if !platform::is_configured_path(&noita_dir) {
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
            // Always track latest write for short stability debounce. Do NOT
            // reset pending_change_since — that would starve snapshots during
            // continuous Noita world writes (backup delay is minutes long).
            self.save_monitor.last_write_at = Some(Instant::now());
            if self.save_monitor.pending_change_since.is_none() {
                self.save_monitor.pending_change_since = Some(Instant::now());
                let _ = logging::log(
                    "DEBUG",
                    "Save file change detected, waiting for backup delay...",
                    "SaveMonitor",
                );
            } else {
                let _ = logging::log(
                    "DEBUG",
                    "Save file changed again during backup delay window...",
                    "SaveMonitor",
                );
            }
        }
    }

    pub(super) fn take_monitor_snapshot(&mut self) -> Result<u64, String> {
        self.take_monitor_snapshot_inner(false)
    }

    pub(super) fn take_update_final_snapshot(&mut self) -> Result<u64, String> {
        self.take_monitor_snapshot_inner(true)
    }

    fn take_monitor_snapshot_inner(&mut self, bypass_update_freeze: bool) -> Result<u64, String> {
        if !self.can_start_monitor_snapshot_inner(bypass_update_freeze) {
            return Err("A backup, restore, snapshot, or update snapshot freeze is active".into());
        }

        let noita_dir = self.settings.noita_dir.clone();
        if !platform::is_configured_path(&noita_dir) {
            return Err("No Noita save directory is configured".into());
        }
        let (preset_name, session_id) = match &self.save_monitor.current_session {
            Some(session) => (session.preset_name.clone(), session.id.clone()),
            None => return Err("No monitor session is active".into()),
        };
        let include_save01 = self.settings.save_monitor_settings.include_save01;
        let include_entangled = self.settings.save_monitor_settings.include_entangled;
        let entangled_dir = if include_entangled {
            self.configured_entangled_dir()
        } else {
            None
        };
        let tx = self.task_tx.clone();
        self.save_monitor.next_snapshot_request_id =
            self.save_monitor.next_snapshot_request_id.saturating_add(1);
        let request_id = self.save_monitor.next_snapshot_request_id;
        self.save_monitor.snapshot_in_flight = true;
        self.save_monitor.snapshot_request_id = Some(request_id);

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
            let _ = tx.send(TaskResult::SnapshotComplete { request_id, result });
        });
        Ok(request_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use crate::models::{AutoMonitorMode, SessionInfo, SessionStatus};
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
            retained_snapshot_count: 0,
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

        let _ = app.take_monitor_snapshot();

        assert!(!app.save_monitor.snapshot_in_flight);
    }

    #[test]
    fn save_monitor_keeps_backup_delay_start_when_save_changes_again() {
        let dir =
            std::env::temp_dir().join(format!("hallinta_monitor_debounce_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test save dir should be created");
        // Touch a file so scan_save_dirs_mtime reports a non-zero mtime.
        std::fs::write(dir.join("player.txt"), b"x").expect("test file should write");

        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.save_monitor.running = true;
        app.save_monitor.last_known_mtime = 0;
        let old_pending = Instant::now() - Duration::from_secs(180);
        app.save_monitor.pending_change_since = Some(old_pending);

        app.check_save_monitor_changes();

        let still_pending = app
            .save_monitor
            .pending_change_since
            .expect("pending change should remain set");
        assert_eq!(
            still_pending, old_pending,
            "later writes must not restart the multi-minute backup delay"
        );
        assert!(
            app.save_monitor.last_write_at.is_some(),
            "latest write time should advance for short stability debounce"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn update_snapshot_freeze_blocks_ordinary_snapshot_start() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.update_state.status = crate::models::UpdateStatus::Running {
            phase: crate::models::UpdatePhase::Preparing,
            message: "Preparing".to_string(),
        };

        let result = app.take_monitor_snapshot();

        assert!(result.is_err());
        assert!(!app.save_monitor.snapshot_in_flight);
    }

    #[test]
    fn save_monitor_starts_pending_on_first_change() {
        let dir =
            std::env::temp_dir().join(format!("hallinta_monitor_first_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test save dir should be created");
        std::fs::write(dir.join("player.txt"), b"x").expect("test file should write");

        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.save_monitor.running = true;
        app.save_monitor.last_known_mtime = 0;

        app.check_save_monitor_changes();

        assert!(app.save_monitor.pending_change_since.is_some());
        assert!(app.save_monitor.last_write_at.is_some());
        assert!(app.save_monitor.last_known_mtime > 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn process_auto_monitor_off_clears_match_latch() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.save_monitor_settings.auto_monitor_mode = AutoMonitorMode::Off;
        app.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = false;
        app.save_monitor.auto_monitor_match_active = true;

        app.poll_auto_monitor_from_processes(Instant::now());

        assert!(!app.save_monitor.auto_monitor_match_active);
    }

    #[test]
    fn auto_monitor_intro_enable_starts_generated_session_without_prompt() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = true;
        app.settings.save_monitor_settings.auto_monitor_mode = AutoMonitorMode::Off;
        // The intro is opened only after process polling latches the active match.
        app.save_monitor.auto_monitor_match_active = true;

        app.finish_auto_monitor_intro(true);

        assert!(
            !app.settings
                .save_monitor_settings
                .auto_monitor_intro_pending
        );
        assert_eq!(
            app.settings.save_monitor_settings.auto_monitor_mode,
            AutoMonitorMode::Always
        );
        assert!(app.save_monitor.auto_monitor_match_active);
        assert!(app.save_monitor.is_running());
        assert!(app.save_monitor.current_session.is_some());
        assert!(app.active_modal.is_none());
    }

    #[test]
    fn auto_monitor_intro_decline_leaves_off_without_starting() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = true;
        app.settings.save_monitor_settings.auto_monitor_mode = AutoMonitorMode::Off;

        app.finish_auto_monitor_intro(false);

        assert!(
            !app.settings
                .save_monitor_settings
                .auto_monitor_intro_pending
        );
        assert_eq!(
            app.settings.save_monitor_settings.auto_monitor_mode,
            AutoMonitorMode::Off
        );
        assert!(!app.save_monitor.is_running());
    }

    #[test]
    fn manual_monitor_offer_queues_session_check() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.noita_sync_state = crate::models::NoitaSyncState::Live;

        app.offer_save_monitor_start();

        assert!(app.save_monitor.auto_monitor_match_active);
        let result = app
            .task_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("manual offer should check stopped sessions");
        assert!(matches!(
            result,
            crate::tasks::TaskResult::SessionCheckComplete(_)
        ));
    }

    #[test]
    fn running_session_blocks_auto_monitor_intro_and_start() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = true;
        app.settings.save_monitor_settings.auto_monitor_mode = AutoMonitorMode::Always;
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(test_session());
        app.save_monitor.auto_monitor_match_active = false;
        app.save_monitor.last_process_probe = None;
        app.active_modal = None;

        app.poll_auto_monitor_from_processes(Instant::now());

        assert!(
            app.active_modal.is_none(),
            "must not re-ask while a session runs"
        );
        assert!(
            app.save_monitor.auto_monitor_match_active,
            "running session should latch match so we do not fire again immediately after stop mid-game"
        );
        // Still only one logical session — poll must not try to start another.
        assert!(app.save_monitor.is_running());
        assert!(matches!(
            app.save_monitor
                .current_session
                .as_ref()
                .map(|s| s.id.as_str()),
            Some("session-id")
        ));
    }

    #[test]
    fn finish_intro_enable_skips_start_when_already_running() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(test_session());
        app.settings
            .save_monitor_settings
            .auto_monitor_intro_pending = true;

        app.finish_auto_monitor_intro(true);

        assert!(
            !app.settings
                .save_monitor_settings
                .auto_monitor_intro_pending
        );
        assert_eq!(
            app.settings.save_monitor_settings.auto_monitor_mode,
            AutoMonitorMode::Always
        );
        assert_eq!(
            app.save_monitor
                .current_session
                .as_ref()
                .map(|s| s.id.as_str()),
            Some("session-id"),
            "must not replace the active session"
        );
    }
}
