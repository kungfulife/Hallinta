use crate::models::{WorkshopCheckReport, WorkshopInstallState};
use std::path::{Path, PathBuf};

pub const NOITA_APP_ID: &str = "881100";

fn detect_steam_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Ok(path) = detect_steam_from_registry_wow64() {
            return Ok(PathBuf::from(path));
        }
        if let Ok(path) = detect_steam_from_registry_hkcu() {
            return Ok(PathBuf::from(path));
        }
        let common_paths = [
            r"C:\Program Files (x86)\Steam",
            r"C:\Program Files\Steam",
            r"D:\Steam",
            r"D:\Program Files (x86)\Steam",
        ];
        for path in &common_paths {
            if PathBuf::from(path).exists() {
                return Ok(PathBuf::from(path));
            }
        }
        Err("Steam installation not found".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        let home = dirs::home_dir().ok_or("Could not find home directory")?;
        let candidates = [
            home.join(".steam").join("steam"),
            home.join(".local").join("share").join("Steam"),
        ];
        for path in &candidates {
            if path.exists() {
                return Ok(path.clone());
            }
        }
        Err("Steam installation not found".to_string())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Steam path detection is not supported on this platform".to_string())
    }
}

#[cfg(target_os = "windows")]
fn detect_steam_from_registry_wow64() -> Result<String, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm
        .open_subkey(r"SOFTWARE\WOW6432Node\Valve\Steam")
        .map_err(|e| format!("Registry key not found: {}", e))?;
    let path: String = key
        .get_value("InstallPath")
        .map_err(|e| format!("InstallPath not found: {}", e))?;
    if PathBuf::from(&path).exists() {
        Ok(path)
    } else {
        Err("Registry path does not exist on disk".to_string())
    }
}

#[cfg(target_os = "windows")]
fn detect_steam_from_registry_hkcu() -> Result<String, String> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu
        .open_subkey(r"SOFTWARE\Valve\Steam")
        .map_err(|e| format!("Registry key not found: {}", e))?;
    let path: String = key
        .get_value("SteamPath")
        .map_err(|e| format!("SteamPath not found: {}", e))?;
    if PathBuf::from(&path).exists() {
        Ok(path)
    } else {
        Err("Registry path does not exist on disk".to_string())
    }
}

pub fn check_workshop_mods_installed(
    workshop_ids: &[String],
) -> Result<WorkshopCheckReport, String> {
    let steam_path = detect_steam_path()?;
    Ok(check_workshop_mods_installed_at(
        workshop_ids,
        &steam_path,
    ))
}

fn check_workshop_mods_installed_at(
    workshop_ids: &[String],
    steam_path: &Path,
) -> WorkshopCheckReport {
    let library_paths = get_steam_library_paths(steam_path);
    let content_roots: Vec<PathBuf> = library_paths
        .iter()
        .map(|lib| {
            PathBuf::from(lib)
                .join("steamapps")
                .join("workshop")
                .join("content")
                .join(NOITA_APP_ID)
        })
        .filter(|path| path.is_dir())
        .collect();
    let content_roots_found = content_roots.len();
    let has_workshop_ids = workshop_ids
        .iter()
        .any(|id| !id.trim().is_empty() && id.trim() != "0");

    let diagnostic = if has_workshop_ids && content_roots.is_empty() {
        Some(format!(
            "No Noita workshop content folders found under {} Steam librar{}",
            library_paths.len(),
            if library_paths.len() == 1 { "y" } else { "ies" }
        ))
    } else {
        None
    };

    let statuses: Vec<(String, WorkshopInstallState)> = workshop_ids
        .iter()
        .map(|id| {
            let trimmed = id.trim();
            let state = if trimmed == "0" || trimmed.is_empty() {
                WorkshopInstallState::Installed
            } else if content_roots.is_empty() {
                WorkshopInstallState::Unknown
            } else if content_roots.iter().any(|root| root.join(trimmed).exists()) {
                WorkshopInstallState::Installed
            } else {
                WorkshopInstallState::Missing
            };
            (id.clone(), state)
        })
        .collect();

    WorkshopCheckReport {
        statuses,
        libraries_checked: library_paths,
        content_roots_found,
        diagnostic,
    }
}

fn get_steam_library_paths(steam_path: &Path) -> Vec<String> {
    let mut paths = vec![steam_path.to_string_lossy().to_string()];

    let vdf_path = steam_path
        .join("steamapps")
        .join("libraryfolders.vdf");

    if let Ok(content) = std::fs::read_to_string(&vdf_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("\"path\"") {
                let parts: Vec<&str> = trimmed.split('"').collect();
                if parts.len() >= 4 {
                    let lib_path = parts[3].replace("\\\\", "\\");
                    if PathBuf::from(&lib_path).exists() && !paths.contains(&lib_path) {
                        paths.push(lib_path);
                    }
                }
            }
        }
    }

    paths
}

