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

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_mods_from_xml ────────────────────────────────────────────

    #[test]
    fn test_parse_empty_mods_tag() {
        let xml = "<Mods>\n</Mods>";
        let mods = parse_mods_from_xml(xml).unwrap();
        assert!(mods.is_empty());
    }

    /// Verifies the real Noita mod_config.xml format (attribute-order-independent).
    #[test]
    fn test_parse_real_noita_format() {
        let xml = r#"<Mods>
  <Mod enabled="0" name="bruh" settings_fold_open="0" workshop_item_id="2362171854" >
  </Mod>
  <Mod enabled="1" name="test_mod" settings_fold_open="1" workshop_item_id="0" >
  </Mod>
</Mods>"#;
        let mods = parse_mods_from_xml(xml).unwrap();
        assert_eq!(mods.len(), 2);

        assert_eq!(mods[0].name, "bruh");
        assert!(!mods[0].enabled);
        assert_eq!(mods[0].workshop_id, "2362171854");
        assert!(!mods[0].settings_fold_open);

        assert_eq!(mods[1].name, "test_mod");
        assert!(mods[1].enabled);
        assert_eq!(mods[1].workshop_id, "0");
        assert!(mods[1].settings_fold_open);
    }

    #[test]
    fn test_parse_mod_name_with_spaces_and_special_chars() {
        let xml = r#"<Mods>
  <Mod enabled="0" name="New Biomes + Secrets" settings_fold_open="0" workshop_item_id="1985575640" >
  </Mod>
</Mods>"#;
        let mods = parse_mods_from_xml(xml).unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].name, "New Biomes + Secrets");
    }

    #[test]
    fn test_parse_ignores_non_mod_lines() {
        let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Mods>\n</Mods>";
        let mods = parse_mods_from_xml(xml).unwrap();
        assert!(mods.is_empty());
    }

    #[test]
    fn test_parse_missing_workshop_id_defaults_to_zero() {
        // Some older entries might omit workshop_item_id entirely.
        let xml = r#"<Mods>
  <Mod enabled="1" name="local_mod" settings_fold_open="0" >
  </Mod>
</Mods>"#;
        let mods = parse_mods_from_xml(xml).unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(mods[0].workshop_id, "0");
    }

    // ── XML escape/unescape round-trip ────────────────────────────────

    #[test]
    fn test_mods_to_xml_roundtrip() {
        let original = vec![
            ModEntry {
                name: "my & mod".to_string(),
                enabled: true,
                workshop_id: "123".to_string(),
                settings_fold_open: false,
            },
            ModEntry {
                name: "another".to_string(),
                enabled: false,
                workshop_id: "0".to_string(),
                settings_fold_open: true,
            },
        ];

        let xml = mods_to_xml(&original);
        let parsed = parse_mods_from_xml(&xml).unwrap();

        assert_eq!(parsed.len(), original.len());
        for (a, b) in original.iter().zip(parsed.iter()) {
            assert_eq!(a.name, b.name, "name mismatch after round-trip");
            assert_eq!(a.enabled, b.enabled);
            assert_eq!(a.workshop_id, b.workshop_id);
            assert_eq!(a.settings_fold_open, b.settings_fold_open);
        }
    }

    #[test]
    fn test_xml_special_char_escaping() {
        let mods = vec![ModEntry {
            name: "mod\"with<special>&chars'".to_string(),
            enabled: true,
            workshop_id: "0".to_string(),
            settings_fold_open: false,
        }];
        let xml = mods_to_xml(&mods);
        // Raw special chars must not appear unescaped in the output
        assert!(!xml.contains("mod\"with<special>&chars'"));
        // Round-trip must recover original name
        let parsed = parse_mods_from_xml(&xml).unwrap();
        assert_eq!(parsed[0].name, mods[0].name);
    }

    // ── mods_equal ────────────────────────────────────────────────────

    #[test]
    fn test_write_and_read_mod_config() {
        let dir = std::env::temp_dir().join("hallinta_test_mods");
        std::fs::create_dir_all(&dir).unwrap();

        let mods = vec![
            ModEntry {
                name: "alpha".to_string(),
                enabled: true,
                workshop_id: "111".to_string(),
                settings_fold_open: false,
            },
            ModEntry {
                name: "beta".to_string(),
                enabled: false,
                workshop_id: "0".to_string(),
                settings_fold_open: false,
            },
        ];

        write_mod_config(&dir, &mods_to_xml(&mods)).unwrap();
        let xml = read_mod_config(&dir).unwrap();
        let parsed = parse_mods_from_xml(&xml).unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "alpha");
        assert!(parsed[0].enabled);
        assert_eq!(parsed[1].name, "beta");
        assert!(!parsed[1].enabled);

        std::fs::remove_dir_all(&dir).ok();
    }
}
