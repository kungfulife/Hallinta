use crate::core::{
    backup, file_watcher, gallery, logging, mods, platform, presets, save_monitor, settings,
    workshop,
};
use crate::models::*;
use crate::tasks::TaskResult;
use eframe::egui;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct HallintaApp {
    // Core data
    pub settings: AppSettings,
    pub presets: BTreeMap<String, Vec<ModEntry>>,
    pub current_mods: Vec<ModEntry>,
    pub selected_preset: String,

    // UI state
    pub active_view: View,
    pub search_query: String,
    pub filter_mode: FilterMode,
    pub sort_mode: SortMode,
    pub compact_mode: bool,
    pub dark_mode: bool,

    // Modal state
    pub active_modal: Option<Modal>,

    // Feature state
    pub save_monitor: SaveMonitorState,
    pub backup_state: BackupState,
    pub file_watcher: FileWatcherState,

    // Async coordination
    pub async_runtime: tokio::runtime::Handle,
    pub task_tx: mpsc::Sender<TaskResult>,
    pub task_rx: mpsc::Receiver<TaskResult>,

    // Drag state
    pub drag_state: Option<DragState>,

    // Timers
    last_log_flush: Instant,
    last_auto_backup: Option<Instant>,
    last_backup_cleanup: Option<Instant>,

    // Normal mode window size (for restoring after compact)
    normal_window_size: Option<egui::Vec2>,

    // Deferred min size: viewport commands are processed next frame, so when
    // LOWERING the min we first clear it to (1,1), then apply the real value
    // on the following frame via this field.
    deferred_min_size: Option<egui::Vec2>,

    // Track whether close was requested while monitor is running
    close_requested: bool,

    // Keyboard / focus signals
    pub focus_search_requested: bool,

    // Window focus tracking. File watcher pauses while unfocused and forces an
    // immediate check on regaining focus (matches the JS version's behaviour
    // and avoids polling a directory the user isn't looking at).
    was_focused: bool,
}

impl HallintaApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: tokio::runtime::Handle) -> Self {
        let (task_tx, task_rx) = mpsc::channel();

        // Load settings
        let mut app_settings = settings::load_settings().unwrap_or_else(|e| {
            let _ = logging::log("WARN", &format!("Failed to load settings: {}", e), "App");
            AppSettings {
                noita_dir: String::new(),
                entangled_dir: String::new(),
                dark_mode: false,
                selected_preset: "Default".to_string(),
                version: platform::get_version(),
                log_settings: LogSettings::default(),
                backup_settings: BackupSettings::default(),
                save_monitor_settings: SaveMonitorSettings::default(),
                steam_path: String::new(),
                compact_mode: false,
                ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
                last_filter_mode: String::new(),
                last_sort_mode: String::new(),
            }
        });

        // Load presets
        let app_presets = presets::load_presets().unwrap_or_else(|e| {
            let _ = logging::log("WARN", &format!("Failed to load presets: {}", e), "App");
            let mut m = BTreeMap::new();
            m.insert("Default".to_string(), Vec::new());
            m
        });

        // Dev build sandbox: full copy of real save dirs into dev_data on first run,
        // mod_config.xml sync on subsequent runs. Never touch real files during session.
        let dev_noita_dir = if cfg!(debug_assertions) {
            match platform::seed_dev_sandbox() {
                Ok(msg) => {
                    let _ = logging::log("INFO", &format!("[DEV] {}", msg), "DevData");
                }
                Err(e) => {
                    let _ = logging::log(
                        "WARN",
                        &format!("[DEV] Dev sandbox error: {}", e),
                        "DevData",
                    );
                }
            }
            platform::get_dev_save_dir().ok()
        } else {
            None
        };
        let dev_entangled_dir = if cfg!(debug_assertions) {
            platform::get_dev_entangled_dir().ok()
        } else {
            None
        };

