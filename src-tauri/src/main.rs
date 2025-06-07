use tauri::command;
use std::path::PathBuf;
use dirs::home_dir;

#[command]
fn get_noita_config_path() -> String {
    if let Some(profile_dir) = home_dir() {
        let local_low = profile_dir.join("AppData").join("LocalLow");
        let noita_path = local_low.join("Nolla_Games_Noita").join("save00").join("mod_config.xml");
        noita_path.to_string_lossy().into_owned()
    } else {
        "C:\\Users\\Default\\AppData\\LocalLow\\Nolla_Games_Noita\\save00\\mod_config.xml".to_string()
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_noita_config_path])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}