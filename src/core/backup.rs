use crate::core::logging;
use crate::core::settings::get_data_dir;
use crate::models::{AppSettings, BackupInfo, ModEntry, RestoreOptions};
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read as IoRead, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn add_directory_to_zip(
    zip: &mut ZipWriter<fs::File>,
    source_dir: &Path,
    prefix: &str,
) -> Result<(), String> {
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(source_dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                let _ = logging::log("WARN", &format!("Backup: could not read entry: {}", e), "Backup");
                continue;
            }
        };

        let path = entry.path();
        let relative = path
            .strip_prefix(source_dir)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?;

        let archive_name = if prefix.is_empty() {
            relative.to_string_lossy().to_string()
        } else {
            format!("{}/{}", prefix, relative.to_string_lossy())
        };

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
                    let _ = logging::log("WARN", &format!("Backup: could not read file {}: {}", path.display(), e), "Backup");
                }
            }
        }
    }

    Ok(())
}

pub fn create_backup(
    noita_dir: &Path,
    include_save01: bool,
    include_presets: bool,
    include_entangled: bool,
    entangled_dir: Option<&Path>,
) -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        fs::create_dir_all(&backups_dir)
            .map_err(|e| format!("Failed to create backups directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("hallinta_backup_{}.zip", timestamp);
    let zip_path = backups_dir.join(&filename);

    let file =
        fs::File::create(&zip_path).map_err(|e| format!("Failed to create backup zip: {}", e))?;
    let mut zip = ZipWriter::new(file);

    // Always include save00
    if noita_dir.exists() {
        add_directory_to_zip(&mut zip, noita_dir, "save00")?;
    }

    // Optionally include save01
    if include_save01 {
        if let Some(parent) = noita_dir.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
            }
        }
    }

    // Optionally include presets
    if include_presets {
        let presets_path = data_dir.join("presets.json");
        if presets_path.exists() {
            let options: FileOptions<()> =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("presets.json", options)
                .map_err(|e| format!("Failed to add presets to zip: {}", e))?;
            let data =
                fs::read(&presets_path).map_err(|e| format!("Failed to read presets: {}", e))?;
            zip.write_all(&data)
                .map_err(|e| format!("Failed to write presets to zip: {}", e))?;
        }
    }

    // Optionally include Entangled Worlds
    if include_entangled {
        if let Some(ew_path) = entangled_dir {
            if ew_path.exists() {
                add_directory_to_zip(&mut zip, ew_path, "entangled_worlds")?;
            }
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish backup zip: {}", e))?;

    Ok(filename)
}

pub fn list_backups() -> Result<Vec<BackupInfo>, String> {
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
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let metadata =
                fs::metadata(&path).map_err(|e| format!("Failed to read backup metadata: {}", e))?;
            let size_bytes = metadata.len();
            let modified = metadata
                .modified()
                .map(|t| {
                    let datetime: chrono::DateTime<Utc> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();

            let (contains_save00, contains_save01, contains_presets, contains_entangled) =
                peek_zip_contents(&path);

            backups.push(BackupInfo {
                filename,
                timestamp: modified,
                size_bytes,
                contains_save00,
                contains_save01,
                contains_presets,
                contains_entangled,
            });
        }
    }

    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(backups)
}

fn peek_zip_contents(path: &Path) -> (bool, bool, bool, bool) {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return (false, false, false, false),
    };
    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return (false, false, false, false),
    };

    let mut has_save00 = false;
    let mut has_save01 = false;
    let mut has_presets = false;
    let mut has_entangled = false;

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            if name.starts_with("save00/") {
                has_save00 = true;
            }
            if name.starts_with("save01/") {
                has_save01 = true;
            }
            if name == "presets.json" {
                has_presets = true;
            }
            if name.starts_with("entangled_worlds/") {
                has_entangled = true;
            }
        }
    }

    (has_save00, has_save01, has_presets, has_entangled)
}

pub fn delete_backup(filename: &str) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(filename);
    let backups_dir = data_dir.join("backups");
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }
    if !backup_path.starts_with(&backups_dir) {
        return Err("Invalid backup path".to_string());
    }
    fs::remove_file(&backup_path).map_err(|e| format!("Failed to delete backup: {}", e))?;
    Ok(())
}

