use super::HallintaApp;
use crate::core::{logging, platform, settings, updater};
use crate::models::{MonitorResume, UpdateInfo, UpdatePhase, UpdateStatus};
use crate::tasks::TaskResult;
use eframe::egui;

impl HallintaApp {
    pub fn check_for_updates(&mut self, manual: bool) {
        if self.update_state.is_locked()
            || matches!(self.update_state.status, UpdateStatus::Checking { .. })
        {
            return;
        }
        if !platform::is_dist_build() {
            if manual {
                self.update_state.status = UpdateStatus::Failed {
                    message:
                        "Automatic updates are enabled only in official GitHub release builds."
                            .to_string(),
                    retryable: false,
                };
            }
            return;
        }
        self.update_state.generation = self.update_state.generation.wrapping_add(1);
        let generation = self.update_state.generation;
        self.update_state.status = UpdateStatus::Checking { manual };
        let tx = self.task_tx.clone();
        let current = platform::get_version();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || updater::check_latest(&current))
                .await
                .unwrap_or_else(|error| Err(format!("Update check task failed: {error}")));
            let _ = tx.send(TaskResult::UpdateCheckComplete {
                generation,
                manual,
                result,
            });
        });
    }

    pub fn begin_update(&mut self, info: UpdateInfo) {
        if self.update_state.is_locked() {
            return;
        }
        if self.settings.dismissed_update_version.is_some() {
            self.settings.dismissed_update_version = None;
            if let Err(error) = settings::save_settings(&self.settings) {
                let _ = logging::log(
                    "WARN",
                    &format!("Could not clear dismissed update version: {error}"),
                    "Updater",
                );
            }
        }
        self.update_state.generation = self.update_state.generation.wrapping_add(1);
        self.update_state.selected_version = Some(info.version);
        self.update_state.monitor_resume =
            self.save_monitor
                .current_session
                .as_ref()
                .map(|session| MonitorResume {
                    preset_name: session.preset_name.clone(),
                    session_id: session.id.clone(),
                });
        self.update_state.status = UpdateStatus::Running {
            phase: UpdatePhase::Preparing,
            message: "Preparing Hallinta for the update…".to_string(),
        };
    }

    pub(super) fn handle_update_check(
        &mut self,
        generation: u64,
        manual: bool,
        result: Result<Option<UpdateInfo>, String>,
    ) {
        if generation != self.update_state.generation {
            return;
        }
        match result {
            Ok(Some(info)) => {
                // Auto-prompt is quiet for a dismissed version; manual check always offers it.
                if !manual
                    && self.settings.dismissed_update_version.as_deref()
                        == Some(info.version.as_str())
                {
                    self.update_state.status = UpdateStatus::Idle;
                } else {
                    self.update_state.status = UpdateStatus::Available(info);
                }
            }
            Ok(None) => {
                self.update_state.status = if manual {
                    UpdateStatus::Failed {
                        message: "Hallinta is up to date.".to_string(),
                        retryable: false,
                    }
                } else {
                    UpdateStatus::Idle
                };
            }
            Err(error) if manual => self.fail_update(error, true),
            Err(error) => {
                let _ = logging::log(
                    "WARN",
                    &format!("Automatic update check skipped: {error}"),
                    "Updater",
                );
                self.update_state.status = UpdateStatus::Idle;
            }
        }
    }

    pub(super) fn poll_update(&mut self, ctx: &egui::Context) {
        if matches!(
            self.update_state.status,
            UpdateStatus::Running {
                phase: UpdatePhase::Preparing,
                ..
            }
        ) && !self.save_monitor.snapshot_in_flight
        {
            if self.backup_state.in_progress || self.backup_state.restoring {
                return;
            }
            if self.save_monitor.is_running() {
                match self.take_update_final_snapshot() {
                    Ok(request_id) => {
                        self.update_state.pending_final_snapshot_id = Some(request_id);
                        self.update_state.status = UpdateStatus::Running {
                            phase: UpdatePhase::Snapshotting,
                            message: "Saving the final monitor snapshot…".to_string(),
                        };
                    }
                    Err(error) => self.fail_update(
                        format!(
                            "Hallinta stayed open because the final snapshot could not start: {error}"
                        ),
                        true,
                    ),
                }
            } else {
                self.start_external_install();
            }
        }

        if self.update_state.restart_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub(super) fn finish_update_snapshot(
        &mut self,
        request_id: u64,
        success: bool,
        error: Option<&str>,
    ) {
        if self.update_state.pending_final_snapshot_id != Some(request_id) {
            return;
        }
        self.update_state.pending_final_snapshot_id = None;
        if !success {
            self.fail_update(
                format!(
                    "Hallinta stayed open because the final monitor snapshot failed: {}",
                    error.unwrap_or("unknown error")
                ),
                true,
            );
            return;
        }
        self.stop_monitor_for_close();
        self.start_external_install();
    }

    fn start_external_install(&mut self) {
        let Some(version) = self.update_state.selected_version.clone() else {
            self.fail_update("The selected update version is missing.".to_string(), false);
            return;
        };
        let generation = self.update_state.generation;
        self.update_state.status = UpdateStatus::Running {
            phase: UpdatePhase::Installing,
            message: format!("Downloading, verifying, and installing Hallinta v{version}…"),
        };
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || updater::install(&version))
                .await
                .unwrap_or_else(|error| Err(format!("Update task failed: {error}")));
            let _ = tx.send(TaskResult::UpdateInstallComplete { generation, result });
        });
    }

    pub(super) fn handle_update_install(&mut self, generation: u64, result: Result<(), String>) {
        if generation != self.update_state.generation {
            return;
        }
        match result {
            Ok(()) => {
                let restart = crate::core::relaunch::RestartIntent {
                    monitor_resume: self.update_state.monitor_resume.clone(),
                };
                if let Ok(mut request) = self.restart_request.lock() {
                    *request = Some(restart);
                } else {
                    self.fail_update("Could not prepare Hallinta to restart.".to_string(), false);
                    return;
                }
                self.update_state.restart_requested = true;
                self.update_state.status = UpdateStatus::Running {
                    phase: UpdatePhase::Restarting,
                    message: "Update installed. Restarting Hallinta…".to_string(),
                };
            }
            Err(error) => self.fail_update(error, true),
        }
    }

    fn fail_update(&mut self, message: String, retryable: bool) {
        let _ = logging::log("ERROR", &message, "Updater");
        self.update_state.pending_final_snapshot_id = None;
        self.update_state.selected_version = None;
        self.update_state.restart_requested = false;
        if let Some(resume) = self.update_state.monitor_resume.take()
            && !self.save_monitor.is_running()
        {
            self.resume_monitor_session_for(&resume.preset_name, &resume.session_id);
        }
        self.update_state.status = UpdateStatus::Failed { message, retryable };
    }

    pub fn dismiss_update_status(&mut self) {
        if self.update_state.is_locked() {
            return;
        }
        if let UpdateStatus::Available(info) = &self.update_state.status {
            self.settings.dismissed_update_version = Some(info.version.clone());
            if let Err(error) = settings::save_settings(&self.settings) {
                let _ = logging::log(
                    "ERROR",
                    &format!("Could not persist dismissed update version: {error}"),
                    "Updater",
                );
            }
        }
        self.update_state.status = UpdateStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use crate::core::platform;
    use crate::models::{SessionInfo, SessionStatus, UpdateInfo, UpdatePhase, UpdateStatus};

    fn session(id: &str) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            name: "Live session".to_string(),
            preset_name: "Default".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: None,
            status: SessionStatus::Monitoring,
            snapshot_count: 2,
            locked_mods: Vec::new(),
            folder_name: id.to_string(),
        }
    }

    #[test]
    fn development_build_update_checks_never_start_network_work() {
        assert!(!platform::is_dist_build());
        let (_runtime, mut app) = test_app(Vec::new());

        app.check_for_updates(false);
        assert_eq!(app.update_state.status, UpdateStatus::Idle);

        app.check_for_updates(true);
        assert_eq!(
            app.update_state.status,
            UpdateStatus::Failed {
                message: "Automatic updates are enabled only in official GitHub release builds."
                    .to_string(),
                retryable: false,
            }
        );
    }

    #[test]
    fn accepted_update_waits_for_final_snapshot_when_monitoring() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(session("live"));

        app.begin_update(UpdateInfo {
            version: "0.8.3".to_string(),
            notes: String::new(),
        });

        assert_eq!(app.update_state.selected_version.as_deref(), Some("0.8.3"));
        assert!(matches!(
            app.update_state.status,
            UpdateStatus::Running {
                phase: UpdatePhase::Preparing,
                ..
            }
        ));
        assert!(app.update_state.monitor_resume.is_some());
    }

    #[test]
    fn failed_final_snapshot_restores_unlocked_failure_state() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.update_state.pending_final_snapshot_id = Some(7);
        app.update_state.selected_version = Some("0.8.3".to_string());
        app.update_state.status = UpdateStatus::Running {
            phase: UpdatePhase::Snapshotting,
            message: "Saving".to_string(),
        };

        app.finish_update_snapshot(7, false, Some("disk full"));

        assert!(!app.update_state.is_locked());
        assert!(matches!(
            app.update_state.status,
            UpdateStatus::Failed { ref message, .. } if message.contains("disk full")
        ));
    }

    #[test]
    fn dismiss_remembers_version_and_quiets_automatic_checks_only() {
        let (_runtime, mut app) = test_app(Vec::new());
        let info = UpdateInfo {
            version: "0.9.0".to_string(),
            notes: "notes".to_string(),
        };
        app.update_state.status = UpdateStatus::Available(info.clone());

        app.dismiss_update_status();

        assert_eq!(app.update_state.status, UpdateStatus::Idle);
        assert_eq!(
            app.settings.dismissed_update_version.as_deref(),
            Some("0.9.0")
        );

        app.handle_update_check(app.update_state.generation, false, Ok(Some(info.clone())));
        assert_eq!(app.update_state.status, UpdateStatus::Idle);

        app.handle_update_check(app.update_state.generation, true, Ok(Some(info.clone())));
        assert_eq!(app.update_state.status, UpdateStatus::Available(info));

        let newer = UpdateInfo {
            version: "0.9.1".to_string(),
            notes: String::new(),
        };
        app.handle_update_check(app.update_state.generation, false, Ok(Some(newer.clone())));
        assert_eq!(app.update_state.status, UpdateStatus::Available(newer));
    }

    #[test]
    fn accepting_update_clears_dismissed_version() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.dismissed_update_version = Some("0.9.0".to_string());

        app.begin_update(UpdateInfo {
            version: "0.9.0".to_string(),
            notes: String::new(),
        });

        assert!(app.settings.dismissed_update_version.is_none());
    }
}
