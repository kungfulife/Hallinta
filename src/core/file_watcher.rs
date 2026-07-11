use crate::core::mods;
use std::path::Path;

/// Check if mod_config.xml has been modified externally since `last_modified_time`.
/// Returns `Some(new_mtime)` if modified, `None` if unchanged, or the metadata error.
pub fn check_for_external_changes(
    directory: &Path,
    last_modified_time: u64,
) -> Result<Option<u64>, String> {
    let config_path = directory.join("mod_config.xml");
    let current_time = mods::get_file_modified_time(&config_path)?;
    Ok((current_time > last_modified_time).then_some(current_time))
}
