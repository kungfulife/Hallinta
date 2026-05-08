use super::HallintaApp;
use crate::core::{backup, file_watcher, logging, mods};
use crate::models::{ConfirmAction, ModEntry, Modal};
use eframe::egui;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl HallintaApp {
    // ── Timer Checks ───────────────────────────────────────────────────

    pub(super) fn check_timers(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        // Log flush (every 5 seconds)
        if now.duration_since(self.last_log_flush) > Duration::from_secs(5) {
            let _ = logging::flush_log_buffer();
            self.last_log_flush = now;
        }

        // File watcher (every 5 seconds — paused while unfocused, eagerly fires on regain)
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let regained_focus = focused && !self.was_focused;
        self.was_focused = focused;

        let should_check = focused
            && (regained_focus
                || self
                    .file_watcher
                    .last_check
                    .is_none_or(|t| now.duration_since(t) > self.file_watcher.check_interval));
        if should_check && self.active_modal.is_none() {
            self.file_watcher.last_check = Some(now);
            self.check_external_changes();
        }

        // Backup cleanup (every 6 hours, plus once on first frame)
        let cleanup_interval = Duration::from_secs(6 * 60 * 60);
        let should_cleanup = self
            .last_backup_cleanup
            .is_none_or(|t| now.duration_since(t) > cleanup_interval);
        if should_cleanup {
            self.last_backup_cleanup = Some(now);
            let days = self.settings.backup_settings.auto_delete_days;
            if days > 0 {
                self.async_runtime.spawn(async move {
                    let _ = tokio::task::spawn_blocking(move || backup::cleanup_old_backups(days))
                        .await;
                });
            }
        }

        // Auto-backup scheduler
        let interval_min = self.settings.backup_settings.backup_interval_minutes;
        if interval_min > 0 && !self.backup_state.in_progress && !self.backup_state.restoring {
            let interval = Duration::from_secs(interval_min as u64 * 60);
            let should_backup = self
                .last_auto_backup
                .is_none_or(|t| now.duration_since(t) > interval);
            if should_backup {
                self.last_auto_backup = Some(now);
                self.start_auto_backup();
            }
        }

        // Save monitor (change-detection based)
        if self.save_monitor.is_running() && !self.save_monitor.snapshot_in_flight {
            let should_scan = self
                .save_monitor
                .last_scan
                .is_none_or(|t| now.duration_since(t) > Duration::from_secs(2));
            if should_scan {
                self.save_monitor.last_scan = Some(now);
                self.check_save_monitor_changes();
            }
            // Wait 5 seconds after change detected for stability
            if let Some(change_time) = self.save_monitor.pending_change_since
                && now.duration_since(change_time) > Duration::from_secs(5)
            {
                self.save_monitor.pending_change_since = None;
                self.take_monitor_snapshot();
            }
        }

        // Request periodic repaint for timers
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn check_external_changes(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let dir = PathBuf::from(&noita_dir);
        if let Some(new_mtime) =
            file_watcher::check_for_external_changes(&dir, self.file_watcher.last_modified_time)
        {
            self.file_watcher.last_modified_time = new_mtime;

            if let Ok(xml) = mods::read_mod_config(&dir)
                && let Ok(file_mods) = mods::parse_mods_from_xml(&xml)
                && !mods_equal(&self.current_mods, &file_mods)
            {
                let _ = logging::log(
                    "INFO",
                    &format!(
                        "External mod_config.xml change detected ({} mods on disk vs {} in memory)",
                        file_mods.len(),
                        self.current_mods.len()
                    ),
                    "FileWatcher",
                );
                self.active_modal = Some(Modal::Confirm {
                    message: format!(
                        "mod_config.xml was modified externally and no longer matches your \"{}\" preset.",
                        self.selected_preset
                    ),
                    confirm_text: "Accept External Changes".to_string(),
                    cancel_text: "Keep Current Preset".to_string(),
                    action: ConfirmAction::AcceptExternalChanges(file_mods),
                    cancel_action: Some(ConfirmAction::KeepCurrentPreset),
                });
            }
        }
    }
}
fn mods_equal(a: &[ModEntry], b: &[ModEntry]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| {
        x.name == y.name
            && x.enabled == y.enabled
            && x.workshop_id == y.workshop_id
            && x.settings_fold_open == y.settings_fold_open
    })
}
