use super::{HallintaApp, UpdateHandoff};
use crate::core::{logging, platform, updater};
use crate::models::{MonitorResume, UpdateInfo, UpdatePhase, UpdateStatus};
use crate::tasks::TaskResult;
use eframe::egui;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
                .unwrap_or_else(|e| Err(format!("Update check task failed: {e}")));
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
        if let Err(error) = updater::helper_creation_flags() {
            self.fail_update(error, false);
            return;
        }
        let original = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.fail_update(format!("Could not locate Hallinta.exe: {error}"), false);
                return;
            }
        };
        let (staging, _, _) = match updater::unique_paths(&original) {
            Ok(paths) => paths,
            Err(error) => {
                self.fail_update(error, false);
                return;
            }
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.update_state.generation = self.update_state.generation.wrapping_add(1);
        let generation = self.update_state.generation;
        self.update_state.snapshot_freeze = true;
        self.update_state.cancel_token = Some(cancel.clone());
        self.update_state.staged_path = None;
        self.update_state.monitor_resume =
            self.save_monitor
                .current_session
                .as_ref()
                .map(|session| MonitorResume {
                    preset_name: session.preset_name.clone(),
                    session_id: session.id.clone(),
                });
        self.update_state.status = UpdateStatus::Running {
            phase: UpdatePhase::Downloading,
            message: format!("Downloading Hallinta v{}…", info.version),
            progress: Some(0.0),
            can_cancel: true,
        };
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let progress_tx = tx.clone();
            let destination = staging.clone();
            let result = tokio::task::spawn_blocking(move || {
                updater::download(&info, &destination, &cancel, |downloaded, total| {
                    let _ = progress_tx.send(TaskResult::UpdateDownloadProgress {
                        generation,
                        downloaded,
                        total,
                    });
                })?;
                Ok(destination)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Update download task failed: {e}")));
            let _ = tx.send(TaskResult::UpdateDownloadComplete { generation, result });
        });
    }

    pub fn cancel_update(&mut self) {
        let can_cancel = matches!(
            self.update_state.status,
            UpdateStatus::Running {
                can_cancel: true,
                ..
            }
        );
        if can_cancel {
            if let Some(cancel) = &self.update_state.cancel_token {
                cancel.store(true, Ordering::Release);
            }
            self.update_state.status = UpdateStatus::Cancelling;
        }
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
            Ok(Some(info)) => self.update_state.status = UpdateStatus::Available(info),
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

    pub(super) fn handle_update_progress(&mut self, generation: u64, downloaded: u64, total: u64) {
        if generation != self.update_state.generation {
            return;
        }
        if let UpdateStatus::Running {
            phase: UpdatePhase::Downloading,
            message,
            progress,
            ..
        } = &mut self.update_state.status
        {
            *message = format!(
                "Downloading update… {:.1} / {:.1} MB",
                downloaded as f64 / 1_048_576.0,
                total as f64 / 1_048_576.0
            );
            *progress = (total > 0).then_some(downloaded as f32 / total as f32);
        }
    }

    pub(super) fn handle_update_download(
        &mut self,
        generation: u64,
        result: Result<std::path::PathBuf, String>,
    ) {
        if generation != self.update_state.generation {
            if let Ok(path) = result {
                let _ = fs::remove_file(path);
            }
            return;
        }
        self.update_state.cancel_token = None;
        match result {
            Ok(path) => {
                self.update_state.staged_path = Some(path);
                self.update_state.status = UpdateStatus::Running {
                    phase: UpdatePhase::WaitingForSnapshot,
                    message: "Download verified. Preparing a safe restart…".to_string(),
                    progress: Some(1.0),
                    can_cancel: false,
                };
            }
            Err(error) if error == "Update cancelled." => self.reset_update(),
            Err(error) => self.fail_update(error, true),
        }
    }

    pub(super) fn poll_update(&mut self, ctx: &egui::Context) {
        if matches!(
            self.update_state.status,
            UpdateStatus::Running {
                phase: UpdatePhase::WaitingForSnapshot,
                ..
            }
        ) && !self.save_monitor.snapshot_in_flight
        {
            if self.save_monitor.is_running() {
                if self.backup_state.in_progress || self.backup_state.restoring {
                    return;
                }
                match self.take_update_final_snapshot() {
                    Ok(request_id) => {
                        self.update_state.pending_final_snapshot_id = Some(request_id);
                        self.update_state.status = UpdateStatus::Running {
                            phase: UpdatePhase::Snapshotting,
                            message: "Saving the final monitor snapshot…".to_string(),
                            progress: None,
                            can_cancel: false,
                        };
                    }
                    Err(error) => self.fail_update(
                        format!("Hallinta stayed open because the final snapshot could not start: {error}"),
                        true,
                    ),
                }
            } else {
                self.launch_update_helper();
            }
        }

        self.poll_helper_ack(ctx);
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
        self.launch_update_helper();
    }

    fn launch_update_helper(&mut self) {
        let Some(staging) = self.update_state.staged_path.clone() else {
            self.fail_update("The verified staged update is missing.".to_string(), true);
            return;
        };
        self.update_state.status = UpdateStatus::Running {
            phase: UpdatePhase::PreparingRestart,
            message: "Preparing the safe restart helper…".to_string(),
            progress: None,
            can_cancel: false,
        };
        let original = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.fail_update(format!("Could not locate Hallinta.exe: {error}"), false);
                return;
            }
        };
        let (_, helper, rollback) = match updater::unique_paths(&original) {
            Ok(paths) => paths,
            Err(error) => {
                self.fail_update(error, false);
                return;
            }
        };
        let ack = staging.with_extension("helper-ack");
        let ready = staging.with_extension("ready");
        let handoff = staging.with_extension("handoff");
        if let Err(error) = fs::write(&handoff, original.to_string_lossy().as_bytes()) {
            self.fail_update(
                format!("Could not create the update handoff barrier: {error}"),
                true,
            );
            return;
        }
        let result = (|| -> Result<std::process::Child, String> {
            fs::copy(&original, &helper)
                .map_err(|e| format!("Could not create the update helper: {e}"))?;
            let creation_time = updater::process_creation_time(std::process::id())?;
            let flags = updater::helper_creation_flags()?;
            let mut command = std::process::Command::new(&helper);
            command
                .arg("--hallinta-update-helper")
                .arg(&original)
                .arg(&staging)
                .arg(&rollback)
                .arg(std::process::id().to_string())
                .arg(creation_time.to_string())
                .arg(updater::sha256_file(&staging)?)
                .arg(&ack)
                .arg(&ready)
                .arg(&handoff)
                .arg(
                    self.update_state
                        .monitor_resume
                        .as_ref()
                        .map_or("", |resume| resume.preset_name.as_str()),
                )
                .arg(
                    self.update_state
                        .monitor_resume
                        .as_ref()
                        .map_or("", |resume| resume.session_id.as_str()),
                );
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                command.creation_flags(flags);
            }
            command
                .spawn()
                .map_err(|e| format!("Could not launch the update helper: {e}"))
        })();
        match result {
            Ok(child) => {
                let helper_identity = updater::process_creation_time(child.id())
                    .map(|created| format!("{}\n{}\n{created}", original.display(), child.id()));
                if let Err(error) = helper_identity.and_then(|identity| {
                    fs::write(&handoff, identity).map_err(|write_error| {
                        format!("Could not arm update handoff: {write_error}")
                    })
                }) {
                    let mut child = child;
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(&handoff);
                    let _ = fs::remove_file(&helper);
                    self.fail_update(error, true);
                    return;
                }
                self.update_handoff = Some(UpdateHandoff {
                    child,
                    ack_path: ack,
                    staging_path: staging,
                    helper_path: helper,
                    rollback_path: rollback,
                    ready_path: ready,
                    handoff_path: handoff,
                    started: std::time::Instant::now(),
                });
                self.update_state.status = UpdateStatus::Running {
                    phase: UpdatePhase::WaitingForHelper,
                    message: "Waiting for the update helper…".to_string(),
                    progress: None,
                    can_cancel: false,
                };
            }
            Err(error) => {
                let _ = fs::remove_file(helper);
                let _ = fs::remove_file(handoff);
                self.fail_update(error, true);
            }
        }
    }

    fn poll_helper_ack(&mut self, ctx: &egui::Context) {
        let Some(handoff) = &mut self.update_handoff else {
            return;
        };
        let child_exited = match handoff.child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                self.cleanup_handoff();
                self.fail_update(
                    format!("Could not inspect the update helper: {error}"),
                    true,
                );
                return;
            }
        };
        if let Some(status) = child_exited {
            let message = format!("The update helper exited before takeover ({status}).");
            self.cleanup_handoff();
            self.fail_update(message, true);
            return;
        }
        if handoff.ack_path.exists() {
            let acknowledged_pid = fs::read_to_string(&handoff.ack_path).unwrap_or_default();
            let expected_identity = updater::process_creation_time(handoff.child.id())
                .map(|created| format!("{}:{created}", handoff.child.id()));
            if expected_identity.as_deref() != Ok(acknowledged_pid.trim()) {
                self.cleanup_handoff();
                self.fail_update(
                    "The update helper acknowledgement was invalid.".to_string(),
                    true,
                );
                return;
            }
            self.update_state.update_restart_shutdown = true;
            self.update_state.status = UpdateStatus::Running {
                phase: UpdatePhase::Restarting,
                message: "Restarting into the update…".to_string(),
                progress: None,
                can_cancel: false,
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if handoff.started.elapsed() > std::time::Duration::from_secs(10) {
            self.cleanup_handoff();
            self.fail_update(
                "The update helper did not acknowledge startup in time.".to_string(),
                true,
            );
        }
    }

    fn cleanup_handoff(&mut self) {
        if let Some(mut handoff) = self.update_handoff.take() {
            let _ = handoff.child.kill();
            let _ = handoff.child.wait();
            for path in [
                handoff.ack_path,
                handoff.staging_path,
                handoff.helper_path,
                handoff.rollback_path,
                handoff.ready_path,
                handoff.handoff_path,
            ] {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn fail_update(&mut self, message: String, retryable: bool) {
        let _ = logging::log("ERROR", &message, "Updater");
        if let Some(path) = self.update_state.staged_path.take() {
            let _ = fs::remove_file(path);
        }
        self.update_state.cancel_token = None;
        self.update_state.snapshot_freeze = false;
        self.update_state.pending_final_snapshot_id = None;
        if !self.save_monitor.is_running()
            && let Some(resume) = self.update_state.monitor_resume.take()
        {
            self.resume_monitor_session_for(&resume.preset_name, &resume.session_id);
        }
        self.update_state.status = UpdateStatus::Failed { message, retryable };
    }

    pub fn dismiss_update_status(&mut self) {
        if !self.update_state.is_locked() {
            self.update_state.status = UpdateStatus::Idle;
        }
    }

    pub(super) fn signal_startup_ready(&mut self) {
        if let Some(path) = self.startup_ready_path.take()
            && let Err(error) = fs::write(&path, b"ready")
        {
            let _ = logging::log(
                "ERROR",
                &format!("Could not signal update readiness: {error}"),
                "Updater",
            );
        }
    }

    fn reset_update(&mut self) {
        if let Some(path) = self.update_state.staged_path.take() {
            let _ = fs::remove_file(path);
        }
        self.update_state.cancel_token = None;
        self.update_state.snapshot_freeze = false;
        self.update_state.pending_final_snapshot_id = None;
        self.update_state.monitor_resume = None;
        self.update_state.status = UpdateStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_app;
    use crate::core::platform;
    use crate::models::UpdateStatus;

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
}
