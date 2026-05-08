use crate::core::settings::get_data_dir;
use crate::models::ModEntry;
use std::collections::BTreeMap;
use std::fs;

/// Validate a preset file for security and sanity.
pub fn validate_preset_file(content: &str) -> Result<(), String> {
    if content.len() > 1_048_576 {
        return Err("Preset file is too large (max 1 MB).".to_string());
    }

    let export: crate::models::PresetExportFile =
        serde_json::from_str(content).map_err(|e| format!("Invalid preset file format: {}", e))?;

    if export.hallinta_export != "presets" {
        return Err("Not a valid Hallinta preset file.".to_string());
    }

    if export.presets.len() > 100 {
        return Err("Too many presets (max 100).".to_string());
    }

    for (name, mods) in &export.presets {
        check_string_safe(name, "preset name")?;
        if mods.len() > 500 {
            return Err(format!("Preset \"{}\" has too many mods (max 500).", name));
        }
        for m in mods {
            check_string_safe(&m.name, "mod name")?;
            check_string_safe(&m.workshop_id, "workshop ID")?;
        }
    }

    Ok(())
}

fn check_string_safe(value: &str, field_name: &str) -> Result<(), String> {
    if value.contains('\0') {
        return Err(format!("{} contains null bytes.", field_name));
    }
    let lower = value.to_lowercase();
    if lower.contains("<script") || lower.contains("javascript:") || lower.contains("data:text") {
        return Err(format!("{} contains suspicious content.", field_name));
    }
    if value.contains("../") || value.contains("..\\") {
        return Err(format!("{} contains path traversal.", field_name));
    }
    Ok(())
}

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