pub fn cleanup_old_backups(max_age_days: u32) -> Result<u32, String> {
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

pub fn get_backup_contents(filename: &str) -> Result<BackupInfo, String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let metadata =
        fs::metadata(&backup_path).map_err(|e| format!("Failed to read backup metadata: {}", e))?;
    let modified = metadata
        .modified()
        .map(|t| {
            let datetime: chrono::DateTime<Utc> = t.into();
            datetime.to_rfc3339()
        })
        .unwrap_or_default();

    let (has_save00, has_save01, has_presets, has_entangled) = peek_zip_contents(&backup_path);

    Ok(BackupInfo {
        filename: filename.to_string(),
        timestamp: modified,
        size_bytes: metadata.len(),
        contains_save00: has_save00,
        contains_save01: has_save01,
        contains_presets: has_presets,
        contains_entangled: has_entangled,
    })
}

pub fn restore_backup(
    filename: &str,
    noita_dir: &Path,
    options: &RestoreOptions,
    entangled_dir: Option<&Path>,
) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let file =
        fs::File::open(&backup_path).map_err(|e| format!("Failed to open backup: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read backup zip: {}", e))?;

    let save01_target = noita_dir
        .parent()
        .map(|p| p.join("save01"))
        .ok_or_else(|| "Cannot determine save01 path".to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let entry_name = entry.name().to_string();

        let target_path = if entry_name.starts_with("save00/") && options.restore_save00 {
            let relative = entry_name.strip_prefix("save00/").unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            Some(noita_dir.join(relative))
        } else if entry_name.starts_with("save01/") && options.restore_save01 {
            let relative = entry_name.strip_prefix("save01/").unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            Some(save01_target.join(relative))
        } else if entry_name == "presets.json" && options.restore_presets {
            Some(data_dir.join("presets.json"))
        } else if entry_name.starts_with("entangled_worlds/")
            && options.restore_entangled
            && entangled_dir.is_some()
        {
            let relative = entry_name
                .strip_prefix("entangled_worlds/")
                .unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            entangled_dir.map(|d| d.join(relative))
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
                entry
                    .read_to_end(&mut buf)
                    .map_err(|e| format!("Failed to read zip entry data: {}", e))?;
                fs::write(&target, &buf)
                    .map_err(|e| format!("Failed to write restored file: {}", e))?;
            }
        }
    }

    Ok(())
}

pub fn create_upgrade_backup(
    settings: &AppSettings,
    presets: &BTreeMap<String, Vec<ModEntry>>,
    old_version: &str,
    new_version: &str,
) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let upgrade_backup_dir = data_dir.join("upgrade_backups");
    if !upgrade_backup_dir.exists() {
        fs::create_dir_all(&upgrade_backup_dir)
            .map_err(|e| format!("Failed to create upgrade backup directory: {}", e))?;
    }

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let zip_file_path = upgrade_backup_dir.join(format!(
        "upgrade_backup_from_v{}_to_v{}_{}.zip",
        old_version, new_version, timestamp
    ));

    let settings_json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    let presets_json = serde_json::to_string_pretty(presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;

    let file = fs::File::create(&zip_file_path)
        .map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip.start_file("settings.json", options)
        .map_err(|e| format!("Failed to start file in zip: {}", e))?;
    zip.write_all(settings_json.as_bytes())
        .map_err(|e| format!("Failed to write settings to zip: {}", e))?;

    zip.start_file("presets.json", options)
        .map_err(|e| format!("Failed to start file in zip: {}", e))?;
    zip.write_all(presets_json.as_bytes())
        .map_err(|e| format!("Failed to write presets to zip: {}", e))?;

    let noita_dir = &settings.noita_dir;
    if !noita_dir.is_empty() {
        let save00_path = PathBuf::from(noita_dir);
        if save00_path.exists() {
            add_directory_to_zip(&mut zip, &save00_path, "save00")?;
        }
        if let Some(parent) = save00_path.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
            }
        }
    }

    if !settings.entangled_dir.is_empty() {
        let ew_path = PathBuf::from(&settings.entangled_dir);
        if ew_path.exists() {
            add_directory_to_zip(&mut zip, &ew_path, "entangled_worlds")?;
        }
    }

    zip.finish()
        .map_err(|e| format!("Failed to finish zip: {}", e))?;

    cleanup_old_upgrade_backups(&upgrade_backup_dir, 5)?;
    Ok(())
}

fn cleanup_old_upgrade_backups(upgrade_backup_dir: &Path, keep_count: usize) -> Result<(), String> {
    if !upgrade_backup_dir.exists() {
        return Ok(());
    }

    let mut backups: Vec<_> = fs::read_dir(upgrade_backup_dir)
        .map_err(|e| format!("Failed to read upgrade_backups directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "zip"))
        .collect();

    if backups.len() <= keep_count {
        return Ok(());
    }

    backups.sort_by(|a, b| {
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

    for old_backup in backups.into_iter().skip(keep_count) {
        let _ = fs::remove_file(old_backup.path());
    }

    Ok(())
}
