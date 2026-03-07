use crate::core::{backup, file_watcher, gallery, logging, mods, platform, presets, save_monitor, settings, workshop};
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
    pub compact_mode: bool,
    pub dark_mode: bool,

    // Modal state
    pub active_modal: Option<Modal>,

    // Feature state
    pub save_monitor: SaveMonitorState,
    pub backup_state: BackupState,
    pub gallery_state: GalleryState,
    pub file_watcher: FileWatcherState,

    // Async coordination
    pub async_runtime: tokio::runtime::Handle,
    pub task_tx: mpsc::Sender<TaskResult>,
    pub task_rx: mpsc::Receiver<TaskResult>,

    // Drag state
    pub drag_state: Option<DragState>,

    // Pending settings edit
    pub pending_settings: Option<AppSettings>,

    // Timers
    last_log_flush: Instant,

    // Normal mode window size (for restoring after compact)
    normal_window_size: Option<egui::Vec2>,

    // Track whether close was requested while monitor is running
    close_requested: bool,
}

impl HallintaApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: tokio::runtime::Handle) -> Self {
        let (task_tx, task_rx) = mpsc::channel();

        // Load settings
        let mut app_settings = settings::load_settings().unwrap_or_else(|e| {
            eprintln!("Failed to load settings: {}", e);
            AppSettings {
                noita_dir: String::new(),
                entangled_dir: String::new(),
                dark_mode: false,
                selected_preset: "Default".to_string(),
                version: platform::get_version(),
                log_settings: LogSettings::default(),
                backup_settings: BackupSettings::default(),
                save_monitor_settings: SaveMonitorSettings::default(),
                gallery_settings: GallerySettings::default(),
                compact_mode: false,
            }
        });

        // Load presets
        let app_presets = presets::load_presets().unwrap_or_else(|e| {
            eprintln!("Failed to load presets: {}", e);
            let mut m = BTreeMap::new();
            m.insert("Default".to_string(), Vec::new());
            m
        });

