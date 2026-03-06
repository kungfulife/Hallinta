use std::path::PathBuf;

pub const NOITA_APP_ID: &str = "881100";

pub fn detect_steam_path() -> Result<PathBuf, String> {
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
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

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
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

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
    steam_path: &str,
) -> Result<Vec<(String, bool)>, String> {
    if steam_path.is_empty() {
        return Err("Steam path not configured".to_string());
    }

    let library_paths = get_steam_library_paths(steam_path);

    let results: Vec<(String, bool)> = workshop_ids
        .iter()
        .map(|id| {
            let installed = if id == "0" || id.is_empty() {
                true // Local mod
            } else {
                library_paths.iter().any(|lib| {
                    PathBuf::from(lib)
                        .join("steamapps")
                        .join("workshop")
                        .join("content")
                        .join(NOITA_APP_ID)
                        .join(id)
                        .exists()
                })
            };
            (id.clone(), installed)
        })
        .collect();

    Ok(results)
}

fn get_steam_library_paths(steam_path: &str) -> Vec<String> {
    let mut paths = vec![steam_path.to_string()];

    let vdf_path = PathBuf::from(steam_path)
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
