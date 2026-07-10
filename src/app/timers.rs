use super::HallintaApp;
use crate::core::{file_watcher, logging, mods};
use crate::models::{ModEntry, Modal};
use eframe::egui;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

impl HallintaApp {
    // ── Timer Checks ───────────────────────────────────────────────────

    pub(super) fn check_timers(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        // File watcher (every 5 seconds — paused while unfocused, eagerly fires on regain)
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let regained_focus = focused && !self.was_focused;
        self.was_focused = focused;

        if !self.save_monitor.is_running()
            && self.active_modal.is_none()
            && self.file_watcher.pending_external_mods.is_some()
        {
            self.show_pending_external_mods_after_monitor();
        }

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

        // Save monitor (change-detection based)
        if self.save_monitor.is_running() && self.can_start_monitor_snapshot() {
            let should_scan = self
                .save_monitor
                .last_scan
                .is_none_or(|t| now.duration_since(t) > Duration::from_secs(2));
            if should_scan {
                self.save_monitor.last_scan = Some(now);
                self.check_save_monitor_changes();
            }
            // Snapshot when backup delay since first change has elapsed AND
            // writes have been stable briefly (Noita finished the latest burst).
            if let Some(change_time) = self.save_monitor.pending_change_since {
                let delay_elapsed = now.duration_since(change_time)
                    > monitor_backup_delay(
                        self.settings.save_monitor_settings.backup_delay_minutes,
                    );
                let writes_stable = self
                    .save_monitor
                    .last_write_at
                    .map(|t| now.duration_since(t) > MONITOR_WRITE_STABILITY)
                    .unwrap_or(true);
                if delay_elapsed && writes_stable {
                    self.save_monitor.pending_change_since = None;
                    self.save_monitor.last_write_at = None;
                    self.take_monitor_snapshot();
                }
            }
        }

        // Request periodic repaint for timers
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn check_external_changes(&mut self) {
        let noita_dir = self.settings.noita_dir.clone();
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
            {
                self.defer_or_prompt_external_mods(file_mods);
            }
        }
    }

    pub(super) fn defer_or_prompt_external_mods(&mut self, file_mods: Vec<ModEntry>) {
        if mods_equal(&self.current_mods, &file_mods) {
            self.file_watcher.pending_external_mods = None;
            return;
        }

        let _ = logging::log(
            "INFO",
            &format!(
                "External mod_config.xml change detected ({} mods on disk vs {} in memory)",
                file_mods.len(),
                self.current_mods.len()
            ),
            "FileWatcher",
        );

        if self.save_monitor.is_running() {
            self.file_watcher.pending_external_mods = Some(file_mods);
            return;
        }

        let summary = build_external_mod_change_summary(&self.current_mods, &file_mods);
        self.active_modal = Some(Modal::ExternalModChanges { file_mods, summary });
    }

    pub(super) fn show_pending_external_mods_after_monitor(&mut self) {
        if self.save_monitor.is_running() {
            return;
        }
        if self.active_modal.is_some() {
            return;
        }

        if let Some(file_mods) = self.file_watcher.pending_external_mods.take() {
            self.defer_or_prompt_external_mods(file_mods);
        }
    }
}

/// Minimum time after the first change in a cycle before taking a snapshot.
fn monitor_backup_delay(minutes: u64) -> Duration {
    Duration::from_secs(minutes.clamp(1, 120) * 60)
}

/// Quiet period after the latest write so we avoid zipping mid-save.
const MONITOR_WRITE_STABILITY: Duration = Duration::from_secs(5);