        // Dev build sandbox paths (never touch real save files in debug runs).
        // On first run, seed dev_data/save00/mod_config.xml from the real Noita save.
        let dev_noita_dir = if cfg!(debug_assertions) {
            match platform::seed_dev_mod_config() {
                Ok(msg) => {
                    let _ = logging::log("INFO", &format!("[DEV] {}", msg), "DevData");
                }
                Err(e) => {
                    let _ = logging::log("WARN", &format!("[DEV] Dev data seed error: {}", e), "DevData");
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

        // Apply theme
        let dark_mode = app_settings.dark_mode;
        let compact_mode = app_settings.compact_mode;
        crate::ui::theme::apply_theme(&cc.egui_ctx, dark_mode);

        // Apply compact mode window size if needed
        if compact_mode {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(480.0, 400.0)));
        }

        // Log system info if configured (now with full detail)
        if app_settings.log_settings.collect_system_info {
            platform::log_system_info_on_startup();
        }

        // Backup cleanup
        let _ = backup::cleanup_old_backups(app_settings.backup_settings.auto_delete_days);

        // File watcher: get initial mtime
        let mut file_watcher_state = FileWatcherState::new();
        if !active_noita_dir.is_empty() {
            let config_path = PathBuf::from(&active_noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                file_watcher_state.last_modified_time = mtime;
            }
        }

        let now = Instant::now();
        let auto_backup_interval = app_settings.backup_settings.backup_interval_minutes;
        let backup_state = BackupState {
            auto_backup_due: if auto_backup_interval > 0 {
                Some(now + Duration::from_secs(auto_backup_interval as u64 * 60))
            } else {
                None
            },
            ..BackupState::new()
        };

        let save_monitor_state = SaveMonitorState::new();

        let mut app = Self {
            settings: app_settings,
            presets: app_presets,
            current_mods,
            selected_preset,
            active_view: View::ModList,
            search_query: String::new(),
            filter_mode: FilterMode::All,
            compact_mode,
            dark_mode,
            active_modal: None,
            save_monitor: save_monitor_state,
            backup_state,
            gallery_state: GalleryState::new(),
            file_watcher: file_watcher_state,
            async_runtime: rt,
            task_tx,
            task_rx,
            drag_state: None,
            pending_settings: None,
            last_log_flush: now,
            normal_window_size: None,
            close_requested: false,
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
        logging::write_session_marker("APP_INITIALIZED");
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

        // File watcher (every 5 seconds)
        let should_check = self
            .file_watcher
            .last_check
            .map_or(true, |t| now.duration_since(t) > self.file_watcher.check_interval);
        if should_check && self.active_modal.is_none() {
            self.file_watcher.last_check = Some(now);
            self.check_external_changes();
        }

        // Save monitor (periodic snapshots)
        if self.save_monitor.is_running() {
            let interval = Duration::from_secs(
                self.settings.save_monitor_settings.interval_minutes as u64 * 60,
            );
            let should_snapshot = self
                .save_monitor
                .last_snapshot
                .map_or(false, |t| now.duration_since(t) > interval);
            if should_snapshot {
                self.take_monitor_snapshot();
            }
        }

        // Auto-backup
        if let Some(due) = self.backup_state.auto_backup_due {
            if now >= due && !self.backup_state.in_progress {
                self.run_auto_backup();
                let interval = self.settings.backup_settings.backup_interval_minutes;
                self.backup_state.auto_backup_due =
                    Some(now + Duration::from_secs(interval as u64 * 60));
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

            if let Ok(xml) = mods::read_mod_config(&dir) {
                if let Ok(file_mods) = mods::parse_mods_from_xml(&xml) {
                    if !mods_equal(&self.current_mods, &file_mods) {
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
    }

    // ── Task Result Handling ───────────────────────────────────────────

    fn poll_task_results(&mut self) {
        while let Ok(result) = self.task_rx.try_recv() {
            match result {
                TaskResult::BackupComplete(res) => {
                    self.backup_state.in_progress = false;
                    self.active_modal = None;
                    match res {
                        Ok(filename) => {
                            let _ = logging::log(
                                "INFO",
                                &format!("Backup created: {}", filename),
                                "Backup",
                            );
                            self.load_backup_list_async();
                            let backup_path = settings::get_data_dir()
                                .map(|d| d.join("backups").join(&filename).to_string_lossy().to_string())
                                .unwrap_or(filename.clone());
                            self.active_modal = Some(Modal::Info {
                                title: "Backup Created".to_string(),
                                message: format!("Saved to:\n{}", backup_path),
                            });
                        }
                        Err(e) => {
                            let _ = logging::log("ERROR", &format!("Backup failed: {}", e), "Backup");
                            self.active_modal = Some(Modal::Info {
                                title: "Backup Failed".to_string(),
                                message: e,
                            });
                        }
                    }
                }
                TaskResult::RestoreComplete(res) => {
                    self.backup_state.restoring = false;
                    self.active_modal = None;
                    match res {
                        Ok(()) => {
                            let _ = logging::log("INFO", "Restore complete", "Backup");
                            self.reload_mods();
                        }
                        Err(e) => {
                            let _ =
                                logging::log("ERROR", &format!("Restore failed: {}", e), "Backup");
                            self.active_modal = Some(Modal::Info {
                                title: "Restore Failed".to_string(),
                                message: e,
                            });
                        }
                    }
                }
                TaskResult::SnapshotComplete(res) => {
                    match res {
                        Ok(filename) => {
                            self.save_monitor.snapshot_count += 1;
                            self.save_monitor.last_snapshot = Some(Instant::now());
                            let _ = logging::log(
                                "INFO",
                                &format!("Snapshot created: {}", filename),
                                "SaveMonitor",
                            );
                            // Cleanup old snapshots with keep-every-nth (async)
                            let preset = self.selected_preset.clone();
                            let keep = self.settings.save_monitor_settings.max_snapshots_per_preset;
                            let keep_nth = self.settings.save_monitor_settings.keep_every_nth;
                            let cleanup_tx = self.task_tx.clone();
                            self.async_runtime.spawn(async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    save_monitor::cleanup_monitor_snapshots(&preset, keep, keep_nth)
                                })
                                .await
                                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
                                let _ = cleanup_tx.send(TaskResult::SnapshotCleanupComplete(result));
                            });
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
                TaskResult::CatalogFetched(res) => {
                    self.gallery_state.loading = false;
                    match res {
                        Ok(catalog) => {
                            self.gallery_state.catalog = Some(catalog);
                            self.gallery_state.catalog_fetched_at = Some(Instant::now());
                            self.gallery_state.error = None;
                        }
                        Err(e) => {
                            self.gallery_state.error = Some(e);
                        }
                    }
                }
                TaskResult::PresetDownloaded(res) => {
                    match res {
                        Ok(content) => {
                            self.handle_downloaded_preset(&content);
                        }
                        Err(e) => {
                            self.active_modal = Some(Modal::Info {
                                title: "Download Failed".to_string(),
                                message: e,
                            });
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
                TaskResult::AutoBackupComplete(res) => {
                    match res {
                        Ok(filename) => {
                            let _ = logging::log(
                                "INFO",
                                &format!("Auto-backup created: {}", filename),
                                "Backup",
                            );
                            self.load_backup_list_async();
                        }
                        Err(e) => {
                            let _ = logging::log(
                                "ERROR",
                                &format!("Auto-backup failed: {}", e),
                                "Backup",
                            );
                        }
                    }
                }
                TaskResult::BackupListLoaded(res) => {
                    if let Ok(list) = res {
                        self.backup_state.backup_list = list;
                    }
                }
                TaskResult::SnapshotListLoaded(res) => {
                    if let Ok(list) = res {
                        self.backup_state.snapshot_list = list;
                    }
                }
                TaskResult::WorkshopModsChecked(res) => {
                    if let Ok(status) = res {
                        self.backup_state.workshop_status = status;
                    }
                }
                TaskResult::BackupCleanupComplete(res) => {
                    if let Ok(count) = res {
                        if count > 0 {
                            let _ = logging::log(
                                "INFO",
                                &format!("Cleaned up {} old backup(s)", count),
                                "Backup",
                            );
                            self.load_backup_list_async();
                        }
                    }
                }
                TaskResult::SnapshotCleanupComplete(res) => {
                    if let Ok(count) = res {
                        if count > 0 {
                            let _ = logging::log(
                                "INFO",
                                &format!("Cleaned up {} old snapshot(s)", count),
                                "SaveMonitor",
                            );
                        }
                    }
                }
                TaskResult::BackupDeleted(res) => {
                    match res {
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
                    }
                }
                TaskResult::MonitorDataCleared(res) => {
                    match res {
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
                    }
                }
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

    pub fn load_snapshot_list_async(&self, preset_name: String) {
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || save_monitor::list_monitor_snapshots(&preset_name))
                    .await
                    .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::SnapshotListLoaded(result));
        });
    }

    pub fn check_workshop_mods_async(&self) {
        let steam_path = self.settings.gallery_settings.steam_path.clone();
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
            let result = tokio::task::spawn_blocking(move || {
                backup::delete_backup(&fname).map(|()| fname)
            })
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

    pub fn run_backup_cleanup_async(&self) {
        let days = self.settings.backup_settings.auto_delete_days;
        if days == 0 {
            return;
        }
        let tx = self.task_tx.clone();
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || backup::cleanup_old_backups(days))
                .await
                .unwrap_or_else(|e| Err(format!("Task failed: {}", e)));
            let _ = tx.send(TaskResult::BackupCleanupComplete(result));
        });
    }

    // ── Keyboard Handling ──────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.active_modal.is_some() {
                if !matches!(self.active_modal, Some(Modal::Progress { .. })) {
                    self.active_modal = None;
                }
            } else if self.active_view == View::Settings || self.active_view == View::PresetVault {
                self.active_view = View::ModList;
            }
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
        if let Some(preset_mods) = self.presets.get(&self.selected_preset) {
            self.current_mods = preset_mods.clone();
            self.save_mod_config_and_preset();
            let _ = logging::log(
                "INFO",
                &format!("Switched to preset: {}", self.selected_preset),
                "PresetManager",
            );
            logging::write_session_marker(&format!("PRESET_SWITCH:{}", self.selected_preset));
            // Re-check workshop mods for new preset
            self.check_workshop_mods_async();
        }
    }

    pub fn save_mod_config_and_preset(&mut self) {
        self.presets
            .insert(self.selected_preset.clone(), self.current_mods.clone());

        let noita_dir = self.get_active_noita_dir();
        if !noita_dir.is_empty() {
            let xml = mods::mods_to_xml(&self.current_mods);
            let _ = mods::write_mod_config(&PathBuf::from(&noita_dir), &xml);

            let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                self.file_watcher.last_modified_time = mtime;
            }
        }

        let _ = presets::save_presets(&self.presets);
        self.settings.selected_preset = self.selected_preset.clone();
        let _ = settings::save_settings(&self.settings);
    }

    pub fn apply_settings(&mut self, new_settings: AppSettings) {
        let noita_dir_changed = new_settings.noita_dir != self.settings.noita_dir;
        let backup_days_changed =
            new_settings.backup_settings.auto_delete_days != self.settings.backup_settings.auto_delete_days;
        self.dark_mode = new_settings.dark_mode;
        self.compact_mode = new_settings.compact_mode;
        self.settings = new_settings;
        let _ = settings::save_settings(&self.settings);

        if noita_dir_changed {
            self.reload_mods();
            self.check_workshop_mods_async();
        }

        // Run backup cleanup if auto-delete days changed
        if backup_days_changed {
            self.run_backup_cleanup_async();
        }

        // Update auto-backup timer
        let interval = self.settings.backup_settings.backup_interval_minutes;
        self.backup_state.auto_backup_due = if interval > 0 {
            Some(Instant::now() + Duration::from_secs(interval as u64 * 60))
        } else {
            None
        };
    }

    pub fn toggle_compact_mode(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        self.settings.compact_mode = self.compact_mode;
        let _ = settings::save_settings(&self.settings);

        if self.compact_mode {
            // Save current size before shrinking
            let current_size = ctx.input(|i| i.content_rect().size());
            if current_size.x > 500.0 {
                self.normal_window_size = Some(current_size);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(480.0, 400.0)));
        } else {
            let size = self.normal_window_size.unwrap_or(egui::vec2(1100.0, 800.0));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
    }

    pub fn reload_mods(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let dir = PathBuf::from(&noita_dir);
        if let Ok(xml) = mods::read_mod_config(&dir) {
            if let Ok(file_mods) = mods::parse_mods_from_xml(&xml) {
                self.current_mods = file_mods;
                self.presets
                    .insert(self.selected_preset.clone(), self.current_mods.clone());
                let _ = presets::save_presets(&self.presets);
            }
        }
        let config_path = dir.join("mod_config.xml");
        if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
            self.file_watcher.last_modified_time = mtime;
        }
    }

    pub fn get_active_noita_dir(&self) -> String {
        if cfg!(debug_assertions) {
            if let Ok(dev_dir) = platform::get_dev_save_dir() {
                return dev_dir.to_string_lossy().to_string();
            }
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
            let found_set: std::collections::HashSet<usize> = found_in_order.iter().copied().collect();
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
            .set_file_name(&format!("{}-mod-list.json", self.selected_preset))
            .add_filter("JSON", &["json"])
            .save_file();

        if let Some(path) = path {
            match serde_json::to_string_pretty(&enabled) {
                Ok(content) => {
                    if let Err(e) = mods::write_file(&path, &content) {
                        let _ = logging::log("ERROR", &format!("Export failed: {}", e), "ModManager");
                    } else {
                        let _ = logging::log(
                            "INFO",
                            &format!("Exported {} mods", enabled.len()),
                            "ModManager",
                        );
                    }
                }
                Err(e) => {
                    let _ = logging::log("ERROR", &format!("Serialization failed: {}", e), "ModManager");
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
        if let Some(ref checksum) = import_data.checksum {
            if let Ok(canonical) = serde_json::to_string(&import_data.presets) {
                if !gallery::verify_checksum(&canonical, checksum) {
                    let raw_presets_str =
                        serde_json::to_string(&import_data.presets).unwrap_or_default();
                    if !gallery::verify_checksum(&raw_presets_str, checksum) {
                        let import = PresetImportData {
                            presets: import_data.presets.clone(),
                            selected_names: import_data.presets.keys().cloned().collect(),
                        };
                        self.active_modal = Some(Modal::Confirm {
                            message: "Checksum mismatch: the preset file may have been modified. Continue?".to_string(),
                            confirm_text: "Continue".to_string(),
                            cancel_text: "Cancel".to_string(),
                            action: ConfirmAction::ChecksumMismatchContinue(import),
                            cancel_action: None,
                        });
                        return;
                    }
                }
            }
        }

        // Check for missing workshop mods across all presets
        let steam_path = &self.settings.gallery_settings.steam_path;
        if !steam_path.is_empty() {
            let all_workshop_ids: Vec<String> = import_data
                .presets
                .values()
                .flatten()
                .filter(|m| !m.workshop_id.is_empty() && m.workshop_id != "0")
                .map(|m| m.workshop_id.clone())
                .collect();

            if !all_workshop_ids.is_empty() {
                if let Ok(statuses) = workshop::check_workshop_mods_installed(&all_workshop_ids, steam_path) {
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
        }

        // Show checklist for which presets to import
        let names: Vec<String> = import_data.presets.keys().cloned().collect();
        let items: Vec<ChecklistItem> = names
            .iter()
            .map(|name| {
                let count = import_data.presets.get(name).map_or(0, |m| m.len());
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
            .collect();

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
                label: format!("{} ({:.1} MB)", b.filename, b.size_bytes as f64 / 1_048_576.0),
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

    fn run_auto_backup(&mut self) {
        let noita_dir = PathBuf::from(self.get_active_noita_dir());
        if noita_dir.as_os_str().is_empty() {
            return;
        }
        let tx = self.task_tx.clone();
        logging::write_session_marker("AUTO_BACKUP_START");
        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                backup::create_backup(&noita_dir, false, true, false, None)
            })
            .await
            .unwrap_or_else(|e| Err(format!("Auto-backup task failed: {}", e)));
            let _ = tx.send(TaskResult::AutoBackupComplete(result));
        });
    }

    // ── Save Monitor ───────────────────────────────────────────────────

    pub fn start_save_monitor(&mut self) {
        self.save_monitor.running = true;
        self.save_monitor.snapshot_count = 0;
        let _ = logging::log("INFO", "Save Monitor started", "SaveMonitor");
        logging::write_session_marker("MONITOR_START");
        self.take_monitor_snapshot();
    }

    pub fn stop_save_monitor(&mut self) {
        self.save_monitor.running = false;
        let _ = logging::log("INFO", "Save Monitor stopped", "SaveMonitor");
        logging::write_session_marker("MONITOR_STOP");
    }

    fn take_monitor_snapshot(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let preset_name = self.selected_preset.clone();
        let include_save01 = self.settings.save_monitor_settings.include_save01;
        let include_entangled = self.settings.save_monitor_settings.include_entangled;
        let entangled_dir = if include_entangled {
            self.get_active_entangled_dir()
        } else {
            None
        };
        let tx = self.task_tx.clone();

        self.async_runtime.spawn(async move {
            let result = tokio::task::spawn_blocking(move || {
                save_monitor::create_monitor_snapshot(
                    &noita_dir,
                    &preset_name,
                    include_save01,
                    include_entangled,
                    entangled_dir.as_deref(),
                )
            })
            .await
            .unwrap_or_else(|e| Err(format!("Snapshot task failed: {}", e)));
            let _ = tx.send(TaskResult::SnapshotComplete(result));
        });

        self.save_monitor.last_snapshot = Some(Instant::now());
    }

    // ── Gallery ────────────────────────────────────────────────────────

    pub fn fetch_catalog(&mut self) {
        let url = self.settings.gallery_settings.catalog_url.clone();
        if url.is_empty() {
            self.gallery_state.error = Some("Catalog URL not configured".to_string());
            return;
        }

        // 5-minute cache
        if let Some(fetched_at) = self.gallery_state.catalog_fetched_at {
            if Instant::now().duration_since(fetched_at) < Duration::from_secs(300) {
                return;
            }
        }

        self.gallery_state.loading = true;
        self.gallery_state.error = None;
        let tx = self.task_tx.clone();

        self.async_runtime.spawn(async move {
            let result = gallery::fetch_catalog(&url).await;
            let _ = tx.send(TaskResult::CatalogFetched(result));
        });
    }

    pub fn download_and_import_preset(&mut self, download_url: &str, _checksum: &str) {
        let url = download_url.to_string();
        let tx = self.task_tx.clone();

        self.async_runtime.spawn(async move {
            let result = gallery::download_preset_file(&url).await;
            let _ = tx.send(TaskResult::PresetDownloaded(result));
        });
    }

    fn handle_downloaded_preset(&mut self, content: &str) {
        let import_data: PresetExportFile = match serde_json::from_str(content) {
            Ok(d) => d,
            Err(e) => {
                self.active_modal = Some(Modal::Info {
                    title: "Import Failed".to_string(),
                    message: format!("Invalid preset data: {}", e),
                });
                return;
            }
        };

        if import_data.presets.is_empty() {
            self.active_modal = Some(Modal::Info {
                title: "Import".to_string(),
                message: "No presets found in downloaded file.".to_string(),
            });
            return;
        }

        // Verify checksum
        if let Some(ref checksum) = import_data.checksum {
            if let Ok(canonical) = serde_json::to_string(&import_data.presets) {
                if !gallery::verify_checksum(&canonical, checksum) {
                    let _ = logging::log("WARN", "Checksum mismatch on downloaded preset", "Gallery");
                }
            }
        }

        // Import all presets
        for (name, mods_list) in &import_data.presets {
            let mut target_name = name.clone();
            if self.presets.contains_key(&target_name) {
                target_name = format!("{} (imported)", name);
                let mut counter = 2;
                while self.presets.contains_key(&target_name) {
                    target_name = format!("{} (imported {})", name, counter);
                    counter += 1;
                }
            }
            self.presets.insert(target_name, mods_list.clone());
        }

        let _ = presets::save_presets(&self.presets);
        let _ = logging::log(
            "INFO",
            &format!("Imported {} preset(s) from modpacks", import_data.presets.len()),
            "Gallery",
        );
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
                    let deleted = self.selected_preset.clone();
                    self.presets.remove(&deleted);
                    self.selected_preset = "Default".to_string();
                    self.current_mods = self.presets.get("Default").cloned().unwrap_or_default();
                    self.save_mod_config_and_preset();
                    let _ = logging::log(
                        "INFO",
                        &format!("Deleted preset: {}", deleted),
                        "PresetManager",
                    );
                }
            }
            ConfirmAction::DeleteMod(idx) => {
                if idx < self.current_mods.len() {
                    let name = self.current_mods[idx].name.clone();
                    self.current_mods.remove(idx);
                    self.save_mod_config_and_preset();
                    let _ = logging::log("INFO", &format!("Deleted mod: {}", name), "ModManager");
                }
            }
            ConfirmAction::AcceptExternalChanges(file_mods) => {
                self.current_mods = file_mods;
                self.presets
                    .insert(self.selected_preset.clone(), self.current_mods.clone());
                let _ = presets::save_presets(&self.presets);
            }
            ConfirmAction::KeepCurrentPreset => {
                self.save_mod_config_and_preset();
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
                self.take_monitor_snapshot();
                self.stop_save_monitor();
                self.close_requested = false;
            }
            ConfirmAction::ExitWithoutSnapshot => {
                self.stop_save_monitor();
                self.close_requested = false;
            }
            ConfirmAction::DeleteBackup(filename) => {
                self.delete_backup_async(filename);
            }
            ConfirmAction::ClearMonitorData => {
                self.clear_monitor_data_async();
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
                    self.presets.insert(value.clone(), self.current_mods.clone());
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
                        let item = self.current_mods.remove(from_idx);
                        self.current_mods.insert(target_idx, item);
                        self.save_mod_config_and_preset();
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

                if let Some(path) = path {
                    if let Ok(content) = serde_json::to_string_pretty(&export) {
                        let _ = mods::write_file(&path, &content);
                        let _ = logging::log(
                            "INFO",
                            &format!("Exported {} preset(s)", selected.len()),
                            "PresetManager",
                        );
                    }
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
                let names: Vec<String> = import.presets.keys().cloned().collect();
                let items: Vec<ChecklistItem> = names
                    .iter()
                    .map(|name| {
                        let count = import.presets.get(name).map_or(0, |m| m.len());
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
                    .collect();
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
                if !overwrite && self.presets.contains_key(&target_name) {
                    target_name = format!("{} (imported)", name);
                    let mut counter = 2;
                    while self.presets.contains_key(&target_name) {
                        target_name = format!("{} (imported {})", name, counter);
                        counter += 1;
                    }
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

    // ── Cleanup ────────────────────────────────────────────────────────

    pub fn cleanup_on_exit(&mut self) {
        let _ = logging::log("INFO", "Application shutting down", "App");
        logging::write_session_marker("APP_SHUTDOWN");

        let _ = logging::flush_log_buffer_sync();
        logging::write_session_end_marker();
        let _ = logging::flush_log_buffer_sync();
    }
}

impl eframe::App for HallintaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        if !self.compact_mode {
            crate::ui::sidebar::render_sidebar(self, ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_view {
                View::ModList => {
                    if self.compact_mode {
                        // Compact mode: show only monitor status
                        ui.heading("Save Monitor");
                        if self.save_monitor.is_running() {
                            ui.colored_label(egui::Color32::GREEN, "Running");
                            ui.label(format!("Snapshots: {}", self.save_monitor.snapshot_count));
                            if ui.button("Stop Monitor").clicked() {
                                self.stop_save_monitor();
                            }
                        } else if ui.button("Start Monitor").clicked() {
                            self.start_save_monitor();
                        }
                    } else if self.save_monitor.is_running() {
                        // Monitor running: show monitor status instead of mod list
                        crate::ui::mod_list::render_monitor_active(self, ui);
                    } else {
                        crate::ui::mod_list::render_mod_list(self, ui);
                    }
                }
                View::PresetVault => {
                    if self.save_monitor.is_running() {
                        crate::ui::mod_list::render_monitor_active(self, ui);
                    } else {
                        crate::ui::gallery::render_gallery(self, ui);
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
