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
        // Always auto-detect on first run, regardless of build mode.
        // In dev builds the actual save dir used is dev_data/save00 (set in app.rs),
        // but populating the real paths here lets the settings UI show useful values
        // and enables workshop mod detection.
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

    let mut dirty = false;

    // Auto-detect any missing paths on load (all build modes).
    if settings.noita_dir.trim().is_empty() {
        if let Ok(p) = platform::get_noita_save_path() {
            settings.noita_dir = p.to_string_lossy().to_string();
            dirty = true;
        }
    }
    if settings.entangled_dir.trim().is_empty() {
        if let Ok(p) = platform::get_entangled_worlds_save_path() {
            settings.entangled_dir = p.to_string_lossy().to_string();
            dirty = true;
        }
    }
    if settings.gallery_settings.steam_path.trim().is_empty() {
        if let Ok(p) = crate::core::workshop::detect_steam_path() {
            settings.gallery_settings.steam_path = p.to_string_lossy().to_string();
            dirty = true;
        }
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

    #[test]
    fn test_get_data_dir_returns_path() {
        let result = get_data_dir();
        assert!(result.is_ok(), "get_data_dir should not fail: {:?}", result);
        let path = result.unwrap();
        assert!(path.exists(), "data dir should be created if missing");
        // Dev builds use dev_data/ inside the crate root
        if cfg!(debug_assertions) {
            assert!(
                path.ends_with("dev_data"),
                "dev build data dir should be dev_data, got: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_save_and_load_settings_roundtrip() {
        use crate::models::{
            BackupSettings, GallerySettings, LogSettings, SaveMonitorSettings,
        };

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
            gallery_settings: GallerySettings {
                catalog_url: "https://example.com/catalog.json".to_string(),
                steam_path: "/test/steam".to_string(),
            },
            compact_mode: true,
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
        assert_eq!(
            loaded.gallery_settings.catalog_url,
            original.gallery_settings.catalog_url
        );

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
                "auto_delete_days": 30,
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
            }
        }"#;

        let settings: AppSettings = serde_json::from_str(minimal_json)
            .expect("minimal settings JSON should deserialize without compact_mode field");
        assert!(!settings.compact_mode, "missing compact_mode should default to false");
    }
}
