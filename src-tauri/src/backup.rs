use crate::models::{BackupInfo, RestoreOptions};
use crate::settings::get_data_dir;
use chrono::Utc;
use std::fs;
use std::io::{Read as IoRead, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;
pub(crate) fn add_directory_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    source_dir: &Path,
    prefix: &str,
) -> Result<(), String> {
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: could not read entry: {}", e);
                continue;
            }
        };

        let path = entry.path();
        let relative = path.strip_prefix(source_dir)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?;

        let archive_name = if prefix.is_empty() {
            relative.to_string_lossy().to_string()
        } else {
            format!("{}/{}", prefix, relative.to_string_lossy())
        };

        // Normalize path separators for zip
        let archive_name = archive_name.replace('\\', "/");

        if path.is_dir() {
            if !archive_name.is_empty() && archive_name != "/" {
                let dir_name = if archive_name.ends_with('/') {
                    archive_name.clone()
                } else {
                    format!("{}/", archive_name)
                };
                zip.add_directory(&dir_name, options)
                    .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
            }
        } else {
            zip.start_file(&archive_name, options)
                .map_err(|e| format!("Failed to start file in zip: {}", e))?;
            match fs::read(path) {
                Ok(data) => {
                    zip.write_all(&data)
                        .map_err(|e| format!("Failed to write file to zip: {}", e))?;
                }
                Err(e) => {
                    eprintln!("Warning: could not read file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub(crate) async fn create_backup(noita_dir: String, include_save01: bool, include_presets: bool) -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        fs::create_dir_all(&backups_dir)
            .map_err(|e| format!("Failed to create backups directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("hallinta_backup_{}.zip", timestamp);
    let zip_path = backups_dir.join(&filename);

    let noita_dir_clone = noita_dir.clone();
    let data_dir_clone = data_dir.clone();
    let zip_path_clone = zip_path.clone();
    let filename_clone = filename.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::create(&zip_path_clone)
            .map_err(|e| format!("Failed to create backup zip: {}", e))?;
        let mut zip = ZipWriter::new(file);

        // Always include save00
        let save00_path = PathBuf::from(&noita_dir_clone);
        if save00_path.exists() {
            add_directory_to_zip(&mut zip, &save00_path, "save00")?;
        }

        // Optionally include save01 (sibling of save00)
        if include_save01 {
            if let Some(parent) = save00_path.parent() {
                let save01_path = parent.join("save01");
                if save01_path.exists() {
                    add_directory_to_zip(&mut zip, &save01_path, "save01")?;
                }
            }
        }

        // Optionally include presets
        if include_presets {
            let presets_path = data_dir_clone.join("presets.json");
            if presets_path.exists() {
                let options: FileOptions<()> =
                    FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
                zip.start_file("presets.json", options)
                    .map_err(|e| format!("Failed to add presets to zip: {}", e))?;
                let data = fs::read(&presets_path)
                    .map_err(|e| format!("Failed to read presets: {}", e))?;
                zip.write_all(&data)
                    .map_err(|e| format!("Failed to write presets to zip: {}", e))?;
            }
        }

        zip.finish()
            .map_err(|e| format!("Failed to finish backup zip: {}", e))?;

        Ok::<String, String>(filename_clone)
    })
    .await
    .map_err(|e| format!("Backup task failed: {}", e))?
}

#[tauri::command]
pub(crate) fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    let entries = fs::read_dir(&backups_dir)
        .map_err(|e| format!("Failed to read backups directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            let filename = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to read backup metadata: {}", e))?;
            let size_bytes = metadata.len();

            let modified = metadata.modified()
                .map(|t| {
                    let datetime: chrono::DateTime<Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            // Peek inside the zip to see what it contains
            let (contains_save00, contains_save01, contains_presets) = match std::fs::File::open(&path) {
                Ok(file) => {
                    match zip::ZipArchive::new(file) {
                        Ok(mut archive) => {
                            let mut has_save00 = false;
                            let mut has_save01 = false;
                            let mut has_presets = false;
                            for i in 0..archive.len() {
                                if let Ok(entry) = archive.by_index(i) {
                                    let name = entry.name();
                                    if name.starts_with("save00/") { has_save00 = true; }
                                    if name.starts_with("save01/") { has_save01 = true; }
                                    if name == "presets.json" { has_presets = true; }
                                }
                            }
                            (has_save00, has_save01, has_presets)
                        }
                        Err(_) => (false, false, false),
                    }
                }
                Err(_) => (false, false, false),
            };

            backups.push(BackupInfo {
                filename,
                timestamp: modified,
                size_bytes,
                contains_save00,
                contains_save01,
                contains_presets,
            });
        }
    }

    // Sort by timestamp descending (newest first)
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

#[tauri::command]
pub(crate) fn delete_backup(filename: String) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }
    // Ensure path is within backups directory
    let backups_dir = data_dir.join("backups");
    if !backup_path.starts_with(&backups_dir) {
        return Err("Invalid backup path".to_string());
    }
    fs::remove_file(&backup_path)
        .map_err(|e| format!("Failed to delete backup: {}", e))?;
    Ok(())
}

#[tauri::command]
pub(crate) fn cleanup_old_backups(max_age_days: u32) -> Result<u32, String> {
    if max_age_days == 0 {
        return Ok(0);
    }

    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days as u64 * 86400))
        .ok_or_else(|| "Failed to calculate cutoff time".to_string())?;

    let mut deleted = 0u32;
    let entries = fs::read_dir(&backups_dir)
        .map_err(|e| format!("Failed to read backups directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "zip") {
            if let Ok(metadata) = fs::metadata(&path) {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff {
                        if fs::remove_file(&path).is_ok() {
                            deleted += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(deleted)
}

#[tauri::command]
pub(crate) fn get_backup_contents(filename: String) -> Result<BackupInfo, String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let file = std::fs::File::open(&backup_path)
        .map_err(|e| format!("Failed to open backup: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read backup zip: {}", e))?;

    let metadata = fs::metadata(&backup_path)
        .map_err(|e| format!("Failed to read backup metadata: {}", e))?;

    let modified = metadata.modified()
        .map(|t| {
            let datetime: chrono::DateTime<Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_default();

    let mut has_save00 = false;
    let mut has_save01 = false;
    let mut has_presets = false;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name();
            if name.starts_with("save00/") { has_save00 = true; }
            if name.starts_with("save01/") { has_save01 = true; }
            if name == "presets.json" { has_presets = true; }
        }
    }

    Ok(BackupInfo {
        filename,
        timestamp: modified,
        size_bytes: metadata.len(),
        contains_save00: has_save00,
        contains_save01: has_save01,
        contains_presets: has_presets,
    })
}

#[tauri::command]
pub(crate) async fn restore_backup(filename: String, noita_dir: String, options: RestoreOptions) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(&filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let noita_dir_clone = noita_dir.clone();
    let data_dir_clone = data_dir.clone();
    let backup_path_clone = backup_path.clone();

    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&backup_path_clone)
            .map_err(|e| format!("Failed to open backup: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read backup zip: {}", e))?;

        let save00_target = PathBuf::from(&noita_dir_clone);
        let save01_target = save00_target.parent()
            .map(|p| p.join("save01"))
            .ok_or_else(|| "Cannot determine save01 path".to_string())?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;
            let entry_name = entry.name().to_string();

            // Determine the target path based on the entry prefix
            let target_path = if entry_name.starts_with("save00/") && options.restore_save00 {
                let relative = entry_name.strip_prefix("save00/").unwrap_or(&entry_name);
                if relative.is_empty() { continue; }
                Some(save00_target.join(relative))
            } else if entry_name.starts_with("save01/") && options.restore_save01 {
                let relative = entry_name.strip_prefix("save01/").unwrap_or(&entry_name);
                if relative.is_empty() { continue; }
                Some(save01_target.join(relative))
            } else if entry_name == "presets.json" && options.restore_presets {
                Some(data_dir_clone.join("presets.json"))
            } else {
                None
            };

            if let Some(target) = target_path {
                if entry.is_dir() {
                    let _ = fs::create_dir_all(&target);
                } else {
                    if let Some(parent) = target.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf)
                        .map_err(|e| format!("Failed to read zip entry data: {}", e))?;
                    fs::write(&target, &buf)
                        .map_err(|e| format!("Failed to write restored file: {}", e))?;
                }
            }
        }

        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Restore task failed: {}", e))?
}


