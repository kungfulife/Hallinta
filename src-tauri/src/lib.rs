use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{Read as IoRead, Write};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;
use tokio::fs as tokio_fs;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

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

static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static LOG_FILE_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static MAX_BUFFER_SIZE: usize = 1000;
static INSTANCE_ID: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
});
fn get_data_dir() -> Result<PathBuf, String> {
    let data_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "Could not get project root directory".to_string())?
            .join("dev_data")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| "Could not find local data directory.".to_string())?
            .join("Hallinta")
    };

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
fn get_dev_save_dir(source_noita_dir: String) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Not in dev mode".to_string());
    }

    let dev_data = get_data_dir()?;

    let config_path = dev_data.join("mod_config.xml");
    if !config_path.exists() {
        let source_config = PathBuf::from(&source_noita_dir).join("mod_config.xml");
        if !source_noita_dir.is_empty() && source_config.exists() {
            fs::copy(&source_config, &config_path)
                .map_err(|e| format!("Failed to copy mod_config.xml to dev_data: {}", e))?;
        } else {
            let sample_config =
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n</Mods>";
            fs::write(&config_path, sample_config)
                .map_err(|e| format!("Failed to create sample mod_config.xml: {}", e))?;
        }
    }

    Ok(dev_data.to_string_lossy().to_string())
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
fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write to file: {}", e))
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
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
    // Currently only supports Windows
    #[cfg(target_os = "windows")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let noita_path = home_dir
            .join("AppData")
            .join("LocalLow")
            .join("Nolla_Games_Noita")
            .join("save00");
        if noita_path.exists() {
            Ok(noita_path.to_string_lossy().to_string())
        } else {
            Err("Noita save directory not found".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Noita save path detection is only supported on Windows".to_string())
    }
}

