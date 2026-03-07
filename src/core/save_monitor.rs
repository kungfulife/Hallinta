use crate::core::backup::add_directory_to_zip;
use crate::core::settings::get_data_dir;
use crate::models::MonitorSnapshot;
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

pub fn create_monitor_snapshot(
    noita_dir: &str,
    preset_name: &str,
    include_save01: bool,
    include_entangled: bool,
    entangled_dir: Option<&str>,
) -> Result<String, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(preset_name));
    if !preset_dir.exists() {
        fs::create_dir_all(&preset_dir)
            .map_err(|e| format!("Failed to create preset monitor directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{}.zip", timestamp);
    let zip_path = preset_dir.join(&filename);

    let file =
        fs::File::create(&zip_path).map_err(|e| format!("Failed to create snapshot zip: {}", e))?;
    let mut zip = ZipWriter::new(file);

    // Always include save00
    let save00_path = PathBuf::from(noita_dir);
    if save00_path.exists() {
        add_directory_to_zip(&mut zip, &save00_path, "save00")?;
    }

    // Optionally include save01
    if include_save01 {
        if let Some(parent) = save00_path.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
            }
        }
    }

    // Optionally include Entangled Worlds
    if include_entangled {
        if let Some(ew_dir) = entangled_dir {
            if !ew_dir.is_empty() {
                let ew_path = PathBuf::from(ew_dir);
                if ew_path.exists() {
                    add_directory_to_zip(&mut zip, &ew_path, "entangled_worlds")?;
                }
            }
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish snapshot zip: {}", e))?;

    Ok(filename)
}

pub fn list_monitor_snapshots(preset_name: &str) -> Result<Vec<MonitorSnapshot>, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(preset_name));

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
                preset_name: preset_name.to_string(),
                timestamp: modified,
                size_bytes: metadata.len(),
            });
        }
    }

    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

/// Cleanup old snapshots, keeping every Nth snapshot as "safe" (not deleted).
/// `keep_count` is the max snapshots to keep.
/// `keep_every_nth` marks every Nth oldest snapshot as protected from deletion.
pub fn cleanup_monitor_snapshots(
    preset_name: &str,
    keep_count: usize,
    keep_every_nth: usize,
) -> Result<u32, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(preset_name));

    if !preset_dir.exists() {
        return Ok(0);
    }

    let mut files: Vec<_> = fs::read_dir(&preset_dir)
        .map_err(|e| format!("Failed to read monitor directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "zip"))
        .collect();

    if files.len() <= keep_count {
        return Ok(0);
    }

    // Sort oldest first (ascending by mtime)
    files.sort_by(|a, b| {
        let time_a = a
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let time_b = b
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        time_a.cmp(&time_b)
    });

    let total = files.len();
    let to_remove = total - keep_count;
    let mut deleted = 0u32;

    // The oldest `to_remove` files are candidates for deletion,
    // but we protect every Nth one (1-indexed from oldest).
    for (i, entry) in files.into_iter().enumerate() {
        if i >= to_remove {
            break; // Only consider the oldest excess files
        }
        // Protect every Nth snapshot (1-indexed: 5th, 10th, 15th, ...)
        let position = i + 1; // 1-indexed
        if keep_every_nth > 0 && position % keep_every_nth == 0 {
            continue; // Protected — skip deletion
        }
        if fs::remove_file(entry.path()).is_ok() {
            deleted += 1;
        }
    }

    Ok(deleted)
}

pub fn clear_monitor_data() -> Result<(), String> {
    let monitor_dir = get_monitor_dir()?;
    if monitor_dir.exists() {
        fs::remove_dir_all(&monitor_dir)
            .map_err(|e| format!("Failed to clear monitor data: {}", e))?;
        fs::create_dir_all(&monitor_dir)
            .map_err(|e| format!("Failed to recreate monitor directory: {}", e))?;
    }
    Ok(())
}
