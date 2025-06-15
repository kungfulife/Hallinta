use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use tokio::fs as tokio_fs;
use zip::write::FileOptions;
use zip::ZipWriter;

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

static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static LOG_FILE_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static MAX_BUFFER_SIZE: usize = 1000;

fn get_data_dir() -> Result<PathBuf, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Could not find local data directory.".to_string())?
        .join("Hallinta");

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
    let context: tauri::Context<tauri_runtime_wry::Wry<tauri::EventLoopMessage>> =
        tauri::generate_context!();
    context.package_info().version.to_string()
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

    fs::read_to_string(config_path).map_err(|e| format!("Failed to read mod_config.xml: {}", e))
}

#[tauri::command]
fn write_mod_config(directory: String, content: String) -> Result<(), String> {
    let config_path = PathBuf::from(directory).join("mod_config.xml");

    fs::write(config_path, content).map_err(|e| format!("Failed to write mod_config.xml: {}", e))
}

#[tauri::command]
fn check_file_exists(path: String) -> Result<bool, String> {
    let path = Path::new(&path);
    Ok(path.exists())
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
    let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;

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

    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;

    let current_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
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

    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;

    let current_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();

    Ok(current_time)
}

async fn create_upgrade_backup(
    settings: AppSettings,
    presets: std::collections::HashMap<String, Vec<ModPreset>>,
    old_version: String,
    new_version: String,
) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let upgrade_backup_dir = data_dir.join("upgrade_backups");
    if !upgrade_backup_dir.exists() {
        tokio_fs::create_dir_all(&upgrade_backup_dir)
            .await
            .map_err(|e| format!("Failed to create upgrade backup directory: {}", e))?;
    }
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let zip_file_path = upgrade_backup_dir.join(format!(
        "upgrade_backup_from_v{}_to_v{}_{}.zip",
        old_version, new_version, timestamp
    ));

    let settings_json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    let presets_json = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;

    let zip_file_path_clone = zip_file_path.clone();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_file_path_clone)
            .map_err(|e| format!("Failed to create zip file: {}", e))?;
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("settings.json", options)
            .map_err(|e| format!("Failed to start file in zip: {}", e))?;
        zip.write_all(settings_json.as_bytes())
            .map_err(|e| format!("Failed to write settings to zip: {}", e))?;

        zip.start_file("presets.json", options)
            .map_err(|e| format!("Failed to start file in zip: {}", e))?;
        zip.write_all(presets_json.as_bytes())
            .map_err(|e| format!("Failed to write presets to zip: {}", e))?;

        zip.finish()
            .map_err(|e| format!("Failed to finish zip: {}", e))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Failed to create upgrade backup: {}", e))??;

    Ok(())
}

#[tauri::command]
fn add_log_entry(level: String, message: String, module: String) -> Result<(), String> {
    let timestamp = Utc::now().to_rfc3339();
    let entry = LogEntry {
        timestamp,
        level: level.clone(),
        message: message.clone(),
        module,
    };

    let mut buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    if buffer.len() >= MAX_BUFFER_SIZE {
        buffer.pop_front();
    }
    buffer.push_back(entry.clone());

    let mut file_buffer = LOG_FILE_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
    file_buffer.push_back(entry);

    Ok(())
}

#[tauri::command]
fn get_log_entries() -> Result<Vec<LogEntry>, String> {
    let buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    Ok(buffer.iter().cloned().collect())
}

#[tauri::command]
fn clear_log_buffer() -> Result<(), String> {
    let mut buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    let mut file_buffer = LOG_FILE_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
    buffer.clear();
    file_buffer.clear();
    Ok(())
}

#[tauri::command]
async fn flush_log_buffer() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    if !logs_dir.exists() {
        tokio_fs::create_dir_all(&logs_dir)
            .await
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    let logs = {
        let mut file_buffer = LOG_FILE_BUFFER
            .lock()
            .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
        if file_buffer.is_empty() {
            return Ok(());
        }
        file_buffer.drain(..).collect::<Vec<_>>()
    };

    let mut logs_by_date: std::collections::HashMap<String, Vec<LogEntry>> =
        std::collections::HashMap::new();
    for entry in logs {
        let log_time: DateTime<Utc> = entry
            .timestamp
            .parse()
            .map_err(|e| format!("Failed to parse timestamp: {}", e))?;
        let local_date = log_time
            .with_timezone(&Local)
            .date_naive()
            .format("%Y%m%d")
            .to_string();
        logs_by_date
            .entry(local_date)
            .or_insert(Vec::new())
            .push(entry);
    }

    for (date, entries) in logs_by_date {
        let version = get_version();
        let log_file = logs_dir.join(format!("hallinta_v{}_{}.log", version, date));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| format!("Failed to open log file {}: {}", log_file.display(), e))?;

        for entry in entries {
            let log_line = format!(
                "[{}] [{}] [{}] {}\n",
                entry.timestamp, entry.level, entry.module, entry.message
            );
            file.write_all(log_line.as_bytes()).map_err(|e| {
                format!("Failed to write to log file {}: {}", log_file.display(), e)
            })?;
        }
    }

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

    let mut settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;

    if settings.version != get_version() {
        let old_version = settings.version.clone();
        let new_version = get_version();
        let presets_path = data_dir.join("presets.json");
        let presets = if presets_path.exists() {
            let presets_content = fs::read_to_string(&presets_path)
                .map_err(|e| format!("Failed to read presets file: {}", e))?;
            serde_json::from_str(&presets_content)
                .map_err(|e| format!("Failed to parse presets: {}", e))?
        } else {
            std::collections::HashMap::new()
        };
        create_upgrade_backup(settings.clone(), presets, old_version, new_version).await?;
        add_log_entry(
            "INFO".to_string(),
            "Version update detected, created upgrade backup".to_string(),
            "SettingsManager".to_string(),
        )?;
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

    let presets: std::collections::HashMap<String, Vec<ModPreset>> =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse presets: {}", e))?;

    Ok(presets)
}

#[tauri::command]
fn open_workshop_item(workshop_id: String) -> Result<(), String> {
    if workshop_id == "0" || workshop_id.is_empty() {
        return Err("No workshop ID provided.".to_string());
    }

    let url = format!(
        "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
        workshop_id
    );

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
            add_log_entry,
            get_log_entries,
            clear_log_buffer,
            flush_log_buffer,
            check_file_exists
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