        // Version upgrade check
        let old_version = app_settings.version.clone();
        if settings::check_and_upgrade_version(&mut app_settings).unwrap_or(false) {
            let tx = task_tx.clone();
            let mut s = app_settings.clone();
            if let Some(dev_dir) = &dev_noita_dir {
                s.noita_dir = dev_dir.to_string_lossy().to_string();
            }
            if let Some(dev_dir) = &dev_entangled_dir {
                s.entangled_dir = dev_dir.to_string_lossy().to_string();
            }
            let p = app_presets.clone();
            let new_version = platform::get_version();
            let old_v = old_version.clone();
            rt.spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    backup::create_upgrade_backup(&s, &p, &old_v, &new_version)
                })
                .await
                .unwrap_or_else(|e| Err(format!("Upgrade backup task failed: {}", e)));
                let _ = tx.send(TaskResult::UpgradeBackupComplete(result));
            });
        }

        // Dev mode setup
        let active_noita_dir = dev_noita_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| app_settings.noita_dir.clone());

        // Load mods from active directory
        let selected_preset = app_settings.selected_preset.clone();
        let mut current_mods = app_presets
            .get(&selected_preset)
            .cloned()
            .unwrap_or_default();

        if !active_noita_dir.is_empty() {
            let noita_path = PathBuf::from(&active_noita_dir);
            match mods::read_mod_config(&noita_path) {
                Ok(xml) => match mods::parse_mods_from_xml(&xml) {
                    Ok(file_mods) => {
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Loaded {} mod(s) from {}",
                                file_mods.len(),
                                noita_path.display()
                            ),
                            "Mods",
                        );
                        current_mods = file_mods;
                    }
                    Err(e) => {
                        let _ = logging::log(
                            "ERROR",
                            &format!("Failed to parse mod_config.xml: {}", e),
                            "Mods",
                        );
                    }
                },
                Err(e) => {
                    let _ = logging::log(
                        "WARN",
                        &format!(
                            "mod_config.xml not found at {} — {}",
                            noita_path.display(),
                            e
                        ),
                        "Mods",
                    );
                }
            }
        } else {
            let _ = logging::log(
                "WARN",
                "No Noita save directory configured. Set it in Settings.",
                "Mods",
            );
        }

        // Apply theme and UI scale
        let dark_mode = app_settings.dark_mode;
        let compact_mode = app_settings.compact_mode;
        let scale = app_settings.ui_scale;
        crate::ui::theme::apply_theme(&cc.egui_ctx, dark_mode);
        crate::ui::design::apply_zoom(&cc.egui_ctx, &app_settings);

        // Apply scale-aware window sizing — must happen after apply_zoom
        if compact_mode {
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
                ));
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(
                    crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
                ));
        } else {
            // Reinforce the scaled min size (main.rs sets it too, but this ensures
            // consistency if the viewport builder values were clamped by the OS)
            cc.egui_ctx
                .send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale),
                ));
        }

        // Log system info if configured (now with full detail)
        if app_settings.log_settings.collect_system_info {
            platform::log_system_info_on_startup();
        }

        // File watcher: get initial mtime
        let mut file_watcher_state = FileWatcherState::new();
        if !active_noita_dir.is_empty() {
            let config_path = PathBuf::from(&active_noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                file_watcher_state.last_modified_time = mtime;
            }
        }

        let now = Instant::now();
        let backup_state = BackupState::new();

        let save_monitor_state = SaveMonitorState::new();

        let initial_filter = FilterMode::from_str(&app_settings.last_filter_mode);
        let initial_sort = SortMode::from_str(&app_settings.last_sort_mode);
        let mut app = Self {
            settings: app_settings,
            presets: app_presets,
            current_mods,
            selected_preset,
            active_view: View::ModList,
            search_query: String::new(),
            filter_mode: initial_filter,
            sort_mode: initial_sort,
            compact_mode,
            dark_mode,
            active_modal: None,
            save_monitor: save_monitor_state,
            backup_state,
            file_watcher: file_watcher_state,
            async_runtime: rt,
            task_tx,
            task_rx,
            drag_state: None,
            last_log_flush: now,
            last_auto_backup: None,
            last_backup_cleanup: None,
            normal_window_size: None,
            deferred_min_size: None,
            close_requested: false,
            focus_search_requested: false,
            was_focused: true,
        };

        // Start monitor if configured
        if app.settings.save_monitor_settings.start_in_monitor_mode {
            app.start_save_monitor();
        }

        // Check workshop mods on startup
        app.check_workshop_mods_async();

        // Load backup list async
        app.load_backup_list_async();

        let _ = logging::log("INFO", "Application started", "App");
        let _ = logging::log(
            "INFO",
            &format!(
                "Initial state: preset=\"{}\" mods={} (enabled={}) compact={} dark={} scale={:.2} filter={} sort={}",
                app.selected_preset,
                app.current_mods.len(),
                app.current_mods.iter().filter(|m| m.enabled).count(),
                app.compact_mode,
                app.dark_mode,
                app.settings.ui_scale,
                app.filter_mode.label(),
                app.sort_mode.label(),
            ),
            "App",
        );
        let _ = logging::log(
            "INFO",
            &format!(
                "Backup config: auto_delete_days={} auto_backup_interval_min={}",
                app.settings.backup_settings.auto_delete_days,
                app.settings.backup_settings.backup_interval_minutes
            ),
            "Backup",
        );
        if app.settings.backup_settings.backup_interval_minutes > 0 {
            let _ = logging::log(
                "INFO",
                &format!(
                    "Auto-backup enabled: every {} minute(s)",
                    app.settings.backup_settings.backup_interval_minutes
                ),
                "Backup",
            );
        }
        if app.settings.backup_settings.auto_delete_days > 0 {
            let _ = logging::log(
                "INFO",
                &format!(
                    "Auto-delete enabled: backups older than {} day(s) will be removed",
                    app.settings.backup_settings.auto_delete_days
                ),
                "Backup",
            );
        }
        logging::write_session_marker(&format!("APP_INITIALIZED:v{}", platform::get_version()));
        app
    }

    // ── Timer Checks ───────────────────────────────────────────────────

    fn check_timers(&mut self, ctx: &egui::Context) {
        let now = Instant::now();

        // Log flush (every 5 seconds)
        if now.duration_since(self.last_log_flush) > Duration::from_secs(5) {
            let _ = logging::flush_log_buffer();
            self.last_log_flush = now;
        }

        // File watcher (every 5 seconds — paused while unfocused, eagerly fires on regain)
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let regained_focus = focused && !self.was_focused;
        if regained_focus {
            let _ = logging::log(
                "DEBUG",
                "Window focus regained — running mod_config watch immediately",
                "FileWatcher",
            );
        }
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
            if let Some(change_time) = self.save_monitor.pending_change_since {
                if now.duration_since(change_time) > Duration::from_secs(5) {
                    self.save_monitor.pending_change_since = None;
                    self.take_monitor_snapshot();
                }
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

    // ── Task Result Handling ───────────────────────────────────────────

    fn poll_task_results(&mut self) {
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

    // ── Async Task Dispatchers ────────────────────────────────────────

    pub fn load_backup_list_async(&self) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(backup::list_backups)
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::BackupListLoaded(result));
        });
    }

    pub fn load_sessions_async(&self) {
        let preset = self.selected_preset.clone();
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || save_monitor::list_sessions(&preset))
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionListLoaded(result));
        });
    }

    pub fn load_session_snapshots_async(&self, preset: String, session_id: String) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                save_monitor::list_session_snapshots(&preset, &session_id)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SessionSnapshotsLoaded(result));
        });
    }

    pub fn check_workshop_mods_async(&self) {
        let steam_path = self.settings.steam_path.clone();
        if steam_path.is_empty() {
            return;
        }
        let workshop_ids: Vec<String> = self
            .current_mods
            .iter()
            .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
            .map(|m| m.workshop_id.clone())
            .collect();
        if workshop_ids.is_empty() {
            return;
        }
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                workshop::check_workshop_mods_installed(&workshop_ids, &steam_path)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::WorkshopModsChecked(result));
        });
    }

    pub fn delete_backup_async(&self, filename: String) {
        let tx = self.task_tx.clone();
        let fname = filename.clone();
        self.async_runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || backup::delete_backup(&fname).map(|()| fname))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::BackupDeleted(result));
        });
    }

    pub fn clear_monitor_data_async(&self) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(save_monitor::clear_monitor_data)
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::MonitorDataCleared(result));
        });
    }

    // ── Keyboard Handling ──────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let modal_open = self.active_modal.is_some();
        let monitor_running = self.save_monitor.is_running();
        let backup_busy = self.backup_state.in_progress || self.backup_state.restoring;

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if modal_open {
                if !matches!(self.active_modal, Some(Modal::Progress { .. })) {
                    self.active_modal = None;
                }
            } else if self.active_view == View::Settings {
                self.active_view = View::ModList;
            }
        }

        // Skip remaining shortcuts if a modal/text input is consuming keys
        if modal_open {
            return;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        let ctrl = ctx.input(|i| i.modifiers.command_only());

        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus_search_requested = true;
            self.active_view = View::ModList;
            let _ = logging::log("DEBUG", "Shortcut: focus search (Ctrl+F)", "UI");
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) && !typing {
            self.reload_mods_explicit();
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::B))
            && !backup_busy
            && !monitor_running
            && self.active_view == View::ModList
        {
            let _ = logging::log("INFO", "Shortcut: open backup (Ctrl+B)", "UI");
            self.start_backup_modal();
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::E))
            && !monitor_running
            && !typing
            && self.active_view == View::ModList
        {
            let total = self.current_mods.len();
            for m in &mut self.current_mods {
                m.enabled = true;
            }
            let _ = logging::log(
                "INFO",
                &format!("Shortcut: Enable All (Ctrl+E, {} mods)", total),
                "ModManager",
            );
            self.save_mod_config_and_preset();
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::D))
            && !monitor_running
            && !typing
            && self.active_view == View::ModList
        {
            let total = self.current_mods.len();
            for m in &mut self.current_mods {
                m.enabled = false;
            }
            let _ = logging::log(
                "INFO",
                &format!("Shortcut: Disable All (Ctrl+D, {} mods)", total),
                "ModManager",
            );
            self.save_mod_config_and_preset();
        }
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::Comma)) {
            self.active_view = if self.active_view == View::Settings {
                View::ModList
            } else {
                View::Settings
            };
            let _ = logging::log("DEBUG", "Shortcut: toggle Settings (Ctrl+,)", "UI");
        }
    }

    // ── Close Handling ─────────────────────────────────────────────────

    fn handle_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }

        if self.save_monitor.is_running() && !self.close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_requested = true;
            let _ = logging::log(
                "INFO",
                "Close requested while monitor running — prompting for final snapshot",
                "App",
            );
            self.active_modal = Some(Modal::Confirm {
                message: "Save Monitor is running. Take a final snapshot before closing?"
                    .to_string(),
                confirm_text: "Snapshot & Close".to_string(),
                cancel_text: "Close Without Snapshot".to_string(),
                action: ConfirmAction::ExitWithSnapshot,
                cancel_action: Some(ConfirmAction::ExitWithoutSnapshot),
            });
        }
    }

    // ── Public Actions ─────────────────────────────────────────────────

    pub fn switch_preset(&mut self) {
        self.cancel_drag_if_active();
        if let Some(preset_mods) = self.presets.get(&self.selected_preset) {
            let prev_count = self.current_mods.len();
            let new_count = preset_mods.len();
            let new_enabled = preset_mods.iter().filter(|m| m.enabled).count();
            self.current_mods = preset_mods.clone();
            self.save_mod_config_and_preset();
            let _ = logging::log(
                "INFO",
                &format!(
                    "Switched to preset: {} ({} -> {} mods, {} enabled)",
                    self.selected_preset, prev_count, new_count, new_enabled
                ),
                "PresetManager",
            );
            logging::write_session_marker(&format!("PRESET_SWITCH:{}", self.selected_preset));
            self.check_workshop_mods_async();
        } else {
            let _ = logging::log(
                "WARN",
                &format!(
                    "Preset switch requested but \"{}\" not found",
                    self.selected_preset
                ),
                "PresetManager",
            );
        }
    }

    pub fn save_mod_config_and_preset(&mut self) {
        self.presets
            .insert(self.selected_preset.clone(), self.current_mods.clone());

        let noita_dir = self.get_active_noita_dir();
        if !noita_dir.is_empty() {
            let xml = mods::mods_to_xml(&self.current_mods);
            if let Err(e) = mods::write_mod_config(&PathBuf::from(&noita_dir), &xml) {
                let _ = logging::log(
                    "ERROR",
                    &format!("Failed to write mod_config.xml at {}: {}", noita_dir, e),
                    "ModManager",
                );
            }

            let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                self.file_watcher.last_modified_time = mtime;
            }
        } else {
            let _ = logging::log(
                "WARN",
                "save_mod_config_and_preset called with empty noita_dir — preset still saved",
                "ModManager",
            );
        }

        if let Err(e) = presets::save_presets(&self.presets) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save presets.json: {}", e),
                "PresetManager",
            );
        }
        self.settings.selected_preset = self.selected_preset.clone();
        if let Err(e) = settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save settings.json: {}", e),
                "Settings",
            );
        }
    }

    /// Persist current settings to disk.
    pub fn save_current_settings(&mut self) {
        if let Err(e) = settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save settings.json: {}", e),
                "Settings",
            );
        }
    }

    /// Called when dark_mode checkbox toggles reactively.
    pub fn on_dark_mode_changed(&mut self, ctx: &egui::Context) {
        self.dark_mode = self.settings.dark_mode;
        crate::ui::theme::apply_theme(ctx, self.settings.dark_mode);
        let _ = logging::log(
            "INFO",
            &format!(
                "Theme changed: {}",
                if self.dark_mode { "dark" } else { "light" }
            ),
            "Settings",
        );
        self.save_current_settings();
    }

    /// Called when compact_mode checkbox toggles in settings.
    pub fn on_compact_mode_changed(&mut self, ctx: &egui::Context) {
        let was_compact = self.compact_mode;
        self.compact_mode = self.settings.compact_mode;
        if was_compact != self.compact_mode {
            let _ = logging::log(
                "INFO",
                &format!(
                    "Window mode: {} -> {}",
                    if was_compact { "compact" } else { "normal" },
                    if self.compact_mode {
                        "compact"
                    } else {
                        "normal"
                    }
                ),
                "Settings",
            );
            let scale = self.settings.ui_scale;
            if self.compact_mode {
                let current_size = ctx.input(|i| i.content_rect().size());
                if current_size.x > 500.0 {
                    self.normal_window_size = Some(current_size);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                    crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
                ));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale),
                ));
                let size = self.normal_window_size.unwrap_or_else(|| {
                    crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_NORMAL, scale)
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            }
        }
        self.save_current_settings();
    }

    /// Called when UI scale changes — resize window proportionally and update min size.
    ///
    /// Viewport commands are deferred (processed next frame), so lowering `MinInnerSize`
    /// causes a "one behind" lag — the OS still enforces the old (larger) minimum.
    /// Fix: immediately clear the min to (1,1) so the OS stops clamping, resize the
    /// window in the same batch, then queue the correct minimum for the next frame
    /// via `deferred_min_size`.
    pub fn on_ui_scale_changed(&mut self, ctx: &egui::Context, prev_scale: f32) {
        let scale = self.settings.ui_scale;
        let _ = logging::log(
            "INFO",
            &format!("UI scale changed: {:.2} -> {:.2}", prev_scale, scale),
            "Settings",
        );
        let base_min = if self.compact_mode {
            crate::ui::design::BASE_MIN_COMPACT
        } else {
            crate::ui::design::BASE_MIN_NORMAL
        };

        let new_min = crate::ui::design::scaled_min_size(base_min, scale);

        // Scale the current window size proportionally to the scale change
        let current_size = ctx.input(|i| i.content_rect().size());
        let ratio = scale / prev_scale;
        let new_w = (current_size.x * ratio).max(new_min.x);
        let new_h = (current_size.y * ratio).max(new_min.y);

        // Phase 1 (this frame's command batch):
        // Clear the minimum so the OS stops enforcing the old (possibly larger) value,
        // then resize the window in the same batch.
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(1.0, 1.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(new_w, new_h)));

        // Phase 2 (next frame): apply the correct minimum.
        self.deferred_min_size = Some(new_min);

        // Update stored normal size so compact→normal restores correctly
        if !self.compact_mode {
            self.normal_window_size = Some(egui::vec2(new_w, new_h));
        }

        self.save_current_settings();
    }

    /// Called when noita_dir text field loses focus with a new value.
    pub fn on_noita_dir_changed(&mut self) {
        let _ = logging::log(
            "INFO",
            &format!("Noita save dir changed: {}", self.settings.noita_dir),
            "Settings",
        );
        self.cancel_drag_if_active();
        self.reload_mods();
        self.check_workshop_mods_async();
        self.save_current_settings();
    }

    pub fn toggle_compact_mode(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        self.settings.compact_mode = self.compact_mode;
        let _ = settings::save_settings(&self.settings);

        let scale = self.settings.ui_scale;
        if self.compact_mode {
            // Save current size before shrinking
            let current_size = ctx.input(|i| i.content_rect().size());
            if current_size.x > 500.0 {
                self.normal_window_size = Some(current_size);
            }
            // Must lower min-size BEFORE setting inner size, or the OS clamps it.
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
            ));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale),
            ));
            let size = self.normal_window_size.unwrap_or_else(|| {
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_NORMAL, scale)
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
    }

    pub fn reload_mods(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let dir = PathBuf::from(&noita_dir);
        if let Ok(xml) = mods::read_mod_config(&dir)
            && let Ok(file_mods) = mods::parse_mods_from_xml(&xml)
        {
            self.current_mods = file_mods;
            self.presets
                .insert(self.selected_preset.clone(), self.current_mods.clone());
            let _ = presets::save_presets(&self.presets);
        }
        let config_path = dir.join("mod_config.xml");
        if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
            self.file_watcher.last_modified_time = mtime;
        }
    }

    /// User-initiated reload (logged + workshop check).
    pub fn reload_mods_explicit(&mut self) {
        self.cancel_drag_if_active();
        let before = self.current_mods.len();
        self.reload_mods();
        let after = self.current_mods.len();
        let _ = logging::log(
            "INFO",
            &format!("Manual reload: {} -> {} mod(s)", before, after),
            "ModManager",
        );
        self.check_workshop_mods_async();
    }

    /// Abort an in-flight drag and restore the pre-drag mod order.
    /// Used whenever a disruptive op (filter/sort/reload/external accept) would otherwise
    /// leave drag indices stale and cause panics or silent data corruption.
    pub fn cancel_drag_if_active(&mut self) {
        if let Some(drag) = self.drag_state.take() {
            self.current_mods = drag.pre_drag_snapshot;
            let _ = logging::log("INFO", "Drag cancelled (disruptive op)", "ModList");
        }
    }

    pub fn set_filter_mode(&mut self, mode: FilterMode) {
        if self.filter_mode == mode {
            return;
        }
        self.cancel_drag_if_active();
        self.filter_mode = mode;
        self.settings.last_filter_mode = mode.as_str().to_string();
        let _ = logging::log("DEBUG", &format!("Filter changed: {}", mode.label()), "UI");
        self.save_current_settings();
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        if self.sort_mode == mode {
            return;
        }
        self.cancel_drag_if_active();
        self.sort_mode = mode;
        self.settings.last_sort_mode = mode.as_str().to_string();
        let _ = logging::log("DEBUG", &format!("Sort changed: {}", mode.label()), "UI");
        self.save_current_settings();
    }

    /// Persist the current visual sort order back into the actual mod list / preset.
    /// Used when user explicitly clicks "Apply Sort to Order" — destructive op.
    pub fn apply_sort_to_order(&mut self) {
        if self.sort_mode == SortMode::Default {
            return;
        }
        if self.save_monitor.is_running() {
            return;
        }
        let mut sorted = self.current_mods.clone();
        sort_mods(&mut sorted, self.sort_mode);
        let mode = self.sort_mode;
        self.current_mods = sorted;
        self.sort_mode = SortMode::Default;
        self.settings.last_sort_mode = SortMode::Default.as_str().to_string();
        self.save_mod_config_and_preset();
        let _ = logging::log(
            "INFO",
            &format!("Applied sort to file order: {}", mode.label()),
            "ModManager",
        );
    }

    pub fn get_active_noita_dir(&self) -> String {
        if cfg!(debug_assertions)
            && let Ok(dev_dir) = platform::get_dev_save_dir()
        {
            return dev_dir.to_string_lossy().to_string();
        }
        self.settings.noita_dir.clone()
    }

    fn get_active_entangled_dir(&self) -> Option<String> {
        if cfg!(debug_assertions) {
            return platform::get_dev_entangled_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
        }

        let trimmed = self.settings.entangled_dir.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Check if a workshop mod is installed based on cached workshop status.
    pub fn is_workshop_mod_installed(&self, workshop_id: &str) -> Option<bool> {
        if workshop_id.is_empty() || workshop_id == "0" {
            return Some(true); // Local mod
        }
        self.backup_state
            .workshop_status
            .iter()
            .find(|(id, _)| id == workshop_id)
            .map(|(_, installed)| *installed)
    }

    // ── Import / Export ────────────────────────────────────────────────

    pub fn import_mod_list(&mut self) {
        let path = rfd::FileDialog::new()
            .set_title("Import Mod List")
            .add_filter("JSON", &["json"])
            .pick_file();

        let path = match path {
            Some(p) => p,
            None => return,
        };

        let content = match mods::read_file(&path) {
            Ok(c) => c,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };

        let imported: Vec<ModListEntry> = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Invalid mod list format: {}", e),
                });
                return;
            }
        };

        let mut found_in_order = Vec::new();
        let mut missing = Vec::new();

        for imp in &imported {
            let key = if imp.workshop_id != "0" && !imp.workshop_id.is_empty() {
                &imp.workshop_id
            } else {
                &imp.name
            };

            if let Some(pos) = self.current_mods.iter().position(|m| {
                if m.workshop_id != "0" && !m.workshop_id.is_empty() {
                    &m.workshop_id == key
                } else {
                    &m.name == key
                }
            }) {
                found_in_order.push(pos);
            } else {
                missing.push((imp.name.clone(), imp.workshop_id.clone()));
            }
        }

        if !missing.is_empty() {
            let mut new_mods = Vec::new();
            for &idx in &found_in_order {
                let mut m = self.current_mods[idx].clone();
                m.enabled = true;
                new_mods.push(m);
            }
            let found_set: std::collections::HashSet<usize> =
                found_in_order.iter().copied().collect();
            for (i, m) in self.current_mods.iter().enumerate() {
                if !found_set.contains(&i) {
                    let mut m = m.clone();
                    m.enabled = false;
                    new_mods.push(m);
                }
            }

            self.active_modal = Some(Modal::MissingMods {
                mods: missing,
                action: MissingModsAction::ModImport(new_mods),
            });
        } else {
            self.apply_mod_import(&found_in_order);
        }
    }

    fn apply_mod_import(&mut self, found_indices: &[usize]) {
        let found_set: std::collections::HashSet<usize> = found_indices.iter().copied().collect();
        let mut new_mods = Vec::new();
        for &idx in found_indices {
            let mut m = self.current_mods[idx].clone();
            m.enabled = true;
            new_mods.push(m);
        }
        for (i, m) in self.current_mods.iter().enumerate() {
            if !found_set.contains(&i) {
                let mut m = m.clone();
                m.enabled = false;
                new_mods.push(m);
            }
        }
        self.current_mods = new_mods;
        self.save_mod_config_and_preset();
        let _ = logging::log(
            "INFO",
            &format!("Imported mod list ({} mods matched)", found_indices.len()),
            "ModManager",
        );
    }

    pub fn export_mod_list(&mut self) {
        let enabled: Vec<ModListEntry> = self
            .current_mods
            .iter()
            .filter(|m| m.enabled)
            .map(|m| ModListEntry {
                name: m.name.clone(),
                workshop_id: m.workshop_id.clone(),
            })
            .collect();

        if enabled.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Export".to_string(),
                message: "No enabled mods to export.".to_string(),
            });
            return;
        }

        let path = rfd::FileDialog::new()
            .set_title("Export Enabled Mods")
            .set_file_name(format!("{}-mod-list.json", self.selected_preset))
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            match serde_json::to_string_pretty(&enabled) {
                Ok(content) => {
                    if let Err(e) = mods::write_file(&path, &content) {
                        let _ =
                            logging::log("ERROR", &format!("Export failed: {}", e), "ModManager");
                    } else {
                        let _ = logging::log(
                            "INFO",
                            &format!("Exported {} mods", enabled.len()),
                            "ModManager",
                        );
                    }
                }
                Err(e) => {
                    let _ = logging::log(
                        "ERROR",
                        &format!("Serialization failed: {}", e),
                        "ModManager",
                    );
                }
            }
        }
    }

    pub fn start_export_presets(&mut self) {
        let preset_names: Vec<String> = self.presets.keys().cloned().collect();
        if preset_names.is_empty() {
            return;
        }

        let items: Vec<ChecklistItem> = preset_names
            .iter()
            .map(|name| {
                let count = self.presets.get(name).map_or(0, |m| m.len());
                ChecklistItem {
                    id: name.clone(),
                    label: format!("{} ({} mods)", name, count),
                    checked: true,
                }
            })
            .collect();

        self.active_modal = Some(Modal::Checklist {
            title: "Export Presets".to_string(),
            message: "Select presets to export:".to_string(),
            items,
            action: ChecklistAction::ExportPresets,
        });
    }

    pub fn import_presets(&mut self) {
        let path = rfd::FileDialog::new()
            .set_title("Import Presets")
            .add_filter("JSON", &["json"])
            .pick_file();

        let path = match path {
            Some(p) => p,
            None => return,
        };

        let content = match mods::read_file(&path) {
            Ok(c) => c,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: e,
                });
                return;
            }
        };

        if let Err(e) = presets::validate_preset_file(&content) {
            self.active_modal = Some(Modal::Info {
                title: "Import Rejected".to_string(),
                message: e,
            });
            return;
        }

        let import_data: PresetExportFile = match serde_json::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Invalid preset file: {}", e),
                });
                return;
            }
        };

        if import_data.hallinta_export != "presets" || import_data.presets.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Import Failed".to_string(),
                message: "Invalid preset file format.".to_string(),
            });
            return;
        }

        // Checksum verification
        if let Some(ref checksum) = import_data.checksum
            && let Ok(canonical) = serde_json::to_string(&import_data.presets)
            && !gallery::verify_checksum(&canonical, checksum)
        {
            let raw_presets_str = serde_json::to_string(&import_data.presets).unwrap_or_default();
            if !gallery::verify_checksum(&raw_presets_str, checksum) {
                let import = PresetImportData {
                    presets: import_data.presets.clone(),
                    selected_names: import_data.presets.keys().cloned().collect(),
                };
                self.active_modal = Some(Modal::Confirm {
                    message: "Checksum mismatch: the preset file may have been modified. Continue?"
                        .to_string(),
                    confirm_text: "Continue".to_string(),
                    cancel_text: "Cancel".to_string(),
                    action: ConfirmAction::ChecksumMismatchContinue(import),
                    cancel_action: None,
                });
                return;
            }
        }

        // Check for missing workshop mods across all presets
        let steam_path = &self.settings.steam_path;
        if !steam_path.is_empty() {
            let all_workshop_ids: Vec<String> = import_data
                .presets
                .values()
                .flatten()
                .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
                .map(|m| m.workshop_id.clone())
                .collect();

            if !all_workshop_ids.is_empty()
                && let Ok(statuses) =
                    workshop::check_workshop_mods_installed(&all_workshop_ids, steam_path)
            {
                let missing: Vec<(String, String)> = import_data
                    .presets
                    .values()
                    .flatten()
                    .filter(|m| {
                        statuses
                            .iter()
                            .any(|(id, installed)| id == &m.workshop_id && !installed)
                    })
                    .map(|m| (m.name.clone(), m.workshop_id.clone()))
                    .collect();

                if !missing.is_empty() {
                    let import = PresetImportData {
                        presets: import_data.presets,
                        selected_names: Vec::new(),
                    };
                    self.active_modal = Some(Modal::MissingMods {
                        mods: missing,
                        action: MissingModsAction::PresetImport(import),
                    });
                    return;
                }
            }
        }

        // Show checklist for which presets to import
        let items = self.build_preset_import_checklist(&import_data.presets);

        self.active_modal = Some(Modal::Checklist {
            title: "Import Presets".to_string(),
            message: "Select presets to import:".to_string(),
            items,
            action: ChecklistAction::ImportPresets(PresetImportData {
                presets: import_data.presets,
                selected_names: Vec::new(),
            }),
        });
    }

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
                let noita_dir = self.get_active_noita_dir();
                let include_save01 = self.settings.save_monitor_settings.include_save01;
                let entangled = if self.settings.save_monitor_settings.include_entangled {
                    self.get_active_entangled_dir()
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
                let noita_dir = self.get_active_noita_dir();
                let include_save01 = self.settings.save_monitor_settings.include_save01;
                let entangled = if self.settings.save_monitor_settings.include_entangled {
                    self.get_active_entangled_dir()
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

    fn check_save_monitor_changes(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let include_save01 = self.settings.save_monitor_settings.include_save01;
        let entangled_dir = if self.settings.save_monitor_settings.include_entangled {
            self.get_active_entangled_dir()
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

    fn take_monitor_snapshot(&mut self) {
        let noita_dir = self.get_active_noita_dir();
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
            self.get_active_entangled_dir()
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

    // ── Open mod_config.xml ───────────────────────────────────────────

    pub fn open_mod_config_file(&self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
        if mods::check_file_exists(&config_path) {
            let _ = platform::open_file(&config_path);
        }
    }

    // ── Modal Action Handlers ──────────────────────────────────────────

    pub fn handle_confirm_action(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DeletePreset => {
                if self.selected_preset != "Default" {
                    self.cancel_drag_if_active();
                    let deleted = self.selected_preset.clone();
                    let mod_count = self.presets.get(&deleted).map(|m| m.len()).unwrap_or(0);
                    self.presets.remove(&deleted);
                    self.selected_preset = "Default".to_string();
                    self.current_mods = self.presets.get("Default").cloned().unwrap_or_default();
                    self.save_mod_config_and_preset();
                    let _ = logging::log(
                        "INFO",
                        &format!(
                            "Deleted preset: {} ({} mods) — switched to Default",
                            deleted, mod_count
                        ),
                        "PresetManager",
                    );
                } else {
                    let _ =
                        logging::log("WARN", "Refused to delete Default preset", "PresetManager");
                }
            }
            ConfirmAction::DeleteMod(hint_idx, expected_name, expected_workshop) => {
                // Re-resolve by name + workshop_id so list mutations between menu open and
                // confirm do not delete the wrong mod.
                let resolved = if hint_idx < self.current_mods.len()
                    && self.current_mods[hint_idx].name == expected_name
                    && self.current_mods[hint_idx].workshop_id == expected_workshop
                {
                    Some(hint_idx)
                } else {
                    self.current_mods
                        .iter()
                        .position(|m| m.name == expected_name && m.workshop_id == expected_workshop)
                };
                match resolved {
                    Some(idx) => {
                        let removed = self.current_mods.remove(idx);
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Deleted mod: {} (workshop_id={}, was at position {})",
                                removed.name,
                                if removed.workshop_id.is_empty() {
                                    "none"
                                } else {
                                    &removed.workshop_id
                                },
                                idx + 1
                            ),
                            "ModManager",
                        );
                    }
                    None => {
                        let _ = logging::log(
                            "WARN",
                            &format!(
                                "Delete mod cancelled: \"{}\" no longer in list (workshop_id={})",
                                expected_name,
                                if expected_workshop.is_empty() {
                                    "none"
                                } else {
                                    &expected_workshop
                                }
                            ),
                            "ModManager",
                        );
                    }
                }
            }
            ConfirmAction::AcceptExternalChanges(file_mods) => {
                self.cancel_drag_if_active();
                let prev_count = self.current_mods.len();
                let new_count = file_mods.len();
                self.current_mods = file_mods;
                self.presets
                    .insert(self.selected_preset.clone(), self.current_mods.clone());
                let _ = presets::save_presets(&self.presets);
                let _ = logging::log(
                    "INFO",
                    &format!(
                        "Accepted external mod_config.xml change ({} -> {} mods)",
                        prev_count, new_count
                    ),
                    "ModManager",
                );
            }
            ConfirmAction::KeepCurrentPreset => {
                self.save_mod_config_and_preset();
                let _ = logging::log(
                    "INFO",
                    "Kept current preset, re-wrote mod_config.xml",
                    "ModManager",
                );
            }
            ConfirmAction::OverwritePresetImport(import) => {
                self.do_import_presets(&import, true);
            }
            ConfirmAction::RenamePresetImport(import) => {
                self.do_import_presets(&import, false);
            }
            ConfirmAction::ChecksumMismatchContinue(import) => {
                let items: Vec<ChecklistItem> = import
                    .presets
                    .keys()
                    .map(|name| {
                        let count = import.presets.get(name).map_or(0, |m| m.len());
                        ChecklistItem {
                            id: name.clone(),
                            label: format!("{} ({} mods)", name, count),
                            checked: true,
                        }
                    })
                    .collect();
                self.active_modal = Some(Modal::Checklist {
                    title: "Import Presets".to_string(),
                    message: "Select presets to import:".to_string(),
                    items,
                    action: ChecklistAction::ImportPresets(import),
                });
            }
            ConfirmAction::ExitWithSnapshot => {
                let _ = logging::log("INFO", "Exit chosen: take final snapshot", "App");
                self.take_monitor_snapshot();
                self.end_monitor_session();
                self.close_requested = false;
            }
            ConfirmAction::ExitWithoutSnapshot => {
                let _ = logging::log("INFO", "Exit chosen: no final snapshot", "App");
                self.end_monitor_session();
                self.close_requested = false;
            }
            ConfirmAction::DeleteBackup(filename) => {
                self.delete_backup_async(filename);
            }
            ConfirmAction::RestoreLatest(filename) => {
                self.apply_default_restore(filename);
            }
            ConfirmAction::ClearMonitorData => {
                self.clear_monitor_data_async();
            }
            ConfirmAction::ContinueMonitorSession(session_id) => {
                self.resume_monitor_session(&session_id);
            }
            ConfirmAction::StartNewMonitorSession => {
                self.start_new_monitor_session();
            }
            ConfirmAction::StopAndEndSession => {
                self.end_monitor_session();
            }
        }
    }

    pub fn handle_input_action(&mut self, action: InputAction, value: String) {
        let value = value.trim().to_string();
        if value.is_empty() {
            return;
        }

        match action {
            InputAction::CreatePreset => {
                if !self.presets.contains_key(&value) {
                    self.presets
                        .insert(value.clone(), self.current_mods.clone());
                    self.selected_preset = value.clone();
                    self.save_mod_config_and_preset();
                    let _ = logging::log(
                        "INFO",
                        &format!("Created preset: {}", value),
                        "PresetManager",
                    );
                }
            }
            InputAction::RenamePreset => {
                if self.selected_preset != "Default"
                    && !self.presets.contains_key(&value)
                    && value != self.selected_preset
                {
                    let old_name = self.selected_preset.clone();
                    if let Some(mods_list) = self.presets.remove(&old_name) {
                        self.presets.insert(value.clone(), mods_list);
                        self.selected_preset = value.clone();
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!("Renamed preset {} -> {}", old_name, value),
                            "PresetManager",
                        );
                    }
                }
            }
            InputAction::MoveModToPosition(from_idx) => {
                if let Ok(target) = value.parse::<usize>() {
                    let target_idx = target.saturating_sub(1);
                    if from_idx < self.current_mods.len()
                        && target_idx < self.current_mods.len()
                        && from_idx != target_idx
                    {
                        let mod_name = self.current_mods[from_idx].name.clone();
                        let item = self.current_mods.remove(from_idx);
                        self.current_mods.insert(target_idx, item);
                        self.save_mod_config_and_preset();
                        let _ = logging::log(
                            "INFO",
                            &format!(
                                "Moved \"{}\" from position {} to {}",
                                mod_name,
                                from_idx + 1,
                                target_idx + 1
                            ),
                            "ModManager",
                        );
                    } else {
                        let _ = logging::log(
                            "WARN",
                            &format!(
                                "Move-to-position rejected (from={}, target={}, len={})",
                                from_idx + 1,
                                target_idx + 1,
                                self.current_mods.len()
                            ),
                            "ModManager",
                        );
                    }
                }
            }
        }
    }

    pub fn handle_checklist_action(&mut self, action: ChecklistAction, selected: Vec<String>) {
        match action {
            ChecklistAction::ExportPresets => {
                if selected.is_empty() {
                    return;
                }

                let mut export_presets = BTreeMap::new();
                for name in &selected {
                    if let Some(mods_list) = self.presets.get(name) {
                        export_presets.insert(name.clone(), mods_list.clone());
                    }
                }

                let checksum = serde_json::to_string(&export_presets)
                    .ok()
                    .map(|s| gallery::compute_checksum(&s));

                let export = PresetExportFile {
                    hallinta_export: "presets".to_string(),
                    version: platform::get_version(),
                    presets: export_presets,
                    checksum,
                };

                let path = rfd::FileDialog::new()
                    .set_title("Export Presets")
                    .set_file_name("hallinta-presets.json")
                    .add_filter("JSON", &["json"])
                    .save_file();

                if let Some(path) = path
                    && let Ok(content) = serde_json::to_string_pretty(&export)
                {
                    let _ = mods::write_file(&path, &content);
                    let _ = logging::log(
                        "INFO",
                        &format!("Exported {} preset(s)", selected.len()),
                        "PresetManager",
                    );
                }
            }
            ChecklistAction::ImportPresets(mut import) => {
                import.selected_names = selected;
                if import.selected_names.is_empty() {
                    return;
                }

                let conflicts: Vec<String> = import
                    .selected_names
                    .iter()
                    .filter(|n| self.presets.contains_key(*n))
                    .cloned()
                    .collect();

                if conflicts.is_empty() {
                    self.do_import_presets(&import, false);
                } else {
                    self.active_modal = Some(Modal::Confirm {
                        message: format!(
                            "{} preset(s) already exist: {}. Overwrite?",
                            conflicts.len(),
                            conflicts.join(", ")
                        ),
                        confirm_text: "Overwrite".to_string(),
                        cancel_text: "Rename".to_string(),
                        action: ConfirmAction::OverwritePresetImport(import.clone()),
                        cancel_action: Some(ConfirmAction::RenamePresetImport(import)),
                    });
                }
            }
            ChecklistAction::Backup => {
                let include_save01 = selected.contains(&"save01".to_string());
                let include_presets = selected.contains(&"presets".to_string());
                let include_entangled = selected.contains(&"entangled".to_string());

                let noita_dir = PathBuf::from(self.get_active_noita_dir());
                let entangled_dir = if include_entangled {
                    self.get_active_entangled_dir().map(PathBuf::from)
                } else {
                    None
                };
                let tx = self.task_tx.clone();

                self.backup_state.in_progress = true;
                self.active_modal = Some(Modal::Progress {
                    message: "Creating backup...".to_string(),
                    progress: 0.5,
                });

                logging::write_session_marker("BACKUP_START");
                self.async_runtime.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        backup::create_backup(
                            &noita_dir,
                            include_save01,
                            include_presets,
                            include_entangled,
                            entangled_dir.as_deref(),
                        )
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Backup task failed: {}", e)));
                    let _ = tx.send(TaskResult::BackupComplete(result));
                });
            }
            ChecklistAction::Restore(ref _filename) => {
                if let Some(filename) = selected.first() {
                    if let Ok(info) = backup::get_backup_contents(filename) {
                        let mut restore_items = Vec::new();
                        if info.contains_save00 {
                            restore_items.push(ChecklistItem {
                                id: "save00".to_string(),
                                label: "save00".to_string(),
                                checked: true,
                            });
                        }
                        if info.contains_save01 {
                            restore_items.push(ChecklistItem {
                                id: "save01".to_string(),
                                label: "save01".to_string(),
                                checked: true,
                            });
                        }
                        if info.contains_presets {
                            restore_items.push(ChecklistItem {
                                id: "presets".to_string(),
                                label: "presets.json".to_string(),
                                checked: true,
                            });
                        }
                        if info.contains_entangled {
                            restore_items.push(ChecklistItem {
                                id: "entangled".to_string(),
                                label: "Entangled Worlds".to_string(),
                                checked: true,
                            });
                        }

                        self.active_modal = Some(Modal::Checklist {
                            title: format!("Restore {}", filename),
                            message: "Select components to restore:".to_string(),
                            items: restore_items,
                            action: ChecklistAction::Restore(filename.clone()),
                        });
                    }
                } else if !_filename.is_empty() {
                    let filename = _filename.clone();
                    let noita_dir = PathBuf::from(self.get_active_noita_dir());
                    let entangled_dir = if selected.contains(&"entangled".to_string()) {
                        self.get_active_entangled_dir().map(PathBuf::from)
                    } else {
                        None
                    };
                    let options = RestoreOptions {
                        restore_save00: selected.contains(&"save00".to_string()),
                        restore_save01: selected.contains(&"save01".to_string()),
                        restore_presets: selected.contains(&"presets".to_string()),
                        restore_entangled: selected.contains(&"entangled".to_string()),
                    };
                    let tx = self.task_tx.clone();

                    self.backup_state.restoring = true;
                    self.active_modal = Some(Modal::Progress {
                        message: "Restoring backup...".to_string(),
                        progress: 0.5,
                    });

                    logging::write_session_marker("RESTORE_START");
                    self.async_runtime.spawn(async move {
                        let result = tokio::task::spawn_blocking(move || {
                            backup::restore_backup(
                                &filename,
                                &noita_dir,
                                &options,
                                entangled_dir.as_deref(),
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("Restore task failed: {}", e)));
                        let _ = tx.send(TaskResult::RestoreComplete(result));
                    });
                }
            }
            ChecklistAction::RestoreSnapshot(zip_path) => {
                let noita_dir = PathBuf::from(self.get_active_noita_dir());
                let entangled_dir = if selected.contains(&"entangled".to_string()) {
                    self.get_active_entangled_dir().map(PathBuf::from)
                } else {
                    None
                };
                let options = RestoreOptions {
                    restore_save00: selected.contains(&"save00".to_string()),
                    restore_save01: selected.contains(&"save01".to_string()),
                    restore_presets: false,
                    restore_entangled: selected.contains(&"entangled".to_string()),
                };
                let tx = self.task_tx.clone();

                self.backup_state.restoring = true;
                self.active_modal = Some(Modal::Progress {
                    message: "Restoring snapshot...".to_string(),
                    progress: 0.5,
                });

                logging::write_session_marker("SNAPSHOT_RESTORE_START");
                self.async_runtime.spawn(async move {
                    let result = tokio::task::spawn_blocking(move || {
                        backup::restore_from_path(
                            &zip_path,
                            &noita_dir,
                            &options,
                            entangled_dir.as_deref(),
                        )
                    })
                    .await
                    .unwrap_or_else(|e| Err(format!("Restore failed: {}", e)));
                    let _ = tx.send(TaskResult::RestoreComplete(result));
                });
            }
        }
    }

    pub fn handle_missing_mods_action(&mut self, action: MissingModsAction) {
        match action {
            MissingModsAction::ModImport(new_mods) => {
                self.current_mods = new_mods;
                self.save_mod_config_and_preset();
            }
            MissingModsAction::PresetImport(import) => {
                // Show the preset selection checklist after acknowledging missing mods
                let items = self.build_preset_import_checklist(&import.presets);
                self.active_modal = Some(Modal::Checklist {
                    title: "Import Presets".to_string(),
                    message: "Select presets to import:".to_string(),
                    items,
                    action: ChecklistAction::ImportPresets(import),
                });
            }
        }
    }

    fn do_import_presets(&mut self, import: &PresetImportData, overwrite: bool) {
        let mut imported = 0;
        for name in &import.selected_names {
            if let Some(mods_list) = import.presets.get(name) {
                let mut target_name = name.clone();
                if !overwrite {
                    target_name = self.unique_preset_name(name);
                }
                self.presets.insert(target_name, mods_list.clone());
                imported += 1;
            }
        }

        let _ = presets::save_presets(&self.presets);
        let _ = logging::log(
            "INFO",
            &format!("Imported {} preset(s)", imported),
            "PresetManager",
        );
    }

    // ── Helpers ────────────────────────────────────────────────────────

    /// Generate a conflict-free preset name by appending " (imported)" / " (imported N)".
    fn unique_preset_name(&self, base_name: &str) -> String {
        let mut target = base_name.to_string();
        if self.presets.contains_key(&target) {
            target = format!("{} (imported)", base_name);
            let mut counter = 2;
            while self.presets.contains_key(&target) {
                target = format!("{} (imported {})", base_name, counter);
                counter += 1;
            }
        }
        target
    }

    fn build_preset_import_checklist(
        &self,
        presets: &BTreeMap<String, Vec<ModEntry>>,
    ) -> Vec<ChecklistItem> {
        presets
            .keys()
            .map(|name| {
                let count = presets.get(name).map_or(0, |m| m.len());
                let exists = self.presets.contains_key(name);
                ChecklistItem {
                    id: name.clone(),
                    label: format!(
                        "{} ({} mods){}",
                        name,
                        count,
                        if exists { " - EXISTS" } else { "" }
                    ),
                    checked: true,
                }
            })
            .collect()
    }

    // ── Cleanup ────────────────────────────────────────────────────────

    pub fn cleanup_on_exit(&mut self) {
        let _ = logging::log("INFO", "Application shutting down", "App");

        // Dev mode: verify real directories are untouched
        if cfg!(debug_assertions) {
            match platform::restore_real_dirs_from_dev() {
                Ok(msg) => {
                    let _ = logging::log("INFO", &format!("[DEV] Exit: {}", msg), "DevData");
                }
                Err(e) => {
                    let _ = logging::log(
                        "WARN",
                        &format!("[DEV] Exit restore error: {}", e),
                        "DevData",
                    );
                }
            }
        }

        logging::write_session_marker("APP_SHUTDOWN");

        let _ = logging::flush_log_buffer_sync();
        logging::write_session_end_marker();
        let _ = logging::flush_log_buffer_sync();
    }
}

