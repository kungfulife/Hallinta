use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use std::time::SystemTime;
use std::path::Path;
use tokio::fs as tokio_fs;
use serde_json;
use chrono::Utc;
use std::sync::Mutex;
use std::collections::VecDeque;

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub noita_dir: String,
    pub entangled_dir: String,
    pub dark_mode: bool,
    pub selected_preset: String,
    pub version: String,
    pub log_settings: LogSettings,
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

// Logging state
static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static MAX_BUFFER_SIZE: usize = 1000;

fn get_data_dir() -> Result<PathBuf, String> {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|_| "Could not get LOCALAPPDATA environment variable.".to_string())?;

    let data_dir = PathBuf::from(local_app_data).join("Hallinta");

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}

#[tauri::command]
fn is_dev_build() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
fn get_version() -> String {
    "0.3.0".to_string()
}

#[tauri::command]
fn get_app_settings_dir() -> Result<String, String> {
    let data_dir = get_data_dir()?;
    Ok(data_dir.to_string_lossy().to_string())
}

#[tauri::command]
fn read_mod_config(directory: String) -> Result<String, String> {
    let config_path = PathBuf::from(directory).join("mod_config.xml");
    if !config_path.exists() {
        return Err("mod_config.xml not found in directory.".to_string());
    }

    fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read mod_config.xml: {}", e))
}

#[tauri::command]
fn write_mod_config(directory: String, content: String) -> Result<(), String> {
    let config_path = PathBuf::from(directory).join("mod_config.xml");

    fs::write(config_path, content)
        .map_err(|e| format!("Failed to write mod_config.xml: {}", e))
}

#[tauri::command]
fn get_exe_dir() -> Result<String, String> {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(parent) = exe_path.parent() {
                Ok(parent.to_string_lossy().to_string())
            } else {
                Err("Could not get parent directory.".to_string())
            }
        }
        Err(e) => Err(format!("Could not get executable path: {}", e)),
    }
}

#[tauri::command]
fn get_noita_save_path() -> Result<String, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "Failed to get home directory.".to_string())?;

    Ok(home_dir
        .join("AppData")
        .join("LocalLow")
        .join("Nolla_Games_Noita")
        .join("save00")
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
async fn open_directory(directory: String) -> Result<(), String> {
    let path = Path::new(&directory);
    if !path.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn check_file_modified(file_path: String, last_modified: u64) -> Result<bool, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Ok(false);
    }

    let metadata = tokio_fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;

    let modified = metadata.modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;

    let current_time = modified.duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();

    Ok(current_time > last_modified)
}

#[tauri::command]
async fn get_file_modified_time(file_path: String) -> Result<u64, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    let metadata = tokio_fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;

    let modified = metadata.modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;

    let current_time = modified.duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();

    Ok(current_time)
}

