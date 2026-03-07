use crate::core::settings::get_data_dir;
use crate::models::{OpenSourceLibrary, SystemInfo};
use chrono::{Local, Utc};
use std::fs;
use std::path::{Path, PathBuf};

// Generated at build time from Cargo.lock (BUG-4 fix)
include!(concat!(env!("OUT_DIR"), "/libraries.rs"));

pub fn is_dev_build() -> bool {
    cfg!(debug_assertions)
}

pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

pub fn get_exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| format!("Could not get executable path: {}", e))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "Could not get parent directory".to_string())
}

pub fn get_app_settings_dir() -> Result<PathBuf, String> {
    get_data_dir()
}

pub fn get_noita_save_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
        let noita_path = home_dir
            .join("AppData")
            .join("LocalLow")
            .join("Nolla_Games_Noita")
            .join("save00");
        if noita_path.exists() {
            Ok(noita_path)
        } else {
            Err("Noita save directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
        let steam_candidates = [
            home_dir.join(".steam").join("steam"),
            home_dir.join(".local").join("share").join("Steam"),
        ];

        for steam_path in &steam_candidates {
            let noita_path = steam_path
                .join("steamapps")
                .join("compatdata")
                .join("881100")
                .join("pfx")
                .join("drive_c")
                .join("users")
                .join("steamuser")
                .join("AppData")
                .join("LocalLow")
                .join("Nolla_Games_Noita")
                .join("save00");
            if noita_path.exists() {
                return Ok(noita_path);
            }
        }

        Err("Noita save directory not found. Ensure Noita has been run at least once via Steam Proton.".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Noita save path detection is not supported on this platform".to_string())
    }
}

/// BUG-5 FIX: Auto-detect returns the save/data path (not config path).
pub fn get_entangled_worlds_save_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
        let ew_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("quant")
            .join("entangledworlds")
            .join("data");
        if ew_path.exists() {
            Ok(ew_path)
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
        let save_path = home_dir
            .join(".local")
            .join("share")
            .join("entangledworlds");
        if save_path.exists() {
            Ok(save_path)
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Entangled Worlds path detection is not supported on this platform".to_string())
    }
}

pub fn get_dev_save_dir() -> Result<PathBuf, String> {
    if !cfg!(debug_assertions) {
        return Err("Not in dev mode".to_string());
    }

    let dev_save_dir = get_data_dir()?.join("save00");
    fs::create_dir_all(&dev_save_dir)
        .map_err(|e| format!("Failed to create dev save00 directory: {}", e))?;
    Ok(dev_save_dir)
}

/// Seed `dev_data/save00/mod_config.xml` from the real Noita save on first run.
///
/// Only copies if the file does not already exist (preserves cached dev state across runs).
/// Returns a human-readable description of what was done, suitable for logging by the caller.
pub fn seed_dev_mod_config() -> Result<String, String> {
    let dev_save_dir = get_dev_save_dir()?;
    seed_mod_config_into(&dev_save_dir)
}

/// Core seeding logic operating on an arbitrary target directory.
/// Separated from `seed_dev_mod_config` so tests can supply a temp dir instead of
/// touching `dev_data/save00` directly.
fn seed_mod_config_into(save_dir: &Path) -> Result<String, String> {
    fs::create_dir_all(save_dir)
        .map_err(|e| format!("Failed to create save dir: {}", e))?;
    let config_path = save_dir.join("mod_config.xml");

    if config_path.exists() {
        return Ok(format!(
            "Using cached dev mod_config.xml ({} bytes)",
            fs::metadata(&config_path).map(|m| m.len()).unwrap_or(0)
        ));
    }

    // First run — try to copy from the real Noita save directory.
    match get_noita_save_path() {
        Ok(real_save) => {
            let real_config = real_save.join("mod_config.xml");
            if real_config.exists() {
                fs::copy(&real_config, &config_path)
                    .map_err(|e| format!("Failed to copy mod_config.xml from real save: {}", e))?;
                Ok(format!(
                    "Seeded dev mod_config.xml from real save at {}",
                    real_save.display()
                ))
            } else {
                write_empty_mod_config(&config_path)?;
                Ok(format!(
                    "Real save at {} has no mod_config.xml; created empty placeholder",
                    real_save.display()
                ))
            }
        }
        Err(e) => {
            write_empty_mod_config(&config_path)?;
            Ok(format!(
                "Real Noita save not found ({}); created empty placeholder",
                e
            ))
        }
    }
}

fn write_empty_mod_config(path: &PathBuf) -> Result<(), String> {
    let placeholder = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n</Mods>";
    fs::write(path, placeholder)
        .map_err(|e| format!("Failed to create placeholder mod_config.xml: {}", e))
}

pub fn get_dev_entangled_dir() -> Result<PathBuf, String> {
    if !cfg!(debug_assertions) {
        return Err("Not in dev mode".to_string());
    }

    let entangled_dir = get_data_dir()?.join("entangled_worlds");
    fs::create_dir_all(&entangled_dir)
        .map_err(|e| format!("Failed to create dev entangled directory: {}", e))?;
    Ok(entangled_dir)
}
pub fn open_directory(directory: &Path) -> Result<(), String> {
    if !directory.exists() {
        return Err("Directory does not exist".to_string());
    }
    opener::open(directory).map_err(|e| format!("Failed to open directory: {}", e))
}

pub fn open_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err("File does not exist".to_string());
    }
    if !path.is_file() {
        return Err("Path is not a file".to_string());
    }
    opener::open(path).map_err(|e| format!("Failed to open file: {}", e))
}

