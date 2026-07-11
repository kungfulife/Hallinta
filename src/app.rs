use crate::core::{backup, logging, mods, platform, presets, save_monitor, settings};
use crate::models::*;
use crate::tasks::TaskResult;
use eframe::egui;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;

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
#[cfg(test)]
pub(crate) mod test_support;
mod timers;
mod update;

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
    pub noita_directory_error: Option<String>,
    #[cfg(debug_assertions)]
    pub preview_noita_directory_warning: bool,

    // Modal state
    pub active_modal: Option<Modal>,

    // Feature state
    pub save_monitor: SaveMonitorState,
    pub backup_state: BackupState,
    pub file_watcher: FileWatcherState,
    pub update_state: UpdateState,

    // Async coordination
    pub async_runtime: tokio::runtime::Handle,
    pub task_tx: mpsc::Sender<TaskResult>,
    pub task_rx: mpsc::Receiver<TaskResult>,
    pending_mod_list_export: Option<(String, Vec<ModListEntry>)>,
    pending_preset_export: Option<Vec<String>>,

    // Drag state
    pub drag_state: Option<DragState>,

    // Normal mode window size (for restoring after compact)
    normal_window_size: Option<egui::Vec2>,

    // Deferred viewport resizing. The OS can clamp a resize against the
    // previous min-size if both commands land in one frame, so size changes
    // are applied over two follow-up frames.
    deferred_viewport_action: Option<DeferredViewportAction>,
    pending_viewport_center: Option<(f32, f32)>,

    // Track whether close was requested while monitor is running
    close_requested: bool,
    close_after_snapshot: bool,
    pending_close_snapshot_id: Option<u64>,
    update_handoff: Option<UpdateHandoff>,
    startup_ready_path: Option<PathBuf>,

    // Keyboard / focus signals
    pub focus_search_requested: bool,

    // Window focus tracking. File watcher pauses while unfocused and forces an
    // immediate check on regaining focus (matches the JS version's behaviour
    // and avoids polling a directory the user isn't looking at).
    was_focused: bool,
}

impl HallintaApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        rt: tokio::runtime::Handle,
        startup_ready_path: Option<PathBuf>,
        startup_monitor_resume: Option<MonitorResume>,
        startup_update_error_path: Option<PathBuf>,
    ) -> Self {
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
                save_monitor_settings: SaveMonitorSettings::default(),
                steam_path: String::new(),
                compact_mode: false,
                ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
                last_filter_mode: String::new(),
                last_sort_mode: String::new(),
            }
        });
        logging::configure(&app_settings.log_settings);

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

        let mut noita_directory_error = None;
        if platform::is_configured_path(&noita_dir) {
            let noita_path = PathBuf::from(&noita_dir);
            match mods::load_mod_config(&noita_path) {
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
                    noita_directory_error = Some(noita_directory_error_message(&noita_dir, &e));
                    let _ = logging::log(
                        "WARN",
                        noita_directory_error.as_deref().unwrap_or(&e),
                        "Mods",
                    );
                }
            }
        } else {
            noita_directory_error = Some(noita_directory_error_message(&noita_dir, ""));
            let _ = logging::log(
                "WARN",
                noita_directory_error
                    .as_deref()
                    .unwrap_or("Noita save directory was not found."),
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
        if platform::is_configured_path(&noita_dir) {
            let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                file_watcher_state.last_modified_time = mtime;
            }
        }

        let backup_state = BackupState::new();

        if let Ok(fixed) = save_monitor::reconcile_interrupted_sessions()
            && fixed > 0
        {
            let _ = logging::log(
                "INFO",
                &format!(
                    "Reconciled {} interrupted monitor session(s) to Stopped",
                    fixed
                ),
                "SaveMonitor",
            );
        }

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
            noita_directory_error,
            #[cfg(debug_assertions)]
            preview_noita_directory_warning: false,
            active_modal: None,
            save_monitor: save_monitor_state,
            backup_state,
            file_watcher: file_watcher_state,
            update_state: UpdateState::default(),
            async_runtime: rt,
            task_tx,
            task_rx,
            pending_mod_list_export: None,
            pending_preset_export: None,
            drag_state: None,
            normal_window_size: None,
            deferred_viewport_action: None,
            pending_viewport_center: None,
            close_requested: false,
            close_after_snapshot: false,
            pending_close_snapshot_id: None,
            update_handoff: None,
            startup_ready_path,
            focus_search_requested: false,
            was_focused: true,
        };

        // Start monitor if configured
        if let Some(resume) = startup_monitor_resume {
            app.resume_monitor_session_for(&resume.preset_name, &resume.session_id);
        } else if app.settings.save_monitor_settings.start_in_monitor_mode {
            app.start_save_monitor();
        }

        // Check workshop mods on startup
        app.check_workshop_mods_async();

        // Load backup list async
        app.load_backup_list_async();

        // Distribution builds check quietly at startup. Manual checks remain
        // available in Settings after a network failure.
        if platform::is_dist_build() {
            app.check_for_updates(false);
        }

        if let Some(path) = startup_update_error_path {
            let message = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                format!("The update failed, and its detailed error could not be read: {error}")
            });
            let _ = std::fs::remove_file(path);
            app.update_state.generation = app.update_state.generation.wrapping_add(1);
            app.update_state.status = UpdateStatus::Failed {
                message,
                retryable: true,
            };
        }

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
        logging::write_session_marker(&format!("APP_INITIALIZED:v{}", platform::get_version()));
        app
    }
}

#[cfg(debug_assertions)]
const NOITA_WARNING_PREVIEW_MESSAGE: &str =
    "Preview: Hallinta could not load mod_config.xml from the detected Noita save directory.";

fn noita_directory_error_message(path: &str, error: &str) -> String {
    if path.trim().is_empty() {
        "Noita save directory was not found.".to_string()
    } else {
        format!("Could not load mod_config.xml from {path}: {error}")
    }
}

struct UpdateHandoff {
    child: std::process::Child,
    ack_path: PathBuf,
    staging_path: PathBuf,
    helper_path: PathBuf,
    rollback_path: PathBuf,
    ready_path: PathBuf,
    handoff_path: PathBuf,
    started: std::time::Instant,
}

#[derive(Clone, Copy)]
enum DeferredViewportAction {
    ResizeThenMin {
        inner_size: egui::Vec2,
        min_size: egui::Vec2,
    },
    ApplyMin {
        min_size: egui::Vec2,
    },
    Reposition {
        outer_pos: egui::Pos2,
    },
}
