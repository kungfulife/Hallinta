use crate::models::ModEntry;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub fn read_mod_config(directory: &Path) -> Result<String, String> {
    let config_path = directory.join("mod_config.xml");
    if !config_path.exists() {
        return Err("mod_config.xml not found in directory".to_string());
    }
    fs::read_to_string(config_path).map_err(|e| format!("Failed to read mod_config.xml: {}", e))
}

pub fn write_mod_config(directory: &Path, content: &str) -> Result<(), String> {
    let config_path = directory.join("mod_config.xml");
    fs::write(config_path, content).map_err(|e| format!("Failed to write mod_config.xml: {}", e))
}

pub fn parse_mods_from_xml(xml: &str) -> Result<Vec<ModEntry>, String> {
    let mut mods = Vec::new();

    for line in xml.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("<Mod ") {
            continue;
        }

        let name = extract_xml_attr(trimmed, "name").unwrap_or_else(|| "Unknown Mod".to_string());
        let enabled = extract_xml_attr(trimmed, "enabled").map_or(false, |v| v == "1");
        let workshop_id =
            extract_xml_attr(trimmed, "workshop_item_id").unwrap_or_else(|| "0".to_string());
        let settings_fold_open =
            extract_xml_attr(trimmed, "settings_fold_open").map_or(false, |v| v == "1");

        mods.push(ModEntry {
            name: unescape_xml_attr(&name),
            enabled,
            workshop_id,
            settings_fold_open,
        });
    }

    Ok(mods)
}

/// BUG-1 FIX: Properly escape XML attribute values when writing mod_config.xml.
pub fn mods_to_xml(mods: &[ModEntry]) -> String {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n");
    for m in mods {
        xml.push_str(&format!(
            "  <Mod name=\"{}\" enabled=\"{}\" workshop_item_id=\"{}\" settings_fold_open=\"{}\"></Mod>\n",
            escape_xml_attr(&m.name),
            if m.enabled { "1" } else { "0" },
            escape_xml_attr(&m.workshop_id),
            if m.settings_fold_open { "1" } else { "0" },
        ));
    }
    xml.push_str("</Mods>");
    xml
}

/// Escape characters that are invalid in XML attribute values.
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
}

/// Unescape XML attribute values when reading.
fn unescape_xml_attr(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&apos;", "'")
}

fn extract_xml_attr(tag: &str, attr_name: &str) -> Option<String> {
    let search = format!("{}=\"", attr_name);
    if let Some(start) = tag.find(&search) {
        let value_start = start + search.len();
        if let Some(end) = tag[value_start..].find('"') {
            return Some(tag[value_start..value_start + end].to_string());
        }
    }
    None
}

pub fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

pub fn write_file(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
}

pub fn check_file_exists(path: &Path) -> bool {
    path.exists()
}

pub fn get_file_modified_time(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Err("File does not exist".to_string());
    }
    let metadata = fs::metadata(path).map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let modified =
        metadata.modified().map_err(|e| format!("Failed to get modification time: {}", e))?;
    let secs = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| format!("Failed to convert time: {}", e))?
        .as_secs();
    Ok(secs)
}
