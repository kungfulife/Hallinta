use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub noita_dir: String,
    pub entangled_dir: String,
    pub dark_mode: bool,
}


#[derive(Serialize, Deserialize, Clone)]
pub struct ModPreset {
    pub name: String,
    pub enabled: bool,
    pub workshop_id: String,
    pub settings_fold_open: bool,
}

#[tauri::command]
fn read_mod_config(directory: String) -> Result<String, String> {
    let config_path = std::path::PathBuf::from(directory).join("mod_config.xml");

    if !config_path.exists() {
        return Err("mod_config.xml not found in directory".to_string());
    }

    std::fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read mod_config.xml: {}", e))
}

#[tauri::command]
fn write_mod_config(directory: String, content: String) -> Result<(), String> {
    let config_path = std::path::PathBuf::from(directory).join("mod_config.xml");

    std::fs::write(config_path, content)
        .map_err(|e| format!("Failed to write mod_config.xml: {}", e))
}


#[tauri::command]
fn get_exe_dir() -> Result<String, String> {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(parent) = exe_path.parent() {
                Ok(parent.to_string_lossy().to_string())
            } else {
                Err("Could not get parent directory".to_string())
            }
        }
        Err(e) => Err(format!("Could not get executable path: {}", e))
    }
}

#[tauri::command]
fn get_noita_save_path() -> String {
    if let Some(home_dir) = dirs::home_dir() {
        let local_low = home_dir.join("AppData").join("LocalLow");
        let noita_path = local_low.join("Nolla_Games_Noita").join("save00");
        noita_path.to_string_lossy().into_owned()
    } else {
        "C:\\Users\\facky\\AppData\\LocalLow\\Nolla_Games_Noita\\save00".to_string()
    }
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let exe_dir = get_exe_dir()?;
    let settings_path = PathBuf::from(exe_dir).join("settings.json");

    let json_content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(settings_path, json_content)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    Ok(())
}

#[tauri::command]
fn load_settings() -> Result<AppSettings, String> {
    let exe_dir = get_exe_dir()?;
    let settings_path = PathBuf::from(exe_dir).join("settings.json");

    if !settings_path.exists() {
        return Ok(AppSettings {
            noita_dir: get_noita_save_path(),
            entangled_dir: String::new(),
            dark_mode: false,
        });
    }

    let content = std::fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings: {}", e))?;

    Ok(settings)
}

#[tauri::command]
fn save_presets(presets: std::collections::HashMap<String, Vec<ModPreset>>) -> Result<(), String> {
    let exe_dir = get_exe_dir()?;
    let presets_path = PathBuf::from(exe_dir).join("presets.json");

    let json_content = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;

    std::fs::write(presets_path, json_content)
        .map_err(|e| format!("Failed to write presets file: {}", e))?;

    Ok(())
}

#[tauri::command]
fn load_presets() -> Result<std::collections::HashMap<String, Vec<ModPreset>>, String> {
    let exe_dir = get_exe_dir()?;
    let presets_path = PathBuf::from(exe_dir).join("presets.json");

    if !presets_path.exists() {
        let mut default_presets = std::collections::HashMap::new();
        default_presets.insert("Default".to_string(), Vec::new());
        return Ok(default_presets);
    }

    let content = std::fs::read_to_string(presets_path)
        .map_err(|e| format!("Failed to read presets file: {}", e))?;

    let presets: std::collections::HashMap<String, Vec<ModPreset>> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse presets: {}", e))?;

    Ok(presets)
}

#[tauri::command]
fn create_backup_folder() -> Result<String, String> {
    let exe_dir = get_exe_dir()?;
    let backup_path = PathBuf::from(exe_dir).join("backups");

    std::fs::create_dir_all(&backup_path)
        .map_err(|e| format!("Failed to create backup folder: {}", e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
fn open_workshop_item(workshop_id: String) -> Result<(), String> {
    if workshop_id == "0" || workshop_id.is_empty() {
        return Err("No workshop ID provided".to_string());
    }

    let url = format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", workshop_id);

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", &url])
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

#[tauri::command]
fn export_mod_list(mods: Vec<ModPreset>, preset_name: String) -> Result<String, String> {
    let export_data = serde_json::json!({
        "preset_name": preset_name,
        "mods": mods,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "app_version": "1.0.0"
    });

    let json_content = serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize mod list: {}", e))?;

    Ok(json_content)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            get_exe_dir,
            get_noita_save_path,
            open_workshop_item,
            save_settings,
            load_settings,
            save_presets,
            load_presets,
            create_backup_folder,
            export_mod_list,
            read_mod_config,
            write_mod_config
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
