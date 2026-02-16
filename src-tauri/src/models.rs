use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupSettings {
    pub auto_delete_days: u32,
    pub backup_interval_minutes: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveMonitorSettings {
    pub interval_minutes: u32,
    pub max_snapshots_per_preset: usize,
    pub include_entangled: bool,
}

impl Default for SaveMonitorSettings {
    fn default() -> Self {
        SaveMonitorSettings {
            interval_minutes: 15,
            max_snapshots_per_preset: 10,
            include_entangled: false,
        }
    }
}

impl Default for BackupSettings {
    fn default() -> Self {
        BackupSettings {
            auto_delete_days: 30,
            backup_interval_minutes: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub noita_dir: String,
    pub entangled_dir: String,
    pub dark_mode: bool,
    pub selected_preset: String,
    pub version: String,
    #[serde(default)]
    pub log_settings: LogSettings,
    #[serde(default)]
    pub backup_settings: BackupSettings,
    #[serde(default)]
    pub save_monitor_settings: SaveMonitorSettings,
    #[serde(default)]
    pub gallery_settings: GallerySettings,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GallerySettings {
    pub catalog_url: String,
    pub steam_path: String,
}

impl Default for GallerySettings {
    fn default() -> Self {
        GallerySettings {
            catalog_url: String::new(),
            steam_path: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CatalogPresetEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub mod_count: usize,
    pub version: String,
    pub checksum: String,
    pub download_url: String,
    pub thumbnail_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Catalog {
    pub catalog_version: String,
    pub last_updated: String,
    pub presets: Vec<CatalogPresetEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkshopModStatus {
    pub workshop_id: String,
    pub installed: bool,
}

fn default_collect_system_info() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogSettings {
    pub max_log_files: usize,
    pub max_log_size_mb: usize,
    pub log_level: String,
    pub auto_save: bool,
    #[serde(default = "default_collect_system_info")]
    pub collect_system_info: bool,
}

impl Default for LogSettings {
    fn default() -> Self {
        let default_log_level = if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "INFO"
        };
        LogSettings {
            max_log_files: 50,
            max_log_size_mb: 10,
            log_level: default_log_level.to_string(),
            auto_save: true,
            collect_system_info: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModPreset {
    pub name: String,
    pub enabled: bool,
    pub workshop_id: String,
    pub settings_fold_open: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub module: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionLock {
    pub created_at: String,
    pub dev_mode_active: bool,
    pub original_mod_config_path: String,
    pub pid: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupInfo {
    pub filename: String,
    pub timestamp: String,
    pub size_bytes: u64,
    pub contains_save00: bool,
    pub contains_save01: bool,
    pub contains_presets: bool,
    #[serde(default)]
    pub contains_entangled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RestoreOptions {
    pub restore_save00: bool,
    pub restore_save01: bool,
    pub restore_presets: bool,
    #[serde(default)]
    pub restore_entangled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub app_version: String,
    pub build_profile: String,
    pub rust_version: String,
    pub cargo_version: String,
    pub build_target: String,
    pub tauri_version: String,
    pub os: String,
    pub os_family: String,
    pub arch: String,
    pub logical_cpu_cores: usize,
    pub local_time: String,
    pub utc_time: String,
    pub executable_dir: String,
    pub app_data_dir: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct OpenSourceLibrary {
    pub name: String,
    pub version: String,
    pub purpose: String,
    pub homepage: String,
}
