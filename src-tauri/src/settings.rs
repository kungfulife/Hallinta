use crate::app::{get_entangled_worlds_config_path, get_noita_save_path, get_version};
use crate::backup::add_directory_to_zip;
use crate::logging::add_log_entry;
use crate::models::{AppSettings, BackupSettings, LogSettings, ModPreset, SaveMonitorSettings};
use chrono::Utc;
use serde_json;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs as tokio_fs;
use zip::write::FileOptions;
use zip::ZipWriter;
pub(crate) fn get_data_dir() -> Result<PathBuf, String> {
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

pub(crate) async fn create_upgrade_backup(
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

    // Capture save paths from settings for full security backup
    let noita_dir = settings.noita_dir.clone();
    let entangled_dir = settings.entangled_dir.clone();

    let zip_file_path_clone = zip_file_path.clone();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_file_path_clone)
            .map_err(|e| format!("Failed to create zip file: {}", e))?;
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        // App critical data
        zip.start_file("settings.json", options)
            .map_err(|e| format!("Failed to start file in zip: {}", e))?;
        zip.write_all(settings_json.as_bytes())
            .map_err(|e| format!("Failed to write settings to zip: {}", e))?;

        zip.start_file("presets.json", options)
            .map_err(|e| format!("Failed to start file in zip: {}", e))?;
        zip.write_all(presets_json.as_bytes())
            .map_err(|e| format!("Failed to write presets to zip: {}", e))?;

        // Save data — save00
        if !noita_dir.is_empty() {
            let save00_path = PathBuf::from(&noita_dir);
            if save00_path.exists() {
                add_directory_to_zip(&mut zip, &save00_path, "save00")?;
            }

            // save01 (sibling of save00)
            if let Some(parent) = save00_path.parent() {
                let save01_path = parent.join("save01");
                if save01_path.exists() {
                    add_directory_to_zip(&mut zip, &save01_path, "save01")?;
                }
            }
        }

        // Entangled Worlds data if configured
        if !entangled_dir.is_empty() {
            let ew_path = PathBuf::from(&entangled_dir);
            if ew_path.exists() {
                add_directory_to_zip(&mut zip, &ew_path, "entangled_worlds")?;
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finish zip: {}", e))?;
        Ok::<(), String>(())
    })
        .await
        .map_err(|e| format!("Failed to create upgrade backup: {}", e))??;

    // Auto-cleanup: keep only the last 5 upgrade backups
    cleanup_old_upgrade_backups(&upgrade_backup_dir, 5)?;

    Ok(())
}

pub(crate) fn cleanup_old_upgrade_backups(upgrade_backup_dir: &Path, keep_count: usize) -> Result<(), String> {
    if !upgrade_backup_dir.exists() {
        return Ok(());
    }

    let mut backups: Vec<_> = fs::read_dir(upgrade_backup_dir)
        .map_err(|e| format!("Failed to read upgrade_backups directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "zip")
        })
        .collect();

    if backups.len() <= keep_count {
        return Ok(());
    }

    // Sort by modified time, newest first
    backups.sort_by(|a, b| {
        let time_a = a.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        let time_b = b.metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH);
        time_b.cmp(&time_a)
    });

    // Remove all but the newest `keep_count`
    for old_backup in backups.into_iter().skip(keep_count) {
        let _ = fs::remove_file(old_backup.path());
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn save_settings(settings: AppSettings) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(settings_path, json_content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn load_settings() -> Result<AppSettings, String> {
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
            save_monitor_settings: SaveMonitorSettings::default(),
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
pub(crate) fn save_presets(presets: std::collections::HashMap<String, Vec<ModPreset>>) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let presets_path = data_dir.join("presets.json");

    let json_content = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;
    fs::write(presets_path, json_content)
        .map_err(|e| format!("Failed to write presets file: {}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) fn load_presets() -> Result<std::collections::HashMap<String, Vec<ModPreset>>, String> {
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

