use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct BackupSettings {
    pub auto_delete_days: u32,
    pub backup_interval_minutes: u32,
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
    pub log_settings: LogSettings,
    #[serde(default)]
    pub backup_settings: BackupSettings,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LogSettings {
    pub max_log_files: usize,
    pub max_log_size_mb: usize,
    pub log_level: String,
    pub auto_save: bool,
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
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RestoreOptions {
    pub restore_save00: bool,
    pub restore_save01: bool,
    pub restore_presets: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SystemInfo {
    pub app_version: String,
    pub build_mode: String,
    pub rust_version: String,
    pub cargo_version: String,
    pub target_triple: String,
    pub tauri_version: String,
    pub os: String,
    pub arch: String,
    pub data_dir: String,
}