pub(crate) fn build_external_mod_change_summary(
    current: &[ModEntry],
    disk: &[ModEntry],
) -> crate::models::ExternalModChangeSummary {
    let current_enabled = current.iter().filter(|m| m.enabled).count();
    let disk_enabled = disk.iter().filter(|m| m.enabled).count();
    let current_by_key = mods_by_key(current);
    let disk_by_key = mods_by_key(disk);

    let added = disk_by_key
        .keys()
        .filter(|key| !current_by_key.contains_key(*key))
        .count();
    let removed = current_by_key
        .keys()
        .filter(|key| !disk_by_key.contains_key(*key))
        .count();
    let enabled_changed = current_by_key
        .iter()
        .filter_map(|(key, current_mod)| {
            disk_by_key.get(key).map(|disk_mod| (current_mod, disk_mod))
        })
        .filter(|(current_mod, disk_mod)| current_mod.enabled != disk_mod.enabled)
        .count();

    let current_common_order: Vec<String> = current
        .iter()
        .map(mod_identity_key)
        .filter(|key| disk_by_key.contains_key(key))
        .collect();
    let disk_common_order: Vec<String> = disk
        .iter()
        .map(mod_identity_key)
        .filter(|key| current_by_key.contains_key(key))
        .collect();

    crate::models::ExternalModChangeSummary {
        current_total: current.len(),
        disk_total: disk.len(),
        current_enabled,
        disk_enabled,
        added,
        removed,
        enabled_changed,
        order_changed: current_common_order != disk_common_order,
    }
}

fn mods_by_key(mods: &[ModEntry]) -> BTreeMap<String, &ModEntry> {
    mods.iter().map(|m| (mod_identity_key(m), m)).collect()
}