#[tauri::command]
fn get_entangled_worlds_config_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {

        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let ew_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("quant")
            .join("entangledworlds");
        if ew_path.exists() {
            Ok(ew_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds config directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let config_path = home_dir.join(".config").join("entangledworlds");

        if config_path.exists() {
            Ok(config_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds config directory not found".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Entangled Worlds path detection is not supported on this platform".to_string())
    }
}

#[tauri::command]
fn get_entangled_worlds_save_path() -> Result<String, String> {

    #[cfg(target_os = "windows")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let ew_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("quant")
            .join("entangledworlds")
            .join("data");
        if ew_path.exists() {
            Ok(ew_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let save_path = home_dir
            .join(".local")
            .join("share")
            .join("entangledworlds");
        if save_path.exists() {
            Ok(save_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Entangled Worlds save path detection is not supported on this platform".to_string())
    }
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
    let normalized_level = level.to_uppercase();
    let timestamp = Utc::now().to_rfc3339();
    let entry = LogEntry {
        timestamp,
        level: normalized_level.clone(),
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

    let version = get_version();
    let instance_id = &*INSTANCE_ID;
    let log_file = logs_dir.join(format!("hallinta_v{}_{}.log", version, instance_id));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file {}: {}", log_file.display(), e))?;
    for entry in logs {
        let log_line = format!(
            "[{}] [{}] [{}] {}\n",
            entry.timestamp, entry.level, entry.module, entry.message
        );
        file.write_all(log_line.as_bytes()).map_err(|e| {
            format!("Failed to write to log file {}: {}", log_file.display(), e)
        })?;
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
        let noita_dir = match get_noita_save_path() {
            Ok(path) => path,
            Err(_) => String::new(),
        };
        let entangled_dir = match get_entangled_worlds_config_path() {
            Ok(path) => path,
            Err(_) => String::new(),
        };
        let default_log_level = if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "INFO"
        };
        let default_settings = AppSettings {
            noita_dir,
            entangled_dir,
            dark_mode: false,
            selected_preset: "Default".to_string(),
            version: get_version(),
            log_settings: LogSettings {
                max_log_files: 50,
                max_log_size_mb: 10,
                log_level: default_log_level.to_string(),
                auto_save: true,
            },
            backup_settings: BackupSettings::default(),
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

// --- Session Lock ---

#[tauri::command]
fn create_session_lock(dev_mode_active: bool, original_mod_config_path: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let lock_path = data_dir.join(".hallinta_session");
    let lock = SessionLock {
        created_at: Utc::now().to_rfc3339(),
        dev_mode_active,
        original_mod_config_path,
        pid: std::process::id(),
    };
    let json = serde_json::to_string_pretty(&lock)
        .map_err(|e| format!("Failed to serialize session lock: {}", e))?;
    fs::write(&lock_path, json)
        .map_err(|e| format!("Failed to write session lock: {}", e))?;
    Ok(())
}

#[tauri::command]
fn remove_session_lock() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let lock_path = data_dir.join(".hallinta_session");
    if lock_path.exists() {
        fs::remove_file(&lock_path)
            .map_err(|e| format!("Failed to remove session lock: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
fn check_session_lock() -> Result<Option<SessionLock>, String> {
    let data_dir = get_data_dir()?;
    let lock_path = data_dir.join(".hallinta_session");
    if !lock_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&lock_path)
        .map_err(|e| format!("Failed to read session lock: {}", e))?;
    let lock: SessionLock = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse session lock: {}", e))?;
    // Check if the PID is still running — if so, it's not stale
    if is_process_running(lock.pid) {
        return Ok(None);
    }
    Ok(Some(lock))
}

fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                stdout.contains(&pid.to_string())
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Path::new(&format!("/proc/{}", pid)).exists()
    }
}

// --- Dev Mode Overwrite ---

#[tauri::command]
fn cache_and_overwrite_mod_config(real_noita_dir: String, dev_data_dir: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let cache_path = data_dir.join(".hallinta_original_mod_config.xml");
    let real_config = PathBuf::from(&real_noita_dir).join("mod_config.xml");
    let dev_config = PathBuf::from(&dev_data_dir).join("mod_config.xml");

    if real_config.exists() {
        fs::copy(&real_config, &cache_path)
            .map_err(|e| format!("Failed to cache original mod_config.xml: {}", e))?;
    }

    if dev_config.exists() {
        fs::copy(&dev_config, &real_config)
            .map_err(|e| format!("Failed to overwrite real mod_config.xml with dev version: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn revert_mod_config(real_noita_dir: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let cache_path = data_dir.join(".hallinta_original_mod_config.xml");
    let real_config = PathBuf::from(&real_noita_dir).join("mod_config.xml");

    if cache_path.exists() {
        fs::copy(&cache_path, &real_config)
            .map_err(|e| format!("Failed to revert mod_config.xml: {}", e))?;
        fs::remove_file(&cache_path)
            .map_err(|e| format!("Failed to remove cached mod_config.xml: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
fn check_mod_config_cache_exists() -> Result<bool, String> {
    let data_dir = get_data_dir()?;
    let cache_path = data_dir.join(".hallinta_original_mod_config.xml");
    Ok(cache_path.exists())
}

fn revert_mod_config_internal() {
    if let Ok(data_dir) = get_data_dir() {
        let lock_path = data_dir.join(".hallinta_session");
        if lock_path.exists() {
            if let Ok(content) = fs::read_to_string(&lock_path) {
                if let Ok(lock) = serde_json::from_str::<SessionLock>(&content) {
                    if lock.dev_mode_active && !lock.original_mod_config_path.is_empty() {
                        let cache_path = data_dir.join(".hallinta_original_mod_config.xml");
                        let real_config = PathBuf::from(&lock.original_mod_config_path);
                        if cache_path.exists() && real_config.parent().map_or(false, |p| p.exists()) {
                            let _ = fs::copy(&cache_path, &real_config);
                            let _ = fs::remove_file(&cache_path);
                        }
                    }
                }
            }
            let _ = fs::remove_file(&lock_path);
        }
    }
}

// --- Backup System ---

fn add_directory_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    source_dir: &Path,
    prefix: &str,
) -> Result<(), String> {
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: could not read entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        let relative = path.strip_prefix(source_dir)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?;

        let archive_name = if prefix.is_empty() {
            relative.to_string_lossy().to_string()
        } else {
            format!("{}/{}", prefix, relative.to_string_lossy())
        };

        // Normalize path separators for zip
        let archive_name = archive_name.replace('\\', "/");

        if path.is_dir() {
            if !archive_name.is_empty() && archive_name != "/" {
                let dir_name = if archive_name.ends_with('/') {
                    archive_name.clone()
                } else {
                    format!("{}/", archive_name)
                };
                zip.add_directory(&dir_name, options)
                    .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
            }
        } else {
            zip.start_file(&archive_name, options)
                .map_err(|e| format!("Failed to start file in zip: {}", e))?;
            match fs::read(path) {
                Ok(data) => {
                    zip.write_all(&data)
                        .map_err(|e| format!("Failed to write file to zip: {}", e))?;
                }
                Err(e) => {
                    eprintln!("Warning: could not read file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn create_backup(noita_dir: String, include_save01: bool, include_presets: bool) -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        fs::create_dir_all(&backups_dir)
            .map_err(|e| format!("Failed to create backups directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("hallinta_backup_{}.zip", timestamp);
    let zip_path = backups_dir.join(&filename);

    let noita_dir_clone = noita_dir.clone();
    let data_dir_clone = data_dir.clone();
    let zip_path_clone = zip_path.clone();
    let filename_clone = filename.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_path_clone)
            .map_err(|e| format!("Failed to create backup zip: {}", e))?;
        let mut zip = ZipWriter::new(file);

        // Always include save00
        let save00_path = PathBuf::from(&noita_dir_clone);
        if save00_path.exists() {
            add_directory_to_zip(&mut zip, &save00_path, "save00")?;
        }

        // Optionally include save01 (sibling of save00)
        if include_save01 {
            if let Some(parent) = save00_path.parent() {
                let save01_path = parent.join("save01");
                if save01_path.exists() {
                    add_directory_to_zip(&mut zip, &save01_path, "save01")?;
                }
            }
        }

        // Optionally include presets
        if include_presets {
            let presets_path = data_dir_clone.join("presets.json");
            if presets_path.exists() {
                let options: FileOptions<()> =
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
                zip.start_file("presets.json", options)
                    .map_err(|e| format!("Failed to add presets to zip: {}", e))?;
                let data = fs::read(&presets_path)
                    .map_err(|e| format!("Failed to read presets: {}", e))?;
                zip.write_all(&data)
                    .map_err(|e| format!("Failed to write presets to zip: {}", e))?;
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finish backup zip: {}", e))?;

        Ok::<String, String>(filename_clone)
    })
    .await
    .map_err(|e| format!("Backup task failed: {}", e))?
}

#[tauri::command]
fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    let entries = fs::read_dir(&backups_dir)
        .map_err(|e| format!("Failed to read backups directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            let filename = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to read backup metadata: {}", e))?;
            let size_bytes = metadata.len();

            let modified = metadata.modified()
                .map(|t| {
                    let datetime: chrono::DateTime<Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            // Peek inside the zip to see what it contains
            let (contains_save00, contains_save01, contains_presets) = match std::fs::File::open(&path) {
                Ok(file) => {
                    match zip::ZipArchive::new(file) {
                        Ok(mut archive) => {
                            let mut has_save00 = false;
                            let mut has_save01 = false;
                            let mut has_presets = false;
                            for i in 0..archive.len() {
                                if let Ok(entry) = archive.by_index(i) {
                                    let name = entry.name();
                                    if name.starts_with("save00/") { has_save00 = true; }
                                    if name.starts_with("save01/") { has_save01 = true; }
                                    if name == "presets.json" { has_presets = true; }
                                }
                            }
                            (has_save00, has_save01, has_presets)
                        }
                        Err(_) => (false, false, false),
                    }
                }
                Err(_) => (false, false, false),
            };

            backups.push(BackupInfo {
                filename,
                timestamp: modified,
                size_bytes,
                contains_save00,
                contains_save01,
                contains_presets,
            });
        }
    }

    // Sort by timestamp descending (newest first)
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

#[tauri::command]
fn delete_backup(filename: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }
    // Ensure path is within backups directory
    let backups_dir = data_dir.join("backups");
    if !backup_path.starts_with(&backups_dir) {
        return Err("Invalid backup path".to_string());
    }
    fs::remove_file(&backup_path)
        .map_err(|e| format!("Failed to delete backup: {}", e))?;
    Ok(())
}

#[tauri::command]
fn cleanup_old_backups(max_age_days: u32) -> Result<u32, String> {
    if max_age_days == 0 {
        return Ok(0);
    }

    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days as u64 * 86400))
        .ok_or_else(|| "Failed to calculate cutoff time".to_string())?;

    let mut deleted = 0u32;
    let entries = fs::read_dir(&backups_dir)
        .map_err(|e| format!("Failed to read backups directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        if fs::remove_file(&path).is_ok() {
                            deleted += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(deleted)
}

#[tauri::command]
fn get_backup_contents(filename: String) -> Result<BackupInfo, String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let file = std::fs::File::open(&backup_path)
        .map_err(|e| format!("Failed to open backup: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read backup zip: {}", e))?;

    let metadata = fs::metadata(&backup_path)
        .map_err(|e| format!("Failed to read backup metadata: {}", e))?;

    let modified = metadata.modified()
        .map(|t| {
            let datetime: chrono::DateTime<Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_default();

    let mut has_save00 = false;
    let mut has_save01 = false;
    let mut has_presets = false;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name();
            if name.starts_with("save00/") { has_save00 = true; }
            if name.starts_with("save01/") { has_save01 = true; }
            if name == "presets.json" { has_presets = true; }
        }
    }

    Ok(BackupInfo {
        filename,
        timestamp: modified,
        size_bytes: metadata.len(),
        contains_save00: has_save00,
        contains_save01: has_save01,
        contains_presets: has_presets,
    })
}

#[tauri::command]
async fn restore_backup(filename: String, noita_dir: String, options: RestoreOptions) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let noita_dir_clone = noita_dir.clone();
    let data_dir_clone = data_dir.clone();
    let backup_path_clone = backup_path.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&backup_path_clone)
            .map_err(|e| format!("Failed to open backup: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read backup zip: {}", e))?;

        let save00_target = PathBuf::from(&noita_dir_clone);
        let save01_target = save00_target.parent()
            .map(|p| p.join("save01"))
            .ok_or_else(|| "Cannot determine save01 path".to_string())?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let entry_name = entry.name().to_string();

            // Determine the target path based on the entry prefix
            let target_path = if entry_name.starts_with("save00/") && options.restore_save00 {
                let relative = entry_name.strip_prefix("save00/").unwrap_or(&entry_name);
                if relative.is_empty() { continue; }
                Some(save00_target.join(relative))
            } else if entry_name.starts_with("save01/") && options.restore_save01 {
                let relative = entry_name.strip_prefix("save01/").unwrap_or(&entry_name);
                if relative.is_empty() { continue; }
                Some(save01_target.join(relative))
            } else if entry_name == "presets.json" && options.restore_presets {
                Some(data_dir_clone.join("presets.json"))
            } else {
                None
            };

            if let Some(target) = target_path {
                if entry.is_dir() {
                    let _ = fs::create_dir_all(&target);
                } else {
                    if let Some(parent) = target.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf)
                        .map_err(|e| format!("Failed to read zip entry data: {}", e))?;
                    fs::write(&target, &buf)
                        .map_err(|e| format!("Failed to write restored file: {}", e))?;
                }
            }
        }

        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Restore task failed: {}", e))?
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
            get_entangled_worlds_config_path,
            get_entangled_worlds_save_path,
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
            write_file,
            read_file,
            check_file_exists,
            get_dev_save_dir,
            create_session_lock,
            remove_session_lock,
            check_session_lock,
            cache_and_overwrite_mod_config,
            revert_mod_config,
            check_mod_config_cache_exists,
            create_backup,
            list_backups,
            delete_backup,
            cleanup_old_backups,
            get_backup_contents,
            restore_backup
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Safety net: revert mod_config if dev mode was active
                revert_mod_config_internal();
                // Flush any remaining logs
                if let Ok(data_dir) = get_data_dir() {
                    let logs_dir = data_dir.join("logs");
                    let _ = fs::create_dir_all(&logs_dir);
                    let logs = {
                        if let Ok(mut file_buffer) = LOG_FILE_BUFFER.lock() {
                            file_buffer.drain(..).collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        }
                    };
                    if !logs.is_empty() {
                        let version = get_version();
                        let instance_id = &*INSTANCE_ID;
                        let log_file = logs_dir.join(format!("hallinta_v{}_{}.log", version, instance_id));
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
                            for entry in logs {
                                let log_line = format!(
                                    "[{}] [{}] [{}] {}\n",
                                    entry.timestamp, entry.level, entry.module, entry.message
                                );
                                let _ = file.write_all(log_line.as_bytes());
                            }
                        }
                    }
                }
            }
        });
}
