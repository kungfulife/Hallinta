use crate::models::SessionLock;
use crate::settings::get_data_dir;
use chrono::Utc;
use serde_json;
use std::fs;
use std::path::{Path, PathBuf};
#[tauri::command]
pub(crate) fn create_session_lock(dev_mode_active: bool, original_mod_config_path: String) -> Result<(), String> {
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
pub(crate) fn remove_session_lock() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let lock_path = data_dir.join(".hallinta_session");
    if lock_path.exists() {
        fs::remove_file(&lock_path)
            .map_err(|e| format!("Failed to remove session lock: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn check_session_lock() -> Result<Option<SessionLock>, String> {
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

pub(crate) fn is_process_running(pid: u32) -> bool {
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
pub(crate) fn cache_and_overwrite_mod_config(real_noita_dir: String, dev_data_dir: String) -> Result<(), String> {
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
pub(crate) fn revert_mod_config(real_noita_dir: String) -> Result<(), String> {
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
pub(crate) fn check_mod_config_cache_exists() -> Result<bool, String> {
    let data_dir = get_data_dir()?;
    let cache_path = data_dir.join(".hallinta_original_mod_config.xml");
    Ok(cache_path.exists())
}

pub(crate) fn revert_mod_config_internal() {
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


