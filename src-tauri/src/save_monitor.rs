use crate::backup::add_directory_to_zip;
use crate::settings::get_data_dir;
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use zip::ZipWriter;

fn get_monitor_dir() -> Result<PathBuf, String> {
    let data_dir = get_data_dir()?;
    let monitor_dir = data_dir.join("save_monitor");
    if !monitor_dir.exists() {
        fs::create_dir_all(&monitor_dir)
            .map_err(|e| format!("Failed to create save_monitor directory: {}", e))?;
    }
    Ok(monitor_dir)
}

#[tauri::command]
pub(crate) async fn create_monitor_snapshot(
    noita_dir: String,
    preset_name: String,
    include_entangled: bool,
    entangled_dir: String,
) -> Result<String, String> {
    let monitor_dir = get_monitor_dir()?;

    // Organize snapshots by preset name
    let preset_dir = monitor_dir.join(sanitize_dirname(&preset_name));
    if !preset_dir.exists() {
        fs::create_dir_all(&preset_dir)
            .map_err(|e| format!("Failed to create preset monitor directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{}.zip", timestamp);
    let zip_path = preset_dir.join(&filename);

    let noita_dir_clone = noita_dir.clone();
    let entangled_dir_clone = entangled_dir.clone();
    let zip_path_clone = zip_path.clone();
    let filename_clone = filename.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_path_clone)
            .map_err(|e| format!("Failed to create snapshot zip: {}", e))?;
        let mut zip = ZipWriter::new(file);

        // Always include save00
        let save00_path = PathBuf::from(&noita_dir_clone);
        if save00_path.exists() {
            add_directory_to_zip(&mut zip, &save00_path, "save00")?;
        }

        // Always include save01 (sibling of save00)
        if let Some(parent) = save00_path.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
            }
        }

        // Optionally include Entangled Worlds data
        if include_entangled && !entangled_dir_clone.is_empty() {
            let ew_path = PathBuf::from(&entangled_dir_clone);
            if ew_path.exists() {
                add_directory_to_zip(&mut zip, &ew_path, "entangled_worlds")?;
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finish snapshot zip: {}", e))?;

        Ok::<String, String>(filename_clone)
    })
    .await
    .map_err(|e| format!("Snapshot task failed: {}", e))?
}

#[tauri::command]
pub(crate) fn list_monitor_snapshots(preset_name: String) -> Result<Vec<MonitorSnapshot>, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(&preset_name));

    if !preset_dir.exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();
    let entries = fs::read_dir(&preset_dir)
        .map_err(|e| format!("Failed to read monitor directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to read snapshot metadata: {}", e))?;

            let modified = metadata
                .modified()
                .map(|t| {
                    let datetime: chrono::DateTime<Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            snapshots.push(MonitorSnapshot {
                filename,
                preset_name: preset_name.clone(),
                timestamp: modified,
                size_bytes: metadata.len(),
            });
        }
    }

    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

#[tauri::command]
pub(crate) fn cleanup_monitor_snapshots(
    preset_name: String,
    keep_count: usize,
) -> Result<u32, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(&preset_name));

    if !preset_dir.exists() {
        return Ok(0);
    }

    let mut files: Vec<_> = fs::read_dir(&preset_dir)
        .map_err(|e| format!("Failed to read monitor directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "zip")
        })
        .collect();

    if files.len() <= keep_count {
        return Ok(0);
    }

    files.sort_by(|a, b| {
        let time_a = a
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let time_b = b
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        time_b.cmp(&time_a)
    });

    let mut deleted = 0u32;
    for old in files.into_iter().skip(keep_count) {
        if fs::remove_file(old.path()).is_ok() {
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub(crate) fn clear_monitor_data() -> Result<(), String> {
    let monitor_dir = get_monitor_dir()?;
    if monitor_dir.exists() {
        fs::remove_dir_all(&monitor_dir)
            .map_err(|e| format!("Failed to clear monitor data: {}", e))?;
        fs::create_dir_all(&monitor_dir)
            .map_err(|e| format!("Failed to recreate monitor directory: {}", e))?;
    }
    Ok(())
}

fn sanitize_dirname(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MonitorSnapshot {
    pub filename: String,
    pub preset_name: String,
    pub timestamp: String,
    pub size_bytes: u64,
}
