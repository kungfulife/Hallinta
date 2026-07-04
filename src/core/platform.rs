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

pub fn get_git_hash() -> String {
    env!("HALLINTA_GIT_HASH").to_string()
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
/// True when the path is missing or contains no files/subdirectories.
pub fn directory_missing_or_empty(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true)
}

pub fn entangled_dir_usable(path: &str) -> bool {
    let trimmed = path.trim();
    !trimmed.is_empty() && !directory_missing_or_empty(Path::new(trimmed))
}

pub fn save01_dir_for_noita(noita_dir: &str) -> Option<PathBuf> {
    let save00 = PathBuf::from(noita_dir);
    save00.parent().map(|parent| parent.join("save01"))
}

pub fn save01_usable(noita_dir: &str) -> bool {
    save01_dir_for_noita(noita_dir).is_some_and(|path| !directory_missing_or_empty(&path))
}

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
        .map(|n| n.get())
        .unwrap_or(0);
    Ok(SystemInfo {
        app_version: get_version(),
        git_hash: get_git_hash(),
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
                "Hallinta v{} ({}) [{}] | {} {} | {}",
                info.app_version,
                info.build_profile,
                info.git_hash,
                info.os,
                info.arch,
                info.build_target
            ),
            "SystemInfo",
        );
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "Rust {} | Cargo {} | GUI: {} | CPU cores: {}",
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
    }
}

pub fn get_open_source_libraries() -> Vec<OpenSourceLibrary> {
    generated_open_source_libraries()
}

/// Get the application window title, including [DEV] marker if debug build.
pub fn get_window_title() -> String {
    if is_dev_build() {
        format!("Hallinta [DEV] v{} ({})", get_version(), get_git_hash())
    } else {
        "Hallinta".to_string()
    }
}

/// Returns the centered position for a window of `window_size` on the monitor
/// where the cursor currently is. Falls back to `None` if detection fails
/// (caller should use eframe's `centered: true` as fallback).
#[cfg(target_os = "windows")]
pub fn get_cursor_monitor_center(window_w: f32, window_h: f32) -> Option<(f32, f32)> {
    use std::mem;

    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[allow(clippy::upper_case_acronyms)]
    #[repr(C)]
    struct MONITORINFO {
        cb_size: u32,
        rc_monitor: RECT,
        rc_work: RECT,
        dw_flags: u32,
    }

    unsafe extern "system" {
        fn GetCursorPos(lp_point: *mut POINT) -> i32;
        fn MonitorFromPoint(pt: POINT, dw_flags: u32) -> *mut std::ffi::c_void;
        fn GetMonitorInfoW(h_monitor: *mut std::ffi::c_void, lpmi: *mut MONITORINFO) -> i32;
    }

    const MONITOR_DEFAULTTONEAREST: u32 = 0x00000002;

    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor) == 0 {
            return None;
        }

        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }

        let mut info: MONITORINFO = mem::zeroed();
        info.cb_size = mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }

        // Use the work area (excludes taskbar)
        let work = &info.rc_work;
        let mon_w = (work.right - work.left) as f32;
        let mon_h = (work.bottom - work.top) as f32;
        let x = work.left as f32 + (mon_w - window_w) / 2.0;
        let y = work.top as f32 + (mon_h - window_h) / 2.0;

        Some((x.max(work.left as f32), y.max(work.top as f32)))
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
            assert!(
                title.contains("[DEV]"),
                "dev build title should contain [DEV]"
            );
        } else {
            assert!(
                !title.contains("[DEV]"),
                "release build title must not contain [DEV]"
            );
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

    // File-writing helpers that created dev save sandboxes were removed; platform tests
    // intentionally avoid touching repo-local runtime data.
}
