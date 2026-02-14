use crate::models::{OpenSourceLibrary, SystemInfo};
use crate::settings::get_data_dir;
use chrono::{Local, Utc};
use std::fs;
use std::path::{Path, PathBuf};
#[tauri::command]
pub(crate) fn is_dev_build() -> bool {
    cfg!(debug_assertions)
}

#[tauri::command]
pub(crate) fn get_dev_save_dir(source_noita_dir: String) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Not in dev mode".to_string());
    }

    let dev_data = get_data_dir()?;

    let config_path = dev_data.join("mod_config.xml");
    if !config_path.exists() {
        let source_config = PathBuf::from(&source_noita_dir).join("mod_config.xml");
        if !source_noita_dir.is_empty() && source_config.exists() {
            fs::copy(&source_config, &config_path)
                .map_err(|e| format!("Failed to copy mod_config.xml to dev_data: {}", e))?;
        } else {
            let sample_config =
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n</Mods>";
            fs::write(&config_path, sample_config)
                .map_err(|e| format!("Failed to create sample mod_config.xml: {}", e))?;
        }
    }

    Ok(dev_data.to_string_lossy().to_string())
}

#[tauri::command]
pub(crate) fn get_version() -> String {
    let context: tauri::Context<tauri_runtime_wry::Wry<tauri::EventLoopMessage>> =
        tauri::generate_context!();
    context.package_info().version.to_string()
}

#[tauri::command]
pub(crate) fn get_app_settings_dir() -> Result<String, String> {
    let data_dir = get_data_dir()?;
    Ok(data_dir.to_string_lossy().to_string())
}
#[tauri::command]
pub(crate) fn get_exe_dir() -> Result<String, String> {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(parent) = exe_path.parent() {
                Ok(parent.to_string_lossy().to_string())
            } else {
                Err("Could not get parent directory.".to_string())

            }
        }
        Err(e) => Err(format!("Could not get executable path: {}", e)),
    }
}

#[tauri::command]
pub(crate) fn get_noita_save_path() -> Result<String, String> {
    // Currently only supports Windows
    #[cfg(target_os = "windows")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let noita_path = home_dir
            .join("AppData")
            .join("LocalLow")
            .join("Nolla_Games_Noita")
            .join("save00");
        if noita_path.exists() {
            Ok(noita_path.to_string_lossy().to_string())
        } else {
            Err("Noita save directory not found".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Noita save path detection is only supported on Windows".to_string())
    }
}

#[tauri::command]
pub(crate) fn get_entangled_worlds_config_path() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {

        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let ew_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("quant")
            .join("entangledworlds");
        if ew_path.exists() {
            Ok(ew_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds config directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let config_path = home_dir.join(".config").join("entangledworlds");

        if config_path.exists() {
            Ok(config_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds config directory not found".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Entangled Worlds path detection is not supported on this platform".to_string())
    }
}

#[tauri::command]
pub(crate) fn get_entangled_worlds_save_path() -> Result<String, String> {

    #[cfg(target_os = "windows")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let ew_path = home_dir
            .join("AppData")
            .join("Roaming")
            .join("quant")
            .join("entangledworlds")
            .join("data");
        if ew_path.exists() {
            Ok(ew_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(target_os = "linux")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory.".to_string())?;
        let save_path = home_dir
            .join(".local")
            .join("share")
            .join("entangledworlds");
        if save_path.exists() {
            Ok(save_path.to_string_lossy().to_string())
        } else {
            Err("Entangled Worlds save directory not found".to_string())
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("Entangled Worlds save path detection is not supported on this platform".to_string())
    }
}

#[tauri::command]
pub(crate) async fn open_directory(directory: String) -> Result<(), String> {
    let path = Path::new(&directory);
    if !path.exists() {
        return Err("Directory does not exist".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&directory)
            .spawn()
            .map_err(|e| format!("Failed to open directory: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn open_workshop_item(workshop_id: String) -> Result<(), String> {
    if workshop_id == "0" || workshop_id.is_empty() {
        return Err("No workshop ID provided.".to_string());
    }

    let url = format!(
        "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
        workshop_id
    );
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/c", "start", &url])
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open workshop URL: {}", e))?;
    }

    Ok(())
}

// --- Session Lock ---

#[tauri::command]
pub(crate) fn get_system_info() -> Result<SystemInfo, String> {
    let executable_dir = get_exe_dir().unwrap_or_else(|_| "unknown".to_string());
    let app_data_dir = get_app_settings_dir().unwrap_or_else(|_| "unknown".to_string());
    let logical_cpu_cores = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(0);

    Ok(SystemInfo {
        app_version: get_version(),
        build_profile: env!("HALLINTA_PROFILE").to_string(),
        rust_version: env!("HALLINTA_RUSTC_VERSION").to_string(),
        cargo_version: env!("HALLINTA_CARGO_VERSION").to_string(),
        build_target: env!("HALLINTA_TARGET").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        os: std::env::consts::OS.to_string(),
        os_family: std::env::consts::FAMILY.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        logical_cpu_cores,
        local_time: Local::now().to_rfc3339(),
        utc_time: Utc::now().to_rfc3339(),
        executable_dir,
        app_data_dir,
    })
}

#[tauri::command]
pub(crate) fn get_open_source_libraries() -> Vec<OpenSourceLibrary> {
    let libraries = [
        (
            "chrono",
            "0.4.43",
            "Date/time handling",
            "https://crates.io/crates/chrono",
        ),
        (
            "dirs",
            "6.0.0",
            "Platform directories lookup",
            "https://crates.io/crates/dirs",
        ),
        (
            "named-lock",
            "0.4.1",
            "Single-instance process lock",
            "https://crates.io/crates/named-lock",
        ),
        (
            "serde",
            "1.0.228",
            "Serialization framework",
            "https://crates.io/crates/serde",
        ),
        (
            "serde_json",
            "1.0.149",
            "JSON serialization/deserialization",
            "https://crates.io/crates/serde_json",
        ),
        (
            "tauri",
            "2.10.2",
            "Desktop app framework",
            "https://crates.io/crates/tauri",
        ),
        (
            "tauri-build",
            "2.5.5",
            "Tauri build tooling",
            "https://crates.io/crates/tauri-build",
        ),
        (
            "tauri-plugin-dialog",
            "2.6.0",
            "Native dialog integration",
            "https://crates.io/crates/tauri-plugin-dialog",
        ),
        (
            "tauri-plugin-opener",
            "2.5.3",
            "Open files/links with system handlers",
            "https://crates.io/crates/tauri-plugin-opener",
        ),
        (
            "tauri-plugin-shell",
            "2.3.5",
            "Shell/process integration",
            "https://crates.io/crates/tauri-plugin-shell",
        ),
        (
            "tauri-runtime-wry",
            "2.10.0",
            "Tauri runtime backend",
            "https://crates.io/crates/tauri-runtime-wry",
        ),
        (
            "tokio",
            "1.49.0",
            "Async runtime",
            "https://crates.io/crates/tokio",
        ),
        (
            "walkdir",
            "2",
            "Recursive directory traversal",
            "https://crates.io/crates/walkdir",
        ),
        (
            "zip",
            "7.4.0",
            "ZIP read/write support",
            "https://crates.io/crates/zip",
        ),
    ];

    libraries
        .into_iter()
        .map(|(name, version, purpose, homepage)| OpenSourceLibrary {
            name: name.to_string(),
            version: version.to_string(),
            purpose: purpose.to_string(),
            homepage: homepage.to_string(),
        })
        .collect()
}
