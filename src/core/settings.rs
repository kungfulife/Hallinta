use crate::core::{logging, platform};
use crate::models::{AppSettings, LogSettings, SaveMonitorSettings};
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
thread_local! {
    static SAVE_SETTINGS_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_save_settings_call_count() {
    SAVE_SETTINGS_CALLS.with(|calls| calls.set(0));
}

#[cfg(test)]
pub(crate) fn save_settings_call_count() -> usize {
    SAVE_SETTINGS_CALLS.with(std::cell::Cell::get)
}

#[cfg(test)]
static TEST_DATA_DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    std::env::temp_dir().join(format!("hallinta-tests-{}", std::process::id()))
});

pub fn get_data_dir() -> Result<PathBuf, String> {
    #[cfg(test)]
    let data_dir = TEST_DATA_DIR.clone();

    #[cfg(not(test))]
    let data_dir = {
        let local_data_dir = dirs::data_local_dir();
        choose_app_data_dir(local_data_dir.as_deref())?
    };

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
        let save_monitor_settings = SaveMonitorSettings {
            include_entangled: platform::entangled_dir_usable(&entangled_dir),
            ..Default::default()
        };

        let default_settings = AppSettings {
            noita_dir,
            entangled_dir,
            dark_mode: false,
            selected_preset: "Default".to_string(),
            version: platform::get_version(),
            log_settings: LogSettings::default(),
            save_monitor_settings,
            compact_mode: false,
            ui_scale: crate::ui::design::SCALE_INTERNAL_DEFAULT,
            last_filter_mode: String::new(),
            last_sort_mode: String::new(),
            needs_noita_reconciliation: false,
            dismissed_update_version: None,
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
    if !platform::is_configured_path(&settings.noita_dir)
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
    if dirty {
        save_settings(&settings)?;
    }

    Ok(settings)
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    #[cfg(test)]
    SAVE_SETTINGS_CALLS.with(|calls| calls.set(calls.get() + 1));

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
    fn unit_tests_use_isolated_app_data() {
        let test_data_dir = get_data_dir().expect("test data directory should resolve");
        let production_data_dir = choose_app_data_dir(dirs::data_local_dir().as_deref())
            .expect("production data directory should resolve on the test machine");

        assert!(test_data_dir.starts_with(std::env::temp_dir()));
        assert_ne!(test_data_dir, production_data_dir);
    }

    #[test]
    fn test_save_and_load_settings_roundtrip() {
        use crate::models::{LogSettings, SaveMonitorSettings};

        let original = AppSettings {
            noita_dir: "/test/noita".to_string(),
            entangled_dir: "/test/ew".to_string(),
            dark_mode: true,
            selected_preset: "MyPreset".to_string(),
            version: "1.2.3".to_string(),
            log_settings: LogSettings::default(),
            save_monitor_settings: SaveMonitorSettings::default(),
            compact_mode: true,
            ui_scale: 1.0,
            last_filter_mode: "all".to_string(),
            last_sort_mode: "default".to_string(),
            needs_noita_reconciliation: true,
            dismissed_update_version: Some("0.9.0".to_string()),
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let loaded: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.noita_dir, original.noita_dir);
        assert_eq!(loaded.entangled_dir, original.entangled_dir);
        assert_eq!(loaded.dark_mode, original.dark_mode);
        assert_eq!(loaded.selected_preset, original.selected_preset);
        assert_eq!(loaded.compact_mode, original.compact_mode);
        assert_eq!(loaded.version, original.version);
        assert_eq!(loaded.ui_scale, original.ui_scale);
        assert_eq!(loaded.last_filter_mode, original.last_filter_mode);
        assert_eq!(loaded.last_sort_mode, original.last_sort_mode);
        assert_eq!(
            loaded.needs_noita_reconciliation,
            original.needs_noita_reconciliation
        );
        assert_eq!(
            loaded.dismissed_update_version,
            original.dismissed_update_version
        );
        assert_eq!(
            loaded.log_settings.log_level,
            original.log_settings.log_level
        );
        assert_eq!(
            loaded.save_monitor_settings.backup_delay_minutes,
            original.save_monitor_settings.backup_delay_minutes
        );
    }

    #[test]
    fn settings_serialization_omits_steam_path() {
        let legacy_json = r#"{
            "noita_dir": "",
            "entangled_dir": "",
            "dark_mode": false,
            "selected_preset": "Default",
            "version": "0.8.5",
            "steam_path": "D:/PortableSteam"
        }"#;
        let settings: AppSettings =
            serde_json::from_str(legacy_json).expect("legacy Steam path should be ignored");

        let serialized = serde_json::to_value(settings).expect("settings should serialize");

        assert!(serialized.get("steam_path").is_none());
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
                "log_level": "INFO"
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
        assert_eq!(settings.save_monitor_settings.backup_delay_minutes, 3);
        assert!(!settings.needs_noita_reconciliation);
    }

    #[test]
    fn old_nested_settings_missing_fields_use_defaults() {
        let old_json = r#"{
            "noita_dir": "C:/Noita/save00",
            "entangled_dir": "",
            "dark_mode": true,
            "selected_preset": "Custom",
            "version": "0.7.0",
            "log_settings": {
                "log_level": "WARN"
            },
            "save_monitor_settings": {
                "max_snapshots_per_preset": 7
            },
            "steam_path": ""
        }"#;

        let settings: AppSettings =
            serde_json::from_str(old_json).expect("old partial settings should not reset app");

        assert_eq!(settings.log_settings.log_level, "WARN");
        assert_eq!(settings.log_settings.max_log_files, 50);
        assert_eq!(settings.log_settings.max_log_size_mb, 10);
        assert_eq!(settings.save_monitor_settings.max_snapshots_per_session, 7);
        assert_eq!(settings.save_monitor_settings.backup_delay_minutes, 3);
        assert!(!settings.save_monitor_settings.include_entangled);
        assert!(settings.save_monitor_settings.include_save01);
        assert!(settings.dismissed_update_version.is_none());
    }
}
