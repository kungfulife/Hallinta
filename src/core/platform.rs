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

/// Recursively copy all contents of `src_dir` into `dst_dir`.
/// Overwrites existing files. Creates `dst_dir` if it doesn't exist.
fn copy_dir_recursive(src_dir: &Path, dst_dir: &Path) -> Result<u64, String> {
    fs::create_dir_all(dst_dir)
        .map_err(|e| format!("Failed to create dir {}: {}", dst_dir.display(), e))?;
    let mut count = 0u64;
    let entries = fs::read_dir(src_dir)
        .map_err(|e| format!("Failed to read dir {}: {}", src_dir.display(), e))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| format!("Failed to read entry in {}: {}", src_dir.display(), e))?;
        let src_path = entry.path();
        let dst_path = dst_dir.join(entry.file_name());
        if src_path.is_dir() {
            count += copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {}: {}", src_path.display(), e))?;
            count += 1;
        }
    }
    Ok(count)
}

/// Directory inside dev_data that holds pristine snapshots of the real files
/// taken at startup, used to restore on exit.
fn get_originals_dir() -> Result<PathBuf, String> {
    let dir = get_data_dir()?.join(".originals");
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create .originals dir: {}", e))?;
    Ok(dir)
}

/// Seed the dev sandbox with a full copy of real save data.
///
/// **Step 1 — Snapshot originals:** Before touching anything, back up the real
/// `mod_config.xml` (and entangled equivalent) into `dev_data/.originals/` so
/// we can restore them on exit even if something goes wrong.
///
/// **Step 2 — Seed sandbox:**
/// - Initial run (no `dev_data/save00/mod_config.xml`): full recursive copy of
///   real save dirs into `dev_data/`.
/// - Subsequent runs: sandbox is preserved from the previous session (no
///   overwrite). Dev changes persist across dev runs.
pub fn seed_dev_sandbox() -> Result<String, String> {
    let dev_save_dir = get_dev_save_dir()?;
    let dev_ew_dir = get_dev_entangled_dir()?;
    let originals = get_originals_dir()?;
    let mut messages = Vec::new();

    let is_initial = !dev_save_dir.join("mod_config.xml").exists();

    // ── Step 1: Snapshot real files into .originals ────────────────────
    if let Ok(real_save) = get_noita_save_path() {
        let real_config = real_save.join("mod_config.xml");
        if real_config.exists() {
            let orig_save = originals.join("save00");
            fs::create_dir_all(&orig_save)
                .map_err(|e| format!("Failed to create .originals/save00: {}", e))?;
            fs::copy(&real_config, orig_save.join("mod_config.xml"))
                .map_err(|e| format!("Failed to snapshot real mod_config.xml: {}", e))?;
            messages.push("Snapshotted real mod_config.xml".to_string());
        }
    }

    if let Ok(real_ew) = get_entangled_worlds_save_path() {
        let real_ew_config = real_ew.join("mod_config.xml");
        if real_ew_config.exists() {
            let orig_ew = originals.join("entangled_worlds");
            fs::create_dir_all(&orig_ew)
                .map_err(|e| format!("Failed to create .originals/entangled_worlds: {}", e))?;
            fs::copy(&real_ew_config, orig_ew.join("mod_config.xml"))
                .map_err(|e| format!("Failed to snapshot real entangled mod_config.xml: {}", e))?;
            messages.push("Snapshotted real entangled mod_config.xml".to_string());
        }
    }

    // ── Step 2: Seed the dev sandbox ───────────────────────────────────
    if is_initial {
        // First run: full copy of real dirs into dev sandbox
        match get_noita_save_path() {
            Ok(real_save) if real_save.exists() => {
                let count = copy_dir_recursive(&real_save, &dev_save_dir)?;
                messages.push(format!(
                    "Initial seed: copied {} file(s) from {}",
                    count,
                    real_save.display()
                ));
            }
            Ok(real_save) => {
                messages.push(format!("Real Noita save dir does not exist: {}", real_save.display()));
                ensure_mod_config_exists(&dev_save_dir)?;
            }
            Err(e) => {
                messages.push(format!("Could not detect Noita save: {}", e));
                ensure_mod_config_exists(&dev_save_dir)?;
            }
        }

        match get_entangled_worlds_save_path() {
            Ok(real_ew) if real_ew.exists() => {
                let count = copy_dir_recursive(&real_ew, &dev_ew_dir)?;
                messages.push(format!(
                    "Initial seed: copied {} entangled file(s)",
                    count
                ));
            }
            _ => {}
        }
    } else {
        // Subsequent runs: keep the dev sandbox as-is (preserves dev session state)
        messages.push("Dev sandbox intact from previous session".to_string());
    }

    Ok(messages.join(" | "))
}

