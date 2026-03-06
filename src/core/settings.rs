use crate::core::{logging, platform};
use crate::models::{
    AppSettings, BackupSettings, GallerySettings, LogSettings, SaveMonitorSettings,
};
use std::fs;
use std::path::PathBuf;

pub fn get_data_dir() -> Result<PathBuf, String> {
    let data_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dev_data")
    } else {
        dirs::data_local_dir()
            .ok_or_else(|| "Could not find local data directory".to_string())?
            .join("Hallinta")
    };

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}

pub fn load_settings() -> Result<AppSettings, String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    if !settings_path.exists() {
        let (noita_dir, entangled_dir, steam_path) = if cfg!(debug_assertions) {
            (String::new(), String::new(), String::new())
        } else {
            let noita_dir = platform::get_noita_save_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            // BUG-5 FIX: Use save path (not config path) for entangled worlds
            let entangled_dir = platform::get_entangled_worlds_save_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let steam_path = crate::core::workshop::detect_steam_path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            (noita_dir, entangled_dir, steam_path)
        };

        let default_settings = AppSettings {
            noita_dir,
            entangled_dir,
            dark_mode: false,
            selected_preset: "Default".to_string(),
            version: platform::get_version(),
            log_settings: LogSettings::default(),
            backup_settings: BackupSettings::default(),
            save_monitor_settings: SaveMonitorSettings::default(),
            gallery_settings: GallerySettings {
                catalog_url: String::new(),
                steam_path,
            },
            compact_mode: false,
        };
        save_settings(&default_settings)?;
        return Ok(default_settings);
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    let mut settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;

    // Auto-detect steam path if missing
    if !cfg!(debug_assertions) && settings.gallery_settings.steam_path.trim().is_empty() {
        if let Ok(steam_path) = crate::core::workshop::detect_steam_path() {
            settings.gallery_settings.steam_path = steam_path.to_string_lossy().to_string();
            save_settings(&settings)?;
        }
    }

    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    let json_content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(settings_path, json_content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;
    Ok(())
}

/// Check if version changed and create upgrade backup if needed.
/// Returns true if an upgrade was performed.
pub fn check_and_upgrade_version(settings: &mut AppSettings) -> Result<bool, String> {
    let current = platform::get_version();
    if settings.version == current {
        return Ok(false);
    }

    let old_version = settings.version.clone();
    let _ = logging::log(
        "INFO",
        &format!(
            "Version update detected ({} -> {}), creating upgrade backup",
            old_version, current
        ),
        "Settings",
    );

    settings.version = current;
    save_settings(settings)?;
    Ok(true)
}
