use crate::core::backup::add_directory_to_zip;
use crate::core::settings::get_data_dir;
use crate::models::{SessionInfo, SessionStatus, SnapshotEntry};
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

// --- Session CRUD ---

pub fn create_session(
    preset_name: &str,
    session_name: &str,
    locked_mods: &[crate::models::ModEntry],
) -> Result<SessionInfo, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(preset_name));
    let session_id = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let session_dir = preset_dir.join(&session_id);
    fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session directory: {}", e))?;

    let session = SessionInfo {
        id: session_id,
        name: session_name.to_string(),
        preset_name: preset_name.to_string(),
        started_at: Utc::now().to_rfc3339(),
        ended_at: None,
        status: SessionStatus::Active,
        snapshot_count: 0,
        locked_mods: locked_mods.to_vec(),
    };
    save_session(&session)?;
    Ok(session)
}

pub fn save_session(session: &SessionInfo) -> Result<(), String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(&session.preset_name));
    let session_dir = preset_dir.join(&session.id);
    fs::create_dir_all(&session_dir)
        .map_err(|e| format!("Failed to create session directory: {}", e))?;
    let meta_path = session_dir.join("session.json");
    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;
    fs::write(&meta_path, json).map_err(|e| format!("Failed to write session metadata: {}", e))?;
    Ok(())
}

pub fn load_session(preset_name: &str, session_id: &str) -> Result<SessionInfo, String> {
    let monitor_dir = get_monitor_dir()?;
    let meta_path = monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id)
        .join("session.json");
    let content =
        fs::read_to_string(&meta_path).map_err(|e| format!("Failed to read session: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse session: {}", e))
}

pub fn list_sessions(preset_name: &str) -> Result<Vec<SessionInfo>, String> {
    let monitor_dir = get_monitor_dir()?;
    let preset_dir = monitor_dir.join(sanitize_dirname(preset_name));
    if !preset_dir.exists() {
        return Ok(Vec::new());
    }
    let mut sessions = Vec::new();
    for entry in fs::read_dir(&preset_dir).map_err(|e| format!("Failed to read: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        if entry.path().is_dir() {
            let meta_path = entry.path().join("session.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(session) = serde_json::from_str::<SessionInfo>(&content) {
                        sessions.push(session);
                    }
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(sessions)
}

pub fn list_paused_sessions(preset_name: &str) -> Result<Vec<SessionInfo>, String> {
    Ok(list_sessions(preset_name)?
        .into_iter()
        .filter(|s| s.status == SessionStatus::Paused)
        .collect())
}

pub fn generate_session_name() -> String {
    chrono::Local::now()
        .format("Session %Y-%m-%d %H:%M")
        .to_string()
}

// --- Session-scoped snapshots ---

pub fn create_snapshot_in_session(
    noita_dir: &str,
    preset_name: &str,
    session_id: &str,
    include_save01: bool,
    include_entangled: bool,
    entangled_dir: Option<&str>,
) -> Result<String, String> {
    let monitor_dir = get_monitor_dir()?;
    let session_dir = monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id);
    if !session_dir.exists() {
        fs::create_dir_all(&session_dir)
            .map_err(|e| format!("Failed to create session directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("snapshot_{}.zip", timestamp);
    let zip_path = session_dir.join(&filename);

    let file =
        fs::File::create(&zip_path).map_err(|e| format!("Failed to create snapshot zip: {}", e))?;
    let mut zip = ZipWriter::new(file);

    let save00_path = PathBuf::from(noita_dir);
    if save00_path.exists() {
        add_directory_to_zip(&mut zip, &save00_path, "save00")?;
    }

    if include_save01 {
        if let Some(parent) = save00_path.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
            }
        }
    }

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

pub fn list_session_snapshots(
    preset_name: &str,
    session_id: &str,
) -> Result<Vec<SnapshotEntry>, String> {
    let monitor_dir = get_monitor_dir()?;
    let session_dir = monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id);
    if !session_dir.exists() {
        return Ok(Vec::new());
    }
    let mut snapshots = Vec::new();
    for entry in fs::read_dir(&session_dir).map_err(|e| format!("Failed to read: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "zip") {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let metadata = fs::metadata(&path).map_err(|e| format!("Metadata error: {}", e))?;
            let modified = metadata
                .modified()
                .map(|t| {
                    let dt: chrono::DateTime<Utc> = t.into();
                    dt.to_rfc3339()
                })
                .unwrap_or_default();
            snapshots.push(SnapshotEntry {
                filename,
                session_id: session_id.to_string(),
                timestamp: modified,
                size_bytes: metadata.len(),
            });
        }
    }
    snapshots.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(snapshots)
}

pub fn cleanup_session_snapshots(
    preset_name: &str,
    session_id: &str,
    keep_count: usize,
) -> Result<u32, String> {
    let monitor_dir = get_monitor_dir()?;
    let session_dir = monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id);
    if !session_dir.exists() {
        return Ok(0);
    }
    let mut files: Vec<_> = fs::read_dir(&session_dir)
        .map_err(|e| format!("Failed to read session directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "zip"))
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
        time_a.cmp(&time_b)
    });
    let to_remove = files.len() - keep_count;
    let mut deleted = 0u32;
    for entry in files.into_iter().take(to_remove) {
        if fs::remove_file(entry.path()).is_ok() {
            deleted += 1;
        }
    }
    Ok(deleted)
}

pub fn delete_session_snapshots(preset_name: &str, session_id: &str) -> Result<(), String> {
    let monitor_dir = get_monitor_dir()?;
    let session_dir = monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id);
    if session_dir.exists() {
        fs::remove_dir_all(&session_dir).map_err(|e| format!("Failed to delete session: {}", e))?;
    }
    Ok(())
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

pub fn get_snapshot_path(
    preset_name: &str,
    session_id: &str,
    filename: &str,
) -> Result<PathBuf, String> {
    let monitor_dir = get_monitor_dir()?;
    Ok(monitor_dir
        .join(sanitize_dirname(preset_name))
        .join(session_id)
        .join(filename))
}

// --- Change detection ---

pub fn scan_save_dirs_mtime(
    noita_dir: &str,
    include_save01: bool,
    entangled_dir: Option<&str>,
) -> u64 {
    let mut max_mtime: u64 = 0;
    let save00 = PathBuf::from(noita_dir);
    if save00.exists() {
        max_mtime = max_mtime.max(dir_max_mtime(&save00));
    }
    if include_save01 {
        if let Some(parent) = save00.parent() {
            let save01 = parent.join("save01");
            if save01.exists() {
                max_mtime = max_mtime.max(dir_max_mtime(&save01));
            }
        }
    }
    if let Some(ew) = entangled_dir {
        if !ew.is_empty() {
            let ew_path = PathBuf::from(ew);
            if ew_path.exists() {
                max_mtime = max_mtime.max(dir_max_mtime(&ew_path));
            }
        }
    }
    max_mtime
}

fn dir_max_mtime(dir: &PathBuf) -> u64 {
    let mut max: u64 = 0;
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                let epoch = modified
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                max = max.max(epoch);
            }
        }
    }
    max
}
