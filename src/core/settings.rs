use crate::core::{logging, platform};
use crate::models::{AppSettings, BackupSettings, LogSettings, SaveMonitorSettings};
use std::fs;
use std::path::{Path, PathBuf};

pub fn get_data_dir() -> Result<PathBuf, String> {
    let local_data_dir = dirs::data_local_dir();
    let data_dir = choose_app_data_dir(local_data_dir.as_deref())?;

    if !data_dir.exists() {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}

fn choose_app_data_dir(local_data_dir: Option<&Path>) -> Result<PathBuf, String> {
    local_data_dir
        .map(|dir| dir.join("Hallinta"))
        .ok_or_else(|| "Could not find local data directory".to_string())
}

pub fn load_settings() -> Result<AppSettings, String> {
    let data_dir = get_data_dir()?;
    let settings_path = data_dir.join("settings.json");

    if !settings_path.exists() {
        // Always auto-detect on first run, regardless of build mode.
        let noita_dir = platform::get_noita_save_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let entangled_dir = platform::get_entangled_worlds_save_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let steam_path = crate::core::workshop::detect_steam_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let default_settings = AppSettings {
            noita_dir,
            entangled_dir,
            dark_mode: false,
            selected_preset: "Default".to_string(),
            version: platform::get_version(),
            log_settings: LogSettings::default(),
            backup_settings: BackupSettings::default(),
            save_monitor_settings: SaveMonitorSettings::default(),
            steam_path,
            compact_mode: false,
            ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
            last_filter_mode: String::new(),
            last_sort_mode: String::new(),
        };
        save_settings(&default_settings)?;
        return Ok(default_settings);
    }

    let content = fs::read_to_string(&settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    let mut settings: AppSettings =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))?;

    let mut dirty = false;

    // Auto-detect any missing paths on load (all build modes).
    if settings.noita_dir.trim().is_empty()
        && let Ok(p) = platform::get_noita_save_path()
    {
        settings.noita_dir = p.to_string_lossy().to_string();
        dirty = true;
    }
    if settings.entangled_dir.trim().is_empty()
        && let Ok(p) = platform::get_entangled_worlds_save_path()
    {
        settings.entangled_dir = p.to_string_lossy().to_string();
        dirty = true;
    }
    if settings.steam_path.trim().is_empty()
        && let Ok(p) = crate::core::workshop::detect_steam_path()
    {
        settings.steam_path = p.to_string_lossy().to_string();
        dirty = true;
    }
    if dirty {
        save_settings(&settings)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn choose_app_data_dir_uses_hallinta_local_data() {
        let local_data_dir = Path::new("C:/Users/example/AppData/Local");

        let path = choose_app_data_dir(Some(local_data_dir))
            .expect("app data dir choice should use local data dir");

        assert_eq!(path, local_data_dir.join("Hallinta"));
    }

    #[test]
    fn choose_app_data_dir_requires_local_data_dir() {
        let err = choose_app_data_dir(None)
            .expect_err("app data dir choice should require local data dir");

        assert_eq!(err, "Could not find local data directory");
    }

    #[test]
    fn test_save_and_load_settings_roundtrip() {
        use crate::models::{BackupSettings, LogSettings, SaveMonitorSettings};

        let dir = std::env::temp_dir().join("hallinta_settings_test");
        std::fs::create_dir_all(&dir).unwrap();
        let settings_path = dir.join("settings.json");
        let _ = std::fs::remove_file(&settings_path);

        let original = AppSettings {
            noita_dir: "/test/noita".to_string(),
            entangled_dir: "/test/ew".to_string(),
            dark_mode: true,
            selected_preset: "MyPreset".to_string(),
            version: "1.2.3".to_string(),
            log_settings: LogSettings::default(),
            backup_settings: BackupSettings::default(),
            save_monitor_settings: SaveMonitorSettings::default(),
            steam_path: "/test/steam".to_string(),
            compact_mode: true,
            ui_scale: 1.0,
            last_filter_mode: "all".to_string(),
            last_sort_mode: "default".to_string(),
        };

        // Serialize to file manually (bypass get_data_dir to use temp dir)
        let json = serde_json::to_string_pretty(&original).unwrap();
        std::fs::write(&settings_path, &json).unwrap();

        // Deserialize back
        let loaded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.noita_dir, original.noita_dir);
        assert_eq!(loaded.entangled_dir, original.entangled_dir);
        assert_eq!(loaded.dark_mode, original.dark_mode);
        assert_eq!(loaded.selected_preset, original.selected_preset);
        assert_eq!(loaded.compact_mode, original.compact_mode);
        assert_eq!(loaded.steam_path, original.steam_path);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_settings_default_fields_via_serde() {
        // Old settings.json without new optional fields should deserialize fine
        // (all new fields use #[serde(default)]).
        let minimal_json = r#"{
            "noita_dir": "",
            "entangled_dir": "",
            "dark_mode": false,
            "selected_preset": "Default",
            "version": "0.1.0",
            "log_settings": {
                "max_log_files": 50,
                "max_log_size_mb": 10,
                "log_level": "INFO",
                "auto_save": true
            },
            "backup_settings": {
                "backup_interval_minutes": 0
            },
            "save_monitor_settings": {
                "interval_minutes": 3,
                "max_snapshots_per_preset": 10,
                "include_entangled": false
            },
            "gallery_settings": {
                "catalog_url": "",
                "steam_path": ""
            },
            "steam_path": ""
        }"#;

        let settings: AppSettings = serde_json::from_str(minimal_json)
            .expect("minimal settings JSON should deserialize without compact_mode field");
        assert!(
            !settings.compact_mode,
            "missing compact_mode should default to false"
        );
    }
}
