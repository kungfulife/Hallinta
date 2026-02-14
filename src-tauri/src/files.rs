use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tokio::fs as tokio_fs;
#[tauri::command]
pub(crate) fn read_mod_config(directory: String) -> Result<String, String> {
    let config_path = PathBuf::from(directory).join("mod_config.xml");
    if !config_path.exists() {
        return Err("mod_config.xml not found in directory.".to_string());
    }

    fs::read_to_string(config_path).map_err(|e| format!("Failed to read mod_config.xml: {}", e))
}

#[tauri::command]
pub(crate) fn write_mod_config(directory: String, content: String) -> Result<(), String> {
    let config_path = PathBuf::from(directory).join("mod_config.xml");
    fs::write(config_path, content).map_err(|e| format!("Failed to write mod_config.xml: {}", e))
}

#[tauri::command]
pub(crate) fn write_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write to file: {}", e))
}

#[tauri::command]
pub(crate) fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}


#[tauri::command]
pub(crate) fn check_file_exists(path: String) -> Result<bool, String> {
    let path = Path::new(&path);
    Ok(path.exists())
}

#[tauri::command]
pub(crate) async fn check_file_modified(file_path: String, last_modified: u64) -> Result<bool, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Ok(false);
    }

    let metadata = tokio_fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;
    let current_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();

    Ok(current_time > last_modified)
}

#[tauri::command]
pub(crate) async fn get_file_modified_time(file_path: String) -> Result<u64, String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err("File does not exist".to_string());
    }

    let metadata = tokio_fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let modified = metadata
        .modified()
        .map_err(|e| format!("Failed to get modification time: {}", e))?;
    let current_time = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();

    Ok(current_time)
}


