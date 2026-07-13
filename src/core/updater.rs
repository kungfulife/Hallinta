use crate::models::UpdateInfo;
use self_update::backends::github::{ReleaseList, Update};
use self_update::update::Release;
use semver::Version;

const REPO_OWNER: &str = "kungfulife";
const REPO_NAME: &str = "Hallinta";
pub const REPOSITORY_URL: &str = "https://github.com/kungfulife/Hallinta";
const TARGET: &str = "x86_64-pc-windows-msvc";
pub const UPDATE_ASSET_NAME: &str = "Hallinta-x86_64-pc-windows-msvc.zip";
const VERIFYING_KEY: [u8; 32] = *include_bytes!("../assets/hallinta-update.pub");

pub fn release_url(version: &str) -> String {
    let version = version.strip_prefix('v').unwrap_or(version);
    format!("{REPOSITORY_URL}/releases/tag/v{version}")
}

pub fn check_latest(current: &str) -> Result<Option<UpdateInfo>, String> {
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .map_err(display_error)?
        .fetch()
        .map_err(display_error)?;
    select_latest(releases, current)
}

fn select_latest(releases: Vec<Release>, current: &str) -> Result<Option<UpdateInfo>, String> {
    let current = Version::parse(current)
        .map_err(|error| format!("This Hallinta version is invalid: {error}"))?;

    Ok(releases
        .into_iter()
        .filter_map(|release| {
            let version = Version::parse(&release.version).ok()?;
            let has_exact_asset = release
                .assets
                .iter()
                .any(|asset| asset.name == UPDATE_ASSET_NAME);
            (version > current && version.pre.is_empty() && has_exact_asset)
                .then_some((version, release))
        })
        .max_by(|left, right| left.0.cmp(&right.0))
        .map(|(version, release)| UpdateInfo {
            version: version.to_string(),
            notes: release.body.unwrap_or_default(),
        }))
}

pub fn install(version: &str) -> Result<(), String> {
    let tag = format!("v{version}");
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .target(TARGET)
        .identifier(UPDATE_ASSET_NAME)
        .bin_name("Hallinta")
        .bin_path_in_archive("Hallinta.exe")
        .current_version(self_update::cargo_crate_version!())
        .target_version_tag(&tag)
        .show_output(false)
        .show_download_progress(false)
        .no_confirm(true)
        .verifying_keys([VERIFYING_KEY])
        .build()
        .map_err(display_error)?
        .update()
        .map_err(display_error)?;

    match status {
        self_update::Status::Updated(installed) if installed == version => Ok(()),
        self_update::Status::Updated(installed) => Err(format!(
            "The updater installed Hallinta v{installed}, but v{version} was selected."
        )),
        self_update::Status::UpToDate(installed) => Err(format!(
            "The updater reported Hallinta v{installed} is already installed."
        )),
    }
}

fn display_error(error: impl std::fmt::Display) -> String {
    format!("Update failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use self_update::update::ReleaseAsset;

    fn release(version: &str, asset: &str) -> Release {
        Release {
            name: format!("Hallinta {version}"),
            version: version.to_string(),
            date: "2026-07-11".to_string(),
            body: Some("release notes".to_string()),
            assets: vec![ReleaseAsset {
                name: asset.to_string(),
                download_url: "https://api.github.com/assets/1".to_string(),
            }],
        }
    }

    #[test]
    fn selects_highest_stable_newer_release_with_exact_asset() {
        let releases = vec![
            release("0.8.3", UPDATE_ASSET_NAME),
            release("0.9.0-beta.1", UPDATE_ASSET_NAME),
            release("0.8.4", UPDATE_ASSET_NAME),
        ];
        let selected = select_latest(releases, "0.8.2").unwrap().unwrap();
        assert_eq!(selected.version, "0.8.4");
        assert_eq!(selected.notes, "release notes");
    }

    #[test]
    fn skips_wrong_asset_equal_version_and_invalid_versions() {
        let releases = vec![
            release("0.8.3", "Hallinta.exe"),
            release("0.8.2", UPDATE_ASSET_NAME),
            release("nightly", UPDATE_ASSET_NAME),
        ];
        assert!(select_latest(releases, "0.8.2").unwrap().is_none());
    }

    #[test]
    fn builds_repository_release_url_from_version() {
        assert_eq!(
            release_url("0.9.0"),
            "https://github.com/kungfulife/Hallinta/releases/tag/v0.9.0"
        );
    }

    #[test]
    fn release_url_does_not_duplicate_existing_v_prefix() {
        assert_eq!(
            release_url("v0.9.0"),
            "https://github.com/kungfulife/Hallinta/releases/tag/v0.9.0"
        );
    }
}
