use crate::core::logging;
use crate::core::settings::get_data_dir;
use crate::models::{AppSettings, BackupInfo, ModEntry, RestoreOptions};
use chrono::Utc;
use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read as IoRead, Write};
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::FileOptions;

/// Reject zip entry names containing path traversal or absolute components.
/// Prevents Zip Slip where a crafted archive escapes the target directory.
fn is_safe_relative(rel: &str) -> bool {
    let p = Path::new(rel);
    p.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

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
                let _ = logging::log(
                    "WARN",
                    &format!("Backup: could not read entry: {}", e),
                    "Backup",
                );
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
                    let _ = logging::log(
                        "WARN",
                        &format!("Backup: could not read file {}: {}", path.display(), e),
                        "Backup",
                    );
                }
            }
        }
    }

    Ok(())
}

fn validate_save00_source(noita_dir: &Path) -> Result<(), String> {
    if noita_dir.as_os_str().is_empty() {
        return Err(
            "No Noita save directory configured; set it in Settings before creating backups"
                .to_string(),
        );
    }
    if !noita_dir.exists() {
        return Err(format!(
            "Noita save directory does not exist: {}",
            noita_dir.display()
        ));
    }
    if !noita_dir.is_dir() {
        return Err(format!(
            "Noita save directory is not a directory: {}",
            noita_dir.display()
        ));
    }
    Ok(())
}

pub fn create_backup(
    noita_dir: &Path,
    include_save01: bool,
    include_presets: bool,
    include_entangled: bool,
    entangled_dir: Option<&Path>,
    backup_name: &str,
) -> Result<String, String> {
    validate_save00_source(noita_dir)?;

    let data_dir = get_data_dir()?;
    let backups_dir = data_dir.join("backups");
    if !backups_dir.exists() {
        fs::create_dir_all(&backups_dir)
            .map_err(|e| format!("Failed to create backups directory: {}", e))?;
    }

    let _ = logging::log(
        "INFO",
        &format!(
            "Creating backup: save01={} presets={} entangled={}",
            include_save01, include_presets, include_entangled
        ),
        "Backup",
    );

    let name = normalize_manual_backup_name(backup_name, "Backup")?;
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let (filename, zip_path, file) = (1..)
        .find_map(|suffix| {
            let filename = manual_backup_filename(&name, &timestamp, suffix);
            let zip_path = backups_dir.join(&filename);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&zip_path)
            {
                Ok(file) => Some(Ok((filename, zip_path, file))),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(format!("Failed to create backup zip: {error}"))),
            }
        })
        .expect("unbounded suffix iterator must return")?;
    let mut zip = ZipWriter::new(file);

    let result = (|| {
        // Always include save00
        add_directory_to_zip(&mut zip, noita_dir, "save00")?;

        // Optionally include save01
        if include_save01 && let Some(parent) = noita_dir.parent() {
            let save01_path = parent.join("save01");
            if save01_path.exists() {
                add_directory_to_zip(&mut zip, &save01_path, "save01")?;
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
                let data = fs::read(&presets_path)
                    .map_err(|e| format!("Failed to read presets: {}", e))?;
                zip.write_all(&data)
                    .map_err(|e| format!("Failed to write presets to zip: {}", e))?;
            }
        }

        // Optionally include Entangled Worlds
        if include_entangled
            && let Some(ew_path) = entangled_dir
            && ew_path.exists()
        {
            add_directory_to_zip(&mut zip, ew_path, "entangled_worlds")?;
        }

        zip.finish()
            .map_err(|e| format!("Failed to finish backup zip: {}", e))?;
        Ok::<(), String>(())
    })();

    if let Err(error) = result {
        fs::remove_file(&zip_path).ok();
        return Err(error);
    }

    Ok(filename)
}

fn validate_backup_filename(filename: &str) -> Result<(), String> {
    if filename.contains(['/', '\\']) {
        return Err("Invalid backup filename".to_string());
    }
    let mut components = Path::new(filename).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err("Invalid backup filename".to_string());
    }
    if !Path::new(filename)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        return Err("Backup filename must end in .zip".to_string());
    }
    Ok(())
}

pub(crate) fn normalize_manual_backup_name(name: &str, fallback: &str) -> Result<String, String> {
    let normalized = if name.trim().is_empty() {
        fallback.trim()
    } else {
        name.trim()
    };
    if normalized.is_empty() {
        return Err("Backup name cannot be empty".to_string());
    }
    if normalized.chars().count() > 80 {
        return Err("Backup name must be 80 characters or fewer".to_string());
    }
    if normalized
        .chars()
        .any(|character| character.is_control() || "/\\:*?\"<>|".contains(character))
    {
        return Err("Backup name contains an invalid character".to_string());
    }
    Ok(normalized.to_string())
}