#[tauri::command]
async fn create_settings_backup(settings: AppSettings) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_dir = data_dir.join("backups");

    if !backup_dir.exists() {
        tokio_fs::create_dir_all(&backup_dir)
            .await
            .map_err(|e| format!("Failed to create backup directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_file = backup_dir.join(format!("settings_backup_{}.json", timestamp));

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    tokio_fs::write(&backup_file, json_content)
        .await
        .map_err(|e| format!("Failed to write backup file: {}", e))?;

    cleanup_old_backups(&backup_dir, 10).await?;

    Ok(())
}

async fn cleanup_old_backups(backup_dir: &Path, keep_count: usize) -> Result<(), String> {
    let mut entries = tokio_fs::read_dir(backup_dir)
        .await
        .map_err(|e| format!("Failed to read backup directory: {}", e))?;

    let mut backup_files = Vec::new();

    while let Some(entry) = entries.next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {}", e))? {

        let path = entry.path();
        if path.is_file() && path.file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n.starts_with("settings_backup_")) {

            let metadata = entry.metadata()
                .await
                .map_err(|e| format!("Failed to get file metadata: {}", e))?;

            backup_files.push((path, metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
        }
    }

    if backup_files.len() > keep_count {
        backup_files.sort_by(|a, b| b.1.cmp(&a.1));

        for (path, _) in backup_files.iter().skip(keep_count) {
            tokio_fs::remove_file(path)
                .await
                .map_err(|e| format!("Failed to remove old backup: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
fn add_log_entry(level: String, message: String, module: String) -> Result<(), String> {
    let entry = LogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level,
        message,
        module,
    };

    let mut buffer = LOG_BUFFER.lock().map_err(|e| format!("Failed to lock log buffer: {}", e))?;

    if buffer.len() >= MAX_BUFFER_SIZE {
        buffer.pop_front();
    }

    buffer.push_back(entry);

    Ok(())
}

#[tauri::command]
fn get_log_entries() -> Result<Vec<LogEntry>, String> {
    let buffer = LOG_BUFFER.lock().map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    Ok(buffer.iter().cloned().collect())
}

#[tauri::command]
async fn save_logs_to_file() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");

    if !logs_dir.exists() {
        tokio_fs::create_dir_all(&logs_dir)
            .await
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    // Clone the log entries to avoid holding the mutex across await
    let log_entries = {
        let buffer = LOG_BUFFER.lock().map_err(|e| format!("Failed to lock log buffer: {}", e))?;

        if buffer.is_empty() {
            return Ok(());
        }

        buffer.iter().cloned().collect::<Vec<_>>()
    }; // Mutex guard is dropped here

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let log_file = logs_dir.join(format!("hallinta_{}.log", timestamp));

    let mut log_content = String::new();
    for entry in log_entries.iter() {
        log_content.push_str(&format!("[{}] [{}] [{}] {}\n",
                                      entry.timestamp, entry.level, entry.module, entry.message));
    }

    tokio_fs::write(&log_file, log_content)
        .await
        .map_err(|e| format!("Failed to write log file: {}", e))?;

    Ok(())
}

#[tauri::command]
fn clear_log_buffer() -> Result<(), String> {
    let mut buffer = LOG_BUFFER.lock().map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    buffer.clear();
    Ok(())
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(settings_path, json_content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn load_settings() -> Result<AppSettings, String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    if !settings_path.exists() {
        let default_settings = AppSettings {
            noita_dir: get_noita_save_path()?,
            entangled_dir: String::new(),
            dark_mode: false,
            selected_preset: "Default".to_string(),
            version: get_version(),
            log_settings: LogSettings {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: "INFO".to_string(),
                auto_save: true,
            },
        };
        save_settings(default_settings.clone())?;
        return Ok(default_settings);
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let mut settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;

    if settings.version != get_version() {
        let _ = create_settings_backup(settings.clone()).await;
        settings.version = get_version();
        save_settings(settings.clone())?;
    }

    Ok(settings)
}

#[tauri::command]
fn save_presets(presets: std::collections::HashMap<String, Vec<ModPreset>>) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let presets_path = data_dir.join("presets.json");

    let json_content = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;

    fs::write(presets_path, json_content)
        .map_err(|e| format!("Failed to write presets file: {}", e))?;

    Ok(())
}

#[tauri::command]
fn load_presets() -> Result<std::collections::HashMap<String, Vec<ModPreset>>, String> {
    let data_dir = get_data_dir()?;
    let presets_path = data_dir.join("presets.json");

    if !presets_path.exists() {
        let mut default_presets = std::collections::HashMap::new();
        default_presets.insert("Default".to_string(), Vec::new());
        save_presets(default_presets.clone())?;
        return Ok(default_presets);
    }

    let content = fs::read_to_string(&presets_path)
        .map_err(|e| format!("Failed to read presets file: {}", e))?;

    let presets: std::collections::HashMap<String, Vec<ModPreset>> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse presets: {}", e))?;

    Ok(presets)
}

#[tauri::command]
fn open_workshop_item(workshop_id: String) -> Result<(), String> {
    if workshop_id == "0" || workshop_id.is_empty() {
        return Err("No workshop ID provided.".to_string());
    }

    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", workshop_id);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/c", "start", &url])
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_exe_dir,
            get_noita_save_path,
            get_app_settings_dir,
            open_workshop_item,
            save_settings,
            load_settings,
            save_presets,
            load_presets,
            read_mod_config,
            write_mod_config,
            is_dev_build,
            get_version,
            open_directory,
            check_file_modified,
            get_file_modified_time,
            create_settings_backup,
            add_log_entry,
            get_log_entries,
            save_logs_to_file,
            clear_log_buffer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