pub fn open_steam_subscribe(workshop_id: &str) -> Result<(), String> {
    if workshop_id.is_empty() || workshop_id == "0" {
        return Err("No workshop ID provided".to_string());
    }
    let url = format!("steam://subscribe/{}", workshop_id);
    opener::open(&url).map_err(|e| format!("Failed to open Steam subscribe URL: {}", e))
}

/// Open a mod's workshop page, trying Steam client URI first, falling back to browser.
pub fn open_workshop_page(workshop_id: &str) {
    if workshop_id.is_empty() || workshop_id == "0" {
        return;
    }
    // Try Steam client URI first
    let steam_uri = format!("steam://url/CommunityFilePage/{}", workshop_id);
    if opener::open(&steam_uri).is_err() {
        // Fall back to browser
        let url = format!(
            "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
            workshop_id
        );
        let _ = opener::open(&url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::WorkshopInstallState;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "hallinta_{}_{}_{}",
                name,
                std::process::id(),
                nanos
            ));
            fs::create_dir_all(&path).expect("temp dir should be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).ok();
        }
    }

    fn write_libraryfolders(steam: &Path, libraries: &[&Path]) {
        let mut content = String::from("\"libraryfolders\"\n{\n");
        for (idx, library) in libraries.iter().enumerate() {
            let escaped = library.to_string_lossy().replace('\\', "\\\\");
            content.push_str(&format!(
                "\t\"{}\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n",
                idx, escaped
            ));
        }
        content.push_str("}\n");

        let steamapps = steam.join("steamapps");
        fs::create_dir_all(&steamapps).expect("steamapps dir should be created");
        fs::write(steamapps.join("libraryfolders.vdf"), content)
            .expect("libraryfolders.vdf should be written");
    }

    fn state_for(report: &crate::models::WorkshopCheckReport, id: &str) -> WorkshopInstallState {
        report
            .statuses
            .iter()
            .find(|(status_id, _)| status_id == id)
            .map(|(_, state)| *state)
            .expect("status should exist")
    }

    #[test]
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    fn test_detect_steam_path_unsupported_platform_is_err() {
        let result = detect_steam_path();
        assert!(
            result.is_err(),
            "unsupported platform must return Err for Steam detection"
        );
    }

    #[test]
    fn test_check_workshop_local_mods_always_installed() {
        // IDs "0" or "" represent local mods and are always reported installed,
        // even with a fake steam path that doesn't exist.
        let ids = vec!["0".to_string(), String::new()];
        let report = check_workshop_mods_installed_at(
            &ids,
            std::path::Path::new("/nonexistent/steam/path"),
        );
        for (id, state) in &report.statuses {
            assert!(
                matches!(state, WorkshopInstallState::Installed),
                "local mod '{}' should always be reported installed",
                id
            );
        }
    }

    #[test]
    fn workshop_root_absent_reports_unknown_instead_of_missing() {
        let ids = vec!["9999999999".to_string()];
        let steam = TempDir::new("workshop_root_absent");

        let report = check_workshop_mods_installed_at(&ids, steam.path());

        assert_eq!(report.content_roots_found, 0);
        assert_eq!(
            state_for(&report, "9999999999"),
            WorkshopInstallState::Unknown
        );
        assert!(
            report
                .diagnostic
                .as_deref()
                .is_some_and(|msg| msg.contains("No Noita workshop content folders found"))
        );
    }

    #[test]
    fn workshop_root_present_reports_missing_for_absent_id() {
        let ids = vec!["9999999999".to_string()];
        let steam = TempDir::new("workshop_root_present");
        fs::create_dir_all(
            steam
                .path()
                .join("steamapps")
                .join("workshop")
                .join("content")
                .join(NOITA_APP_ID),
        )
        .expect("workshop content root should be created");

        let report = check_workshop_mods_installed_at(&ids, steam.path());

        assert_eq!(report.content_roots_found, 1);
        assert_eq!(
            state_for(&report, "9999999999"),
            WorkshopInstallState::Missing
        );
    }

    #[test]
    fn workshop_scan_finds_mods_on_secondary_library() {
        let ids = vec!["123456".to_string()];
        let steam = TempDir::new("workshop_secondary_steam");
        let secondary = TempDir::new("workshop_secondary_library");
        write_libraryfolders(steam.path(), &[steam.path(), secondary.path()]);
        fs::create_dir_all(
            secondary
                .path()
                .join("steamapps")
                .join("workshop")
                .join("content")
                .join(NOITA_APP_ID)
                .join("123456"),
        )
        .expect("secondary workshop mod should be created");

        let report = check_workshop_mods_installed_at(&ids, steam.path());

        assert_eq!(report.content_roots_found, 1);
        assert_eq!(
            state_for(&report, "123456"),
            WorkshopInstallState::Installed
        );
    }

    #[test]
    fn test_open_steam_subscribe_empty_id_is_err() {
        assert!(open_steam_subscribe("").is_err());
        assert!(open_steam_subscribe("0").is_err());
    }

    #[test]
    fn test_noita_app_id_is_correct() {
        assert_eq!(NOITA_APP_ID, "881100");
    }
}