fn mod_identity_key(mod_entry: &ModEntry) -> String {
    let workshop_id = mod_entry.workshop_id.trim();
    if workshop_id.is_empty() || workshop_id == "0" {
        format!("local:{}", mod_entry.name)
    } else {
        format!("workshop:{}", workshop_id)
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{mod_entry, test_app};
    use super::*;
    use crate::models::Modal;

    #[test]
    fn external_mods_are_deferred_while_monitoring() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.save_monitor.running = true;

        app.defer_or_prompt_external_mods(vec![mod_entry("Alpha", false, "1")]);

        assert!(
            app.active_modal.is_none(),
            "monitoring should not interrupt the UI with an external-change modal"
        );
        let pending = app
            .file_watcher
            .pending_external_mods
            .as_ref()
            .expect("external mods should be remembered for post-monitor review");
        assert_eq!(pending.len(), 1);
        assert!(!pending[0].enabled);
    }

    #[test]
    fn pending_external_mods_prompt_after_monitor_stops() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.file_watcher.pending_external_mods = Some(vec![mod_entry("Alpha", false, "1")]);

        app.show_pending_external_mods_after_monitor();

        assert!(
            app.file_watcher.pending_external_mods.is_none(),
            "pending changes should be consumed when review is shown"
        );
        match app.active_modal {
            Some(Modal::ExternalModChanges {
                ref file_mods,
                ref summary,
            }) => {
                assert_eq!(file_mods.len(), 1);
                assert_eq!(summary.enabled_changed, 1);
            }
            other => panic!("expected external changes modal, got {other:?}"),
        }
    }

    #[test]
    fn matching_external_mods_clear_pending_without_prompt() {
        let current = vec![mod_entry("Alpha", true, "1")];
        let (_runtime, mut app) = test_app(current.clone());
        app.file_watcher.pending_external_mods = Some(vec![mod_entry("Alpha", false, "1")]);

        app.defer_or_prompt_external_mods(current);

        assert!(app.active_modal.is_none());
        assert!(app.file_watcher.pending_external_mods.is_none());
    }

    #[test]
    fn pending_external_mods_wait_when_another_modal_is_open() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.file_watcher.pending_external_mods = Some(vec![mod_entry("Alpha", false, "1")]);
        app.active_modal = Some(Modal::Info {
            title: "Busy".to_string(),
            message: "Finish this first".to_string(),
        });

        app.show_pending_external_mods_after_monitor();

        assert!(
            app.file_watcher.pending_external_mods.is_some(),
            "pending changes should wait behind existing modals"
        );
        assert!(matches!(app.active_modal, Some(Modal::Info { .. })));
    }

    #[test]
    fn external_mod_change_summary_counts_common_differences() {
        let current = vec![
            mod_entry("Alpha", true, "1"),
            mod_entry("Beta", false, "2"),
            mod_entry("Gamma", true, "3"),
        ];
        let disk = vec![
            mod_entry("Beta", true, "2"),
            mod_entry("Alpha", true, "1"),
            mod_entry("Delta", true, "4"),
        ];

        let summary = build_external_mod_change_summary(&current, &disk);

        assert_eq!(summary.current_total, 3);
        assert_eq!(summary.disk_total, 3);
        assert_eq!(summary.added, 1);
        assert_eq!(summary.removed, 1);
        assert_eq!(summary.enabled_changed, 1);
        assert!(summary.order_changed);
    }

    #[test]
    fn monitor_backup_delay_uses_minutes_with_safe_bounds() {
        assert_eq!(monitor_backup_delay(3), Duration::from_secs(180));
        assert_eq!(monitor_backup_delay(0), Duration::from_secs(60));
    }

    #[test]
    fn monitor_snapshot_waits_for_manual_backup_in_progress() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.backup_state.in_progress = true;

        assert!(!app.can_start_monitor_snapshot());
    }

    #[test]
    fn monitor_pending_change_survives_while_manual_backup_is_busy() {
        let ctx = egui::Context::default();
        let (_runtime, mut app) = test_app(Vec::new());
        app.save_monitor.running = true;
        app.backup_state.in_progress = true;
        app.save_monitor.pending_change_since =
            Some(Instant::now() - monitor_backup_delay(3) - Duration::from_secs(1));
        app.save_monitor.last_write_at =
            Some(Instant::now() - MONITOR_WRITE_STABILITY - Duration::from_secs(1));

        app.check_timers(&ctx);

        assert!(app.save_monitor.pending_change_since.is_some());
    }

    #[test]
    fn monitor_waits_for_write_stability_even_after_backup_delay() {
        let ctx = egui::Context::default();
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(crate::models::SessionInfo {
            id: "session-id".to_string(),
            name: "Session".to_string(),
            preset_name: "Default".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: None,
            status: crate::models::SessionStatus::Monitoring,
            snapshot_count: 0,
            locked_mods: Vec::new(),
            folder_name: "session-id".to_string(),
        });
        // Delay fully elapsed, but a write just happened — must not snapshot yet.
        app.save_monitor.pending_change_since =
            Some(Instant::now() - monitor_backup_delay(3) - Duration::from_secs(1));
        app.save_monitor.last_write_at = Some(Instant::now());

        app.check_timers(&ctx);

        assert!(
            app.save_monitor.pending_change_since.is_some(),
            "should wait for short write stability after delay"
        );
        assert!(!app.save_monitor.snapshot_in_flight);
    }

    #[test]
    fn monitor_snapshots_when_delay_and_stability_are_met() {
        let ctx = egui::Context::default();
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.save_monitor.running = true;
        app.save_monitor.current_session = Some(crate::models::SessionInfo {
            id: "session-id".to_string(),
            name: "Session".to_string(),
            preset_name: "Default".to_string(),
            started_at: "2026-01-01T00:00:00Z".to_string(),
            ended_at: None,
            status: crate::models::SessionStatus::Monitoring,
            snapshot_count: 0,
            locked_mods: Vec::new(),
            folder_name: "session-id".to_string(),
        });
        app.save_monitor.pending_change_since =
            Some(Instant::now() - monitor_backup_delay(3) - Duration::from_secs(1));
        app.save_monitor.last_write_at =
            Some(Instant::now() - MONITOR_WRITE_STABILITY - Duration::from_secs(1));

        app.check_timers(&ctx);

        assert!(app.save_monitor.pending_change_since.is_none());
        assert!(app.save_monitor.last_write_at.is_none());
        assert!(app.save_monitor.snapshot_in_flight);
    }
}