/// Restore the real save directories from the snapshots taken at startup.
/// Called on exit to guarantee real files match their pre-session state.
pub fn restore_real_dirs_from_dev() -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Not in dev mode".to_string());
    }
    let originals = get_originals_dir()?;
    let mut messages = Vec::new();

    // ── Restore real Noita mod_config.xml ──────────────────────────────
    let orig_config = originals.join("save00").join("mod_config.xml");
    if orig_config.exists() {
        if let Ok(real_save) = get_noita_save_path() {
            let real_config = real_save.join("mod_config.xml");
            fs::copy(&orig_config, &real_config)
                .map_err(|e| format!("Failed to restore real mod_config.xml: {}", e))?;
            messages.push(format!("Restored real mod_config.xml at {}", real_save.display()));
        }
    }

    // ── Restore real Entangled Worlds mod_config.xml ───────────────────
    let orig_ew_config = originals.join("entangled_worlds").join("mod_config.xml");
    if orig_ew_config.exists() {
        if let Ok(real_ew) = get_entangled_worlds_save_path() {
            let real_ew_config = real_ew.join("mod_config.xml");
            fs::copy(&orig_ew_config, &real_ew_config)
                .map_err(|e| format!("Failed to restore real entangled mod_config.xml: {}", e))?;
            messages.push(format!("Restored real entangled mod_config.xml at {}", real_ew.display()));
        }
    }

    // Clean up .originals
    let _ = fs::remove_dir_all(&originals);
    messages.push("Cleaned up .originals".to_string());

    Ok(messages.join(" | "))
}

/// Ensure mod_config.xml exists in the given directory (create empty placeholder if not).
fn ensure_mod_config_exists(save_dir: &Path) -> Result<(), String> {
    let config_path = save_dir.join("mod_config.xml");
    if !config_path.exists() {
        let placeholder = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n</Mods>";
        fs::write(&config_path, placeholder)
            .map_err(|e| format!("Failed to create placeholder mod_config.xml: {}", e))?;
    }
    Ok(())
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
                "Rust {} | Cargo {} | GUI: {}",
                info.rust_version, info.cargo_version, info.gui_framework
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

    #[repr(C)]
    struct POINT {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct RECT {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

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
    fn test_ensure_mod_config_creates_placeholder() {
        let dir = test_tmp("ensure_mod_config_placeholder");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        ensure_mod_config_exists(&dir).expect("should succeed");
        let content = std::fs::read_to_string(dir.join("mod_config.xml")).unwrap();
        assert!(content.contains("<Mods>"), "must contain <Mods>");
        assert!(!content.contains("<Mod "), "empty placeholder must have no entries");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Tests that ensure_mod_config_exists creates a placeholder and is idempotent.
    #[test]
    fn test_ensure_mod_config_exists_is_idempotent() {
        let dir = test_tmp("ensure_mod_config");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // First call: creates placeholder
        ensure_mod_config_exists(&dir).expect("first call should succeed");
        assert!(dir.join("mod_config.xml").exists());
        let content1 = std::fs::read_to_string(dir.join("mod_config.xml")).unwrap();
        assert!(content1.contains("<Mods>"));

        // Second call: file already exists — no-op
        ensure_mod_config_exists(&dir).expect("second call should succeed");
        let content2 = std::fs::read_to_string(dir.join("mod_config.xml")).unwrap();
        assert_eq!(content1, content2);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Tests that copy_dir_recursive copies files and subdirectories.
    #[test]
    fn test_copy_dir_recursive() {
        let src = test_tmp("copy_src");
        let dst = test_tmp("copy_dst");
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
        std::fs::create_dir_all(src.join("subdir")).unwrap();
        std::fs::write(src.join("a.txt"), "hello").unwrap();
        std::fs::write(src.join("subdir").join("b.txt"), "world").unwrap();

        let count = copy_dir_recursive(&src, &dst).expect("copy should succeed");
        assert_eq!(count, 2);
        assert_eq!(std::fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(dst.join("subdir").join("b.txt")).unwrap(),
            "world"
        );

        std::fs::remove_dir_all(&src).ok();
        std::fs::remove_dir_all(&dst).ok();
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
