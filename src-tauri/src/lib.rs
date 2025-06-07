// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
fn get_noita_config_path() -> String {
    if let Some(home_dir) = dirs::home_dir() {
        let local_low = home_dir.join("AppData").join("LocalLow");
        let noita_path = local_low.join("Nolla_Games_Noita").join("save00").join("mod_config.xml");
        noita_path.to_string_lossy().into_owned()
    } else {
        "C:\\Users\\Default\\AppData\\LocalLow\\Nolla_Games_Noita\\save00\\mod_config.xml".to_string()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, get_exe_dir, get_noita_config_path])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