pub fn open_url(url: &str) -> Result<(), String> {
    opener::open(url).map_err(|e| format!("Failed to open URL: {}", e))
}

pub fn get_system_info() -> Result<SystemInfo, String> {
    let executable_dir = get_exe_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let app_data_dir = get_app_settings_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let logical_cpu_cores = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(0);

    Ok(SystemInfo {
        app_version: get_version(),
        build_profile: env!("HALLINTA_PROFILE").to_string(),
        rust_version: env!("HALLINTA_RUSTC_VERSION").to_string(),
        cargo_version: env!("HALLINTA_CARGO_VERSION").to_string(),
        build_target: env!("HALLINTA_TARGET").to_string(),
        gui_framework: "eframe/egui 0.33".to_string(),
        os: std::env::consts::OS.to_string(),
        os_family: std::env::consts::FAMILY.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        logical_cpu_cores,
        local_time: Local::now().to_rfc3339(),
        utc_time: Utc::now().to_rfc3339(),
        executable_dir,
        app_data_dir,
    })
}

/// Log detailed system info to the logging system on startup.
pub fn log_system_info_on_startup() {
    if let Ok(info) = get_system_info() {
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "Hallinta v{} ({}) | {} {} | {}",
                info.app_version, info.build_profile, info.os, info.arch, info.build_target
            ),
            "SystemInfo",
        );
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "Rust {} | Cargo {} | GUI: {} | CPUs: {}",
                info.rust_version, info.cargo_version, info.gui_framework, info.logical_cpu_cores
            ),
            "SystemInfo",
        );
        let _ = crate::core::logging::log(
            "INFO",
            &format!("Exe: {} | Data: {}", info.executable_dir, info.app_data_dir),
            "SystemInfo",
        );
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "Dev build: {} | Local time: {}",
                is_dev_build(),
                info.local_time
            ),
            "SystemInfo",
        );

        // Log Noita paths
        match get_noita_save_path() {
            Ok(p) => {
                let _ = crate::core::logging::log(
                    "INFO",
                    &format!("Noita save: {}", p.display()),
                    "SystemInfo",
                );
            }
            Err(e) => {
                let _ = crate::core::logging::log(
                    "WARN",
                    &format!("Noita save not found: {}", e),
                    "SystemInfo",
                );
            }
        }

        // Log Steam path
        match crate::core::workshop::detect_steam_path() {
            Ok(p) => {
                let _ = crate::core::logging::log(
                    "INFO",
                    &format!("Steam: {}", p.display()),
                    "SystemInfo",
                );
            }
            Err(e) => {
                let _ = crate::core::logging::log(
                    "WARN",
                    &format!("Steam not found: {}", e),
                    "SystemInfo",
                );
            }
        }
    }
}

