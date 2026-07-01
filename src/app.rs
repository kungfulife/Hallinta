use crate::core::{backup, logging, mods, platform, presets, settings};
use crate::models::*;
use crate::tasks::TaskResult;
use eframe::egui;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Instant;

mod actions;
mod async_tasks;
mod backup_actions;
mod import_export;
mod input;
mod lifecycle;
mod modal_actions;
mod monitor;
mod sorting;
mod task_results;
mod timers;

pub use sorting::sort_mods;

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

        // Version upgrade check
        let old_version = app_settings.version.clone();
        if settings::check_and_upgrade_version(&mut app_settings).unwrap_or(false) {
            let tx = task_tx.clone();
            let s = app_settings.clone();
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

        let noita_dir = app_settings.noita_dir.clone();

        // Load mods from the configured Noita directory.
        let selected_preset = app_settings.selected_preset.clone();
        let mut current_mods = app_presets
            .get(&selected_preset)
            .cloned()
            .unwrap_or_default();

        if !noita_dir.is_empty() {
            let noita_path = PathBuf::from(&noita_dir);
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
        if !noita_dir.is_empty() {
            let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
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
                "Loaded preset \"{}\" with {} mods ({} enabled)",
                app.selected_preset,
                app.current_mods.len(),
                app.current_mods.iter().filter(|m| m.enabled).count(),
            ),
            "App",
        );
        let bs = &app.settings.backup_settings;
        let _ = logging::log(
            "INFO",
            &format!(
                "Backup config: auto-delete {} | auto-backup {}",
                if bs.auto_delete_days > 0 {
                    format!("after {}d", bs.auto_delete_days)
                } else {
                    "off".to_string()
                },
                if bs.backup_interval_minutes > 0 {
                    format!("every {}min", bs.backup_interval_minutes)
                } else {
                    "off".to_string()
                },
            ),
            "Backup",
        );
        logging::write_session_marker(&format!("APP_INITIALIZED:v{}", platform::get_version()));
        app
    }
}
