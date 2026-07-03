use super::HallintaApp;
use crate::models::{
    AppSettings, BackupState, FileWatcherState, FilterMode, LogSettings, ModEntry,
    SaveMonitorSettings, SaveMonitorState, SortMode, View,
};
use std::collections::BTreeMap;
use std::sync::mpsc;

pub(crate) fn mod_entry(name: &str, enabled: bool, workshop_id: &str) -> ModEntry {
    ModEntry {
        name: name.to_string(),
        enabled,
        workshop_id: workshop_id.to_string(),
        settings_fold_open: false,
    }
}

fn default_settings() -> AppSettings {
    AppSettings {
        noita_dir: String::new(),
        entangled_dir: String::new(),
        dark_mode: false,
        selected_preset: "Default".to_string(),
        version: "test".to_string(),
        log_settings: LogSettings::default(),
        save_monitor_settings: SaveMonitorSettings::default(),
        steam_path: String::new(),
        compact_mode: false,
        ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
        last_filter_mode: String::new(),
        last_sort_mode: String::new(),
    }
}

pub(crate) fn test_app(current_mods: Vec<ModEntry>) -> (tokio::runtime::Runtime, HallintaApp) {
    let runtime = tokio::runtime::Runtime::new().expect("test runtime should start");
    let (task_tx, task_rx) = mpsc::channel();
    let app = HallintaApp {
        settings: default_settings(),
        presets: BTreeMap::new(),
        current_mods,
        selected_preset: "Default".to_string(),
        active_view: View::ModList,
        search_query: String::new(),
        filter_mode: FilterMode::All,
        sort_mode: SortMode::Default,
        compact_mode: false,
        dark_mode: false,
        active_modal: None,
        save_monitor: SaveMonitorState::new(),
        backup_state: BackupState::new(),
        file_watcher: FileWatcherState::new(),
        async_runtime: runtime.handle().clone(),
        task_tx,
        task_rx,
        drag_state: None,
        normal_window_size: None,
        deferred_viewport_action: None,
        pending_viewport_center: None,
        close_requested: false,
        close_after_snapshot: false,
        focus_search_requested: false,
        was_focused: true,
    };
    (runtime, app)
}