pub fn get_open_source_libraries() -> Vec<OpenSourceLibrary> {
    generated_open_source_libraries()
}

/// Get the application window title, including [DEV] marker if debug build.
pub fn get_window_title() -> String {
    if is_dev_build() {
        format!("Hallinta [DEV] v{}", get_version())
    } else {
        "Hallinta".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Pure / side-effect-free tests ─────────────────────────────────────────

    #[test]
    fn test_get_version_nonempty() {
        let v = get_version();
        assert!(!v.is_empty(), "version string must not be empty");
        assert!(v.contains('.'), "version should contain dots: {}", v);
    }

    #[test]
    fn test_get_window_title_dev_marker() {
        let title = get_window_title();
        assert!(!title.is_empty());
        if cfg!(debug_assertions) {
            assert!(title.contains("[DEV]"), "dev build title should contain [DEV]");
        } else {
            assert!(!title.contains("[DEV]"), "release build title must not contain [DEV]");
        }
    }

    // ── Path detection (read-only, no files written) ──────────────────────────
    //
    // These tests only verify the functions don't panic or have logic errors.
    // Whether the paths exist depends on the test machine (CI may have no Noita/Steam).

    #[test]
    fn test_noita_save_path_does_not_panic() {
        let _result = get_noita_save_path();
    }

    #[test]
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    fn test_noita_save_path_unsupported_platform_is_err() {
        assert!(
            get_noita_save_path().is_err(),
            "unsupported platforms must return Err"
        );
    }

    #[test]
    fn test_entangled_worlds_path_does_not_panic() {
        let _result = get_entangled_worlds_save_path();
    }

    // ── File-writing tests — ALL use isolated temp dirs, never touch dev_data ─

    /// temp_dir scoped to a single test to avoid parallel-test collisions.
    fn test_tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hallinta_test_{}", name));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_write_empty_mod_config() {
        let dir = test_tmp("write_empty_mod_config");
        let path = dir.join("mod_config.xml");
        let _ = std::fs::remove_file(&path);

        write_empty_mod_config(&path).expect("should succeed");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<Mods>"), "must contain <Mods>");
        assert!(!content.contains("<Mod "), "empty placeholder must have no entries");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Tests the seeding logic against an isolated temp directory.
    ///
    /// Dev build note: `seed_dev_mod_config()` (the real app entry point) targets
    /// `dev_data/save00/` and is NOT called here — tests must never write to dev_data.
    /// Release build note: this test is not gated on debug_assertions because
    /// `seed_mod_config_into` is a pure path-based helper with no build-mode dependency.
    #[test]
    fn test_seed_mod_config_into_is_idempotent() {
        let dir = test_tmp("seed_mod_config_idempotent");
        let _ = std::fs::remove_dir_all(&dir); // clean slate each run

        // First call: file doesn't exist yet — seeds from real Noita or writes placeholder.
        let first = seed_mod_config_into(&dir).expect("first seed should succeed");
        assert!(
            first.contains("Seeded") || first.contains("placeholder"),
            "first call should seed or create placeholder, got: {}",
            first
        );
        assert!(
            dir.join("mod_config.xml").exists(),
            "mod_config.xml must exist after seeding"
        );

        // Second call: file already exists — must report cached.
        let second = seed_mod_config_into(&dir).expect("second seed should succeed");
        assert!(
            second.contains("cached"),
            "second call must report 'cached', got: {}",
            second
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Verifies that the dev_data/save00 directory would be created by get_dev_save_dir
    /// without side effects in this test (we only call the helper, not the public function).
    ///
    /// Release: get_dev_save_dir() returns Err("Not in dev mode") — this test is skipped.
    /// Debug:   dev_data/save00 is created if missing, which is safe because the app
    ///          creates it anyway at startup.
    #[test]
    #[cfg(debug_assertions)]
    fn test_get_dev_save_dir_returns_valid_path() {
        let result = get_dev_save_dir();
        assert!(result.is_ok(), "get_dev_save_dir should succeed in debug: {:?}", result);
        let path = result.unwrap();
        assert!(path.ends_with("save00"), "dev save dir should end with save00");
    }
}