impl eframe::App for HallintaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0. Apply UI zoom (must be before any rendering)
        crate::ui::design::apply_zoom(ctx, &self.settings);

        // 0b. Apply deferred min size (queued on previous frame to avoid one-behind lag)
        if let Some(min) = self.deferred_min_size.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));
        }

        // 1. Poll async task results
        self.poll_task_results();

        // 2. Check timers
        self.check_timers(ctx);

        // 3. Handle close request
        self.handle_close(ctx);

        // 4. Handle keyboard
        self.handle_keyboard(ctx);

        // 5. Render UI
        crate::ui::header::render_header(self, ctx);

        if !self.compact_mode && self.active_view != View::Settings {
            crate::ui::sidebar::render_sidebar(self, ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_view {
                View::ModList => {
                    if self.compact_mode {
                        crate::ui::compact::render_compact(self, ui);
                    } else if self.save_monitor.is_running() {
                        // Monitor running: show monitor status instead of mod list
                        crate::ui::mod_list::render_monitor_active(self, ui);
                    } else {
                        crate::ui::mod_list::render_mod_list(self, ui);
                    }
                }
                View::Settings => {
                    crate::ui::settings::render_settings(self, ui);
                }
            }
        });

        // 6. Render modals on top
        crate::ui::modals::render_modals(self, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cleanup_on_exit();
    }
}

pub fn sort_mods(mods: &mut [ModEntry], mode: SortMode) {
    match mode {
        SortMode::Default => {}
        SortMode::NameAsc => mods.sort_by_key(|m| m.name.to_lowercase()),
        SortMode::NameDesc => {
            mods.sort_by_key(|m| std::cmp::Reverse(m.name.to_lowercase()));
        }
        SortMode::EnabledFirst => mods.sort_by_key(|m| !m.enabled),
        SortMode::DisabledFirst => mods.sort_by_key(|m| m.enabled),
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
