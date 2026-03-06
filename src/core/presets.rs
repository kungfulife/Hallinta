use crate::core::settings::get_data_dir;
use crate::models::ModEntry;
use std::collections::BTreeMap;
use std::fs;

pub fn load_presets() -> Result<BTreeMap<String, Vec<ModEntry>>, String> {
    let data_dir = get_data_dir()?;
    let presets_path = data_dir.join("presets.json");
    if !presets_path.exists() {
        let mut default_presets = BTreeMap::new();
        default_presets.insert("Default".to_string(), Vec::new());
        save_presets(&default_presets)?;
        return Ok(default_presets);
    }

    let content = fs::read_to_string(&presets_path)
        .map_err(|e| format!("Failed to read presets file: {}", e))?;
    let presets: BTreeMap<String, Vec<ModEntry>> =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse presets: {}", e))?;
    Ok(presets)
}

pub fn save_presets(presets: &BTreeMap<String, Vec<ModEntry>>) -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let presets_path = data_dir.join("presets.json");

    let json_content = serde_json::to_string_pretty(presets)
        .map_err(|e| format!("Failed to serialize presets: {}", e))?;
    fs::write(presets_path, json_content)
        .map_err(|e| format!("Failed to write presets file: {}", e))?;
    Ok(())
}
