use crate::core::mods;
use std::path::Path;

/// Check if mod_config.xml has been modified externally since `last_modified_time`.
/// Returns `Some(new_mtime)` if modified, `None` if unchanged.
pub fn check_for_external_changes(directory: &Path, last_modified_time: u64) -> Option<u64> {
    let config_path = directory.join("mod_config.xml");
    if let Ok(current_time) = mods::get_file_modified_time(&config_path) {
        if current_time > last_modified_time {
            return Some(current_time);
        }
    }
    None
}