fn manual_backup_filename(name: &str, timestamp: &str, suffix: usize) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = true;
    let mut count = 0;

    'characters: for character in name.chars() {
        if character.is_alphanumeric() {
            for lowercase in character.to_lowercase() {
                if count == 40 {
                    break 'characters;
                }
                slug.push(lowercase);
                count += 1;
            }
            previous_was_separator = false;
        } else if !previous_was_separator && count < 40 {
            slug.push('_');
            count += 1;
            previous_was_separator = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("backup");
    }

    let collision_suffix = if suffix > 1 {
        format!("_{suffix}")
    } else {
        String::new()
    };
    format!("hallinta_manual_{slug}_{timestamp}{collision_suffix}.zip")
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
        if path.extension().is_some_and(|ext| ext == "zip") {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to read backup metadata: {}", e))?;
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
    validate_backup_filename(filename)?;
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

pub fn get_backup_contents(filename: &str) -> Result<BackupInfo, String> {
    validate_backup_filename(filename)?;
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
    validate_backup_filename(filename)?;
    let data_dir = get_data_dir()?;
    let backup_path = data_dir.join("backups").join(filename);
    if !backup_path.exists() {
        return Err("Backup file not found".to_string());
    }

    let _ = logging::log(
        "INFO",
        &format!(
            "Restoring backup {}: save00={} save01={} presets={} entangled={}",
            filename,
            options.restore_save00,
            options.restore_save01,
            options.restore_presets,
            options.restore_entangled
        ),
        "Backup",
    );

    let file = fs::File::open(&backup_path).map_err(|e| format!("Failed to open backup: {}", e))?;
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
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
                continue;
            }
            Some(noita_dir.join(relative))
        } else if entry_name.starts_with("save01/") && options.restore_save01 {
            let relative = entry_name.strip_prefix("save01/").unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
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
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
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

/// Restore save data from a snapshot ZIP at an arbitrary path.
pub fn restore_from_path(
    zip_path: &Path,
    noita_dir: &Path,
    options: &RestoreOptions,
    entangled_dir: Option<&Path>,
) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open snapshot: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read snapshot ZIP: {}", e))?;

    let save01_target = noita_dir.parent().map(|p| p.join("save01"));

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {}", e))?;
        let entry_name = entry.name().to_string();

        let target_path = if entry_name.starts_with("save00/") && options.restore_save00 {
            let relative = entry_name.strip_prefix("save00/").unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
                continue;
            }
            Some(noita_dir.join(relative))
        } else if entry_name.starts_with("save01/") && options.restore_save01 {
            let relative = entry_name.strip_prefix("save01/").unwrap_or(&entry_name);
            if relative.is_empty() {
                continue;
            }
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
                continue;
            }
            save01_target.as_ref().map(|t| t.join(relative))
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
            if !is_safe_relative(relative) {
                let _ = logging::log(
                    "WARN",
                    &format!("Refused unsafe zip entry: {}", entry_name),
                    "Backup",
                );
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
                    .map_err(|e| format!("Failed to read ZIP entry data: {}", e))?;
                fs::write(&target, &buf)
                    .map_err(|e| format!("Failed to write restored file: {}", e))?;
            }
        }
    }

    Ok(())
}

fn cleanup_old_upgrade_backups(upgrade_backup_dir: &Path, keep_count: usize) -> Result<(), String> {
    if !upgrade_backup_dir.exists() {
        return Ok(());
    }

    let mut backups: Vec<_> = fs::read_dir(upgrade_backup_dir)
        .map_err(|e| format!("Failed to read upgrade_backups directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "zip"))
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

#[cfg(test)]
mod tests {
    use super::{is_safe_relative, validate_backup_filename};

    #[test]
    fn manual_backup_names_are_validated_and_slugged() {
        assert_eq!(
            super::normalize_manual_backup_name("  Night run  ", "Fallback").unwrap(),
            "Night run"
        );
        assert_eq!(
            super::normalize_manual_backup_name("   ", "Backup 2026-07-11 01-30").unwrap(),
            "Backup 2026-07-11 01-30"
        );
        assert!(super::normalize_manual_backup_name("bad/name", "Fallback").is_err());
        assert!(super::normalize_manual_backup_name("bad\u{7}name", "Fallback").is_err());
        assert!(super::normalize_manual_backup_name(&"x".repeat(81), "Fallback").is_err());
        assert_eq!(
            super::manual_backup_filename("Night run", "20260711_013000", 1),
            "hallinta_manual_night_run_20260711_013000.zip"
        );
        assert_eq!(
            super::manual_backup_filename("Night run", "20260711_013000", 2),
            "hallinta_manual_night_run_20260711_013000_2.zip"
        );
    }

    #[test]
    fn safe_relative_accepts_normal_paths() {
        assert!(is_safe_relative("world.png"));
        assert!(is_safe_relative("subdir/file.txt"));
        assert!(is_safe_relative("a/b/c/d.dat"));
        assert!(is_safe_relative("./file.txt"));
    }

    #[test]
    fn safe_relative_rejects_traversal() {
        assert!(!is_safe_relative("../escape.txt"));
        assert!(!is_safe_relative("ok/../../escape.txt"));
        assert!(!is_safe_relative("a/../../b"));
    }

    #[test]
    fn safe_relative_rejects_absolute() {
        assert!(!is_safe_relative("/etc/passwd"));
        #[cfg(windows)]
        assert!(!is_safe_relative("C:\\Windows\\System32\\evil.dll"));
    }

    #[test]
    fn save00_source_must_exist_before_backup() {
        let missing = std::env::temp_dir().join(format!(
            "hallinta_missing_save00_{}_{}",
            std::process::id(),
            "backup"
        ));
        std::fs::remove_dir_all(&missing).ok();

        let err = super::create_backup(&missing, false, false, false, None, "Test backup")
            .expect_err("missing save00 source should fail backup creation");

        assert!(
            err.contains("Noita save directory does not exist"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn backup_filename_must_be_one_zip_component() {
        assert!(validate_backup_filename("backup.zip").is_ok());
        for invalid in [
            "../backup.zip",
            "folder/backup.zip",
            "folder\\backup.zip",
            "/tmp/backup.zip",
            "C:\\Temp\\backup.zip",
            "backup.txt",
            "..",
        ] {
            assert!(
                validate_backup_filename(invalid).is_err(),
                "accepted unsafe backup filename: {invalid}"
            );
        }
    }
}
