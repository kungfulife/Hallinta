use crate::models::UpdateInfo;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/kungfulife/Hallinta/releases/latest";
pub const WINDOWS_ASSET_NAME: &str = "Hallinta-x86_64-pc-windows-msvc.exe";

pub enum StartupMode {
    Normal {
        ready_path: Option<PathBuf>,
        monitor_resume: Option<crate::models::MonitorResume>,
        error_path: Option<PathBuf>,
    },
    Helper(HelperArgs),
}

pub struct HelperArgs {
    pub original: PathBuf,
    pub staging: PathBuf,
    pub rollback: PathBuf,
    pub old_pid: u32,
    pub old_creation_time: u64,
    pub expected_sha256: String,
    pub helper_ack_path: PathBuf,
    pub ready_path: PathBuf,
    pub handoff_path: PathBuf,
    pub monitor_resume: Option<crate::models::MonitorResume>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Could not initialize the update connection: {e}"))
}

pub fn check_latest(current: &str) -> Result<Option<UpdateInfo>, String> {
    let response = client()?
        .get(LATEST_RELEASE_URL)
        .header(USER_AGENT, format!("Hallinta/{current}"))
        .header(ACCEPT, "application/vnd.github+json")
        .send()
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    let release: Release = response
        .json()
        .map_err(|e| format!("GitHub returned an invalid release response: {e}"))?;
    release_to_update(release, current)
}

fn release_to_update(release: Release, current: &str) -> Result<Option<UpdateInfo>, String> {
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let current =
        Version::parse(current).map_err(|e| format!("This Hallinta version is invalid: {e}"))?;
    let version_text = release.tag_name.trim_start_matches(['v', 'V']);
    let version =
        Version::parse(version_text).map_err(|e| format!("GitHub release tag is invalid: {e}"))?;
    if version <= current || !version.pre.is_empty() {
        return Ok(None);
    }

    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == WINDOWS_ASSET_NAME)
        .ok_or_else(|| format!("Release v{version} does not contain {WINDOWS_ASSET_NAME}."))?;
    if asset.size == 0 {
        return Err(format!("Release v{version} contains an empty executable."));
    }
    if !asset
        .browser_download_url
        .starts_with("https://github.com/kungfulife/Hallinta/releases/download/")
    {
        return Err(format!(
            "Release v{version} contains an untrusted download URL."
        ));
    }
    let digest = asset
        .digest
        .as_deref()
        .and_then(|value| value.strip_prefix("sha256:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| format!("Release v{version} has no valid GitHub SHA-256 digest."))?;

    Ok(Some(UpdateInfo {
        version: version.to_string(),
        notes: release.body.unwrap_or_default(),
        download_url: asset.browser_download_url,
        asset_size: asset.size,
        sha256: digest.to_ascii_lowercase(),
    }))
}

pub fn download(
    info: &UpdateInfo,
    destination: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, u64),
) -> Result<(), String> {
    let mut response = client()?
        .get(&info.download_url)
        .header(
            USER_AGENT,
            format!("Hallinta/{}", crate::core::platform::get_version()),
        )
        .send()
        .map_err(network_error)?
        .error_for_status()
        .map_err(network_error)?;
    let expected_len = response.content_length().unwrap_or(info.asset_size);
    if info.asset_size > 0 && expected_len != info.asset_size {
        return Err("The release asset size does not match GitHub metadata.".to_string());
    }

    let parent = destination
        .parent()
        .ok_or_else(|| "The update staging path has no parent directory.".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create the update staging directory: {e}"))?;
    let mut output = File::create(destination)
        .map_err(|e| format!("Could not create the staged update: {e}"))?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];

    let result = (|| {
        loop {
            if cancel.load(Ordering::Acquire) {
                return Err("Update cancelled.".to_string());
            }
            let count = response
                .read(&mut buffer)
                .map_err(|e| format!("The update download was interrupted: {e}"))?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|e| format!("Could not write the staged update: {e}"))?;
            hasher.update(&buffer[..count]);
            downloaded += count as u64;
            on_progress(downloaded, info.asset_size.max(expected_len));
        }
        output
            .sync_all()
            .map_err(|e| format!("Could not flush the staged update: {e}"))?;
        if info.asset_size > 0 && downloaded != info.asset_size {
            return Err(format!(
                "The update download is incomplete ({downloaded} of {} bytes).",
                info.asset_size
            ));
        }
        let actual = hex_lower(&hasher.finalize());
        if actual != info.sha256 {
            return Err("The downloaded update failed SHA-256 verification.".to_string());
        }
        Ok(())
    })();

    if result.is_err() {
        drop(output);
        let _ = fs::remove_file(destination);
    }
    result
}

pub fn unique_paths(original: &Path) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let parent = original
        .parent()
        .ok_or_else(|| "The Hallinta executable has no parent directory.".to_string())?;
    let nonce = format!(
        "{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    );
    Ok((
        parent.join(format!(".hallinta-update-{nonce}.exe")),
        parent.join(format!(".hallinta-helper-{nonce}.exe")),
        parent.join(format!(".hallinta-rollback-{nonce}.exe")),
    ))
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex_lower(&hash.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn network_error(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "The update request timed out. Check your internet connection and retry.".to_string()
    } else if error.is_connect() {
        "Could not connect to GitHub. Check your internet connection and retry.".to_string()
    } else if let Some(status) = error.status() {
        format!("GitHub returned HTTP {status}. Please retry later.")
    } else {
        format!("The update request failed: {error}")
    }
}

pub fn startup_mode() -> Result<StartupMode, String> {
    let args: Vec<_> = std::env::args_os().collect();
    if args
        .get(1)
        .is_some_and(|arg| arg == "--hallinta-update-helper")
    {
        if args.len() != 13 {
            return Err("Invalid update helper arguments.".to_string());
        }
        return Ok(StartupMode::Helper(HelperArgs {
            original: PathBuf::from(&args[2]),
            staging: PathBuf::from(&args[3]),
            rollback: PathBuf::from(&args[4]),
            old_pid: args[5]
                .to_string_lossy()
                .parse()
                .map_err(|_| "Invalid parent process ID.".to_string())?,
            old_creation_time: args[6]
                .to_string_lossy()
                .parse()
                .map_err(|_| "Invalid parent process creation time.".to_string())?,
            expected_sha256: args[7].to_string_lossy().into_owned(),
            helper_ack_path: PathBuf::from(&args[8]),
            ready_path: PathBuf::from(&args[9]),
            handoff_path: PathBuf::from(&args[10]),
            monitor_resume: match (
                args[11].to_string_lossy().into_owned(),
                args[12].to_string_lossy().into_owned(),
            ) {
                (preset, session_id) if !preset.is_empty() && !session_id.is_empty() => {
                    Some(crate::models::MonitorResume {
                        preset_name: preset,
                        session_id,
                    })
                }
                _ => None,
            },
        }));
    }
    let (ready_path, monitor_resume, error_path) = if args
        .get(1)
        .is_some_and(|arg| arg == "--hallinta-update-ready")
    {
        if args.len() != 7 || args[3] != "--resume-monitor" {
            return Err("Invalid update readiness arguments.".to_string());
        }
        let preset = args[4].to_string_lossy().into_owned();
        let session_id = args[5].to_string_lossy().into_owned();
        if args[6] != "--update-launch" {
            return Err("Invalid update readiness marker.".to_string());
        }
        (
            Some(PathBuf::from(&args[2])),
            (!preset.is_empty() && !session_id.is_empty()).then_some(
                crate::models::MonitorResume {
                    preset_name: preset,
                    session_id,
                },
            ),
            None,
        )
    } else if args
        .get(1)
        .is_some_and(|arg| arg == "--hallinta-update-error")
    {
        if args.len() != 6 || args[3] != "--resume-monitor" {
            return Err("Invalid update error arguments.".to_string());
        }
        let preset_name = args[4].to_string_lossy().into_owned();
        let session_id = args[5].to_string_lossy().into_owned();
        (
            None,
            (!preset_name.is_empty() && !session_id.is_empty()).then_some(
                crate::models::MonitorResume {
                    preset_name,
                    session_id,
                },
            ),
            Some(PathBuf::from(&args[2])),
        )
    } else {
        (None, None, None)
    };
    Ok(StartupMode::Normal {
        ready_path,
        monitor_resume,
        error_path,
    })
}

#[cfg(windows)]
pub fn process_creation_time(pid: u32) -> Result<u64, String> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return Err(format!(
                "Could not inspect process {pid}: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        let ok = GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user);
        CloseHandle(handle);
        if ok == 0 {
            return Err(format!(
                "Could not read process creation time: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }
}

#[cfg(not(windows))]
pub fn process_creation_time(_pid: u32) -> Result<u64, String> {
    Err("Self-updates are supported only on Windows.".to_string())
}

pub fn wait_for_active_handoff(original: &Path) -> Result<(), String> {
    let parent = original
        .parent()
        .ok_or_else(|| "The Hallinta executable has no parent directory.".to_string())?;
    let original_text = original.to_string_lossy();
    let started = std::time::Instant::now();
    loop {
        let mut matching = Vec::new();
        let entries = fs::read_dir(parent)
            .map_err(|e| format!("Could not inspect the Hallinta directory for updates: {e}"))?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with(".hallinta-update-") || !name.ends_with(".handoff") {
                continue;
            }
            let content = fs::read_to_string(entry.path()).unwrap_or_default();
            let mut lines = content.lines();
            if !lines
                .next()
                .is_some_and(|path| path.eq_ignore_ascii_case(&original_text))
            {
                continue;
            }
            let helper_is_live = lines
                .next()
                .and_then(|pid| pid.parse::<u32>().ok())
                .zip(lines.next().and_then(|created| created.parse::<u64>().ok()))
                .is_some_and(|(pid, created)| process_creation_time(pid) == Ok(created));
            matching.push((entry.path(), helper_is_live));
        }
        if matching.is_empty() {
            return Ok(());
        }
        let any_live = matching.iter().any(|(_, live)| *live);
        let timeout = if any_live {
            Duration::from_secs(240)
        } else {
            Duration::from_secs(10)
        };
        if started.elapsed() >= timeout {
            // A dead helper must not permanently brick a portable installation.
            for (path, _) in matching {
                let _ = fs::remove_file(path);
            }
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn lock_with_timeout(
    lock: &named_lock::NamedLock,
    timeout: Duration,
    action: &str,
) -> Result<named_lock::NamedLockGuard, String> {
    let started = std::time::Instant::now();
    loop {
        match lock.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(_) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("Timed out {action}: {error}")),
        }
    }
}

#[cfg(windows)]
pub fn helper_creation_flags() -> Result<u32, String> {
    use windows_sys::Win32::System::JobObjects::{
        IsProcessInJob, JOB_OBJECT_LIMIT_BREAKAWAY_OK, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        QueryInformationJobObject,
    };
    use windows_sys::Win32::System::Threading::{CREATE_BREAKAWAY_FROM_JOB, GetCurrentProcess};
    unsafe {
        let mut in_job = 0;
        if IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut in_job) == 0 {
            return Err(format!(
                "Could not inspect the Windows job: {}",
                std::io::Error::last_os_error()
            ));
        }
        if in_job == 0 {
            return Ok(0);
        }
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        if QueryInformationJobObject(
            std::ptr::null_mut(),
            JobObjectExtendedLimitInformation,
            (&mut limits as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            std::mem::size_of_val(&limits) as u32,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(format!(
                "Could not inspect Windows job limits: {}",
                std::io::Error::last_os_error()
            ));
        }
        let flags = limits.BasicLimitInformation.LimitFlags;
        if flags & (JOB_OBJECT_LIMIT_BREAKAWAY_OK | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK) == 0 {
            return Err("Windows is preventing Hallinta from launching a safe update helper. Move Hallinta outside a managed sandbox and retry.".to_string());
        }
        Ok(CREATE_BREAKAWAY_FROM_JOB)
    }
}

#[cfg(windows)]
fn replace_file_with_rollback(
    replaced: &Path,
    replacement: &Path,
    backup: Option<&Path>,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let replaced_display = replaced.display().to_string();
    let replaced = wide(replaced);
    let replacement = wide(replacement);
    let backup = backup.map(wide);
    let backup_ptr = backup
        .as_ref()
        .map_or(std::ptr::null(), |path| path.as_ptr());
    if unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            backup_ptr,
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    } == 0
    {
        Err(format!(
            "Could not replace {}: {}",
            replaced_display,
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn schedule_delete_on_reboot(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_DELAY_UNTIL_REBOOT, MoveFileExW};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        MoveFileExW(wide.as_ptr(), std::ptr::null(), MOVEFILE_DELAY_UNTIL_REBOOT);
    }
}

#[cfg(not(windows))]
pub fn helper_creation_flags() -> Result<u32, String> {
    Err("Self-updates are supported only on Windows.".to_string())
}

#[cfg(windows)]
pub fn run_helper(args: HelperArgs) -> Result<(), String> {
    match run_helper_inner(&args) {
        Ok(()) => Ok(()),
        Err(error) => {
            let error_path = args.ready_path.with_extension("update-error.txt");
            let detail =
                format!("{error}\n\nThe existing executable was left in place or restored.");
            let _ = fs::write(&error_path, &detail);
            let _ = fs::remove_file(&args.staging);
            let _ = fs::remove_file(&args.helper_ack_path);
            let _ = fs::remove_file(&args.ready_path);
            let _ = fs::remove_file(&args.handoff_path);
            if args.original.exists() {
                let _ = std::process::Command::new(&args.original)
                    .arg("--hallinta-update-error")
                    .arg(&error_path)
                    .arg("--resume-monitor")
                    .arg(
                        args.monitor_resume
                            .as_ref()
                            .map_or("", |resume| resume.preset_name.as_str()),
                    )
                    .arg(
                        args.monitor_resume
                            .as_ref()
                            .map_or("", |resume| resume.session_id.as_str()),
                    )
                    .spawn();
            }
            if let Ok(helper) = std::env::current_exe() {
                schedule_delete_on_reboot(&helper);
            }
            Err(error)
        }
    }
}

#[cfg(windows)]
fn run_helper_inner(args: &HelperArgs) -> Result<(), String> {
    use named_lock::NamedLock;
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, WaitForSingleObject,
    };

    let helper_identity = format!(
        "{}:{}",
        std::process::id(),
        process_creation_time(std::process::id())?
    );
    fs::write(&args.helper_ack_path, helper_identity)
        .map_err(|e| format!("Could not acknowledge update helper startup: {e}"))?;

    unsafe {
        let parent = OpenProcess(
            0x0010_0000 | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            args.old_pid,
        );
        if !parent.is_null() {
            let created = process_creation_time(args.old_pid)?;
            if created != args.old_creation_time {
                CloseHandle(parent);
                return Err(
                    "The original Hallinta process identity changed; update aborted.".to_string(),
                );
            }
            if WaitForSingleObject(parent, 120_000) != WAIT_OBJECT_0 {
                CloseHandle(parent);
                return Err("Timed out waiting for Hallinta to close; update aborted.".to_string());
            }
            CloseHandle(parent);
        }
    }

    let lock = NamedLock::create("hallinta_noita")
        .map_err(|e| format!("Could not create the Hallinta update lock: {e}"))?;
    // The parent has exited, but another manually launched instance could race
    // the helper. Never replace the executable without owning the same lock.
    let guard = lock_with_timeout(
        &lock,
        Duration::from_secs(30),
        "acquiring the Hallinta update lock",
    )?;
    // Re-hash while holding the lock so the verified bytes cannot be swapped
    // between the UI's download check and ReplaceFileW.
    if sha256_file(&args.staging)? != args.expected_sha256 {
        return Err(
            "The staged executable changed before replacement; update aborted.".to_string(),
        );
    }
    replace_file_with_rollback(&args.original, &args.staging, Some(&args.rollback))?;
    drop(guard);

    let mut child = match std::process::Command::new(&args.original)
        .arg("--hallinta-update-ready")
        .arg(&args.ready_path)
        .arg("--resume-monitor")
        .arg(
            args.monitor_resume
                .as_ref()
                .map_or("", |resume| resume.preset_name.as_str()),
        )
        .arg(
            args.monitor_resume
                .as_ref()
                .map_or("", |resume| resume.session_id.as_str()),
        )
        .arg("--update-launch")
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return rollback_and_restart(args, None),
    };
    for _ in 0..300 {
        if args.ready_path.exists() {
            let _ = fs::remove_file(&args.ready_path);
            let _ = fs::remove_file(&args.helper_ack_path);
            let _ = fs::remove_file(&args.handoff_path);
            let _ = fs::remove_file(&args.rollback);
            if let Ok(helper) = std::env::current_exe() {
                schedule_delete_on_reboot(&helper);
            }
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("Could not inspect updated Hallinta: {e}"))?
        {
            return rollback_and_restart(args, Some(status.code().unwrap_or(1)));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let _ = child.kill();
    let _ = child.wait();
    rollback_and_restart(args, None)
}

#[cfg(windows)]
fn rollback_and_restart(args: &HelperArgs, exit_code: Option<i32>) -> Result<(), String> {
    let lock = named_lock::NamedLock::create("hallinta_noita")
        .map_err(|e| format!("Could not create rollback lock: {e}"))?;
    let _guard = lock_with_timeout(
        &lock,
        Duration::from_secs(30),
        "acquiring the rollback lock",
    )?;
    replace_file_with_rollback(&args.original, &args.rollback, None)
        .map_err(|error| format!("The update failed and rollback also failed: {error}"))?;
    Err(format!(
        "The updated Hallinta did not become ready{}; the previous version was restored.",
        exit_code.map_or(String::new(), |code| format!(" (exit code {code})"))
    ))
}

#[cfg(not(windows))]
pub fn run_helper(_args: HelperArgs) -> Result<(), String> {
    Err("Self-updates are supported only on Windows.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, name: &str, digest: Option<&str>) -> Release {
        Release {
            tag_name: tag.to_string(),
            body: Some("notes".to_string()),
            draft: false,
            prerelease: false,
            assets: vec![ReleaseAsset {
                name: name.to_string(),
                browser_download_url: format!(
                    "https://github.com/kungfulife/Hallinta/releases/download/{tag}/{WINDOWS_ASSET_NAME}"
                ),
                size: 42,
                digest: digest.map(str::to_string),
            }],
        }
    }

    #[test]
    fn accepts_only_newer_release_with_exact_asset_and_digest() {
        let info = release_to_update(
            release(
                "v0.8.1",
                WINDOWS_ASSET_NAME,
                Some(&format!("sha256:{}", "a".repeat(64))),
            ),
            "0.8.0",
        )
        .unwrap()
        .unwrap();
        assert_eq!(info.version, "0.8.1");
        assert_eq!(info.asset_size, 42);
    }

    #[test]
    fn rejects_downgrades_and_missing_digest() {
        assert!(
            release_to_update(
                release(
                    "v0.8.0",
                    WINDOWS_ASSET_NAME,
                    Some(&format!("sha256:{}", "a".repeat(64)))
                ),
                "0.8.0"
            )
            .unwrap()
            .is_none()
        );
        assert!(
            release_to_update(release("v0.8.1", WINDOWS_ASSET_NAME, None), "0.8.0")
                .unwrap_err()
                .contains("digest")
        );
    }

    #[test]
    fn rejects_untrusted_url_wrong_asset_and_nonstable_release() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let mut untrusted = release("v0.8.1", WINDOWS_ASSET_NAME, Some(&digest));
        untrusted.assets[0].browser_download_url =
            "https://attacker.invalid/Hallinta.exe".to_string();
        assert!(
            release_to_update(untrusted, "0.8.0")
                .unwrap_err()
                .contains("untrusted")
        );

        assert!(
            release_to_update(release("v0.8.1", "Hallinta.exe", Some(&digest)), "0.8.0")
                .unwrap_err()
                .contains(WINDOWS_ASSET_NAME)
        );

        let mut prerelease = release("v0.8.1-beta.1", WINDOWS_ASSET_NAME, Some(&digest));
        prerelease.prerelease = true;
        assert!(release_to_update(prerelease, "0.8.0").unwrap().is_none());
        let mut draft = release("v0.8.1", WINDOWS_ASSET_NAME, Some(&digest));
        draft.draft = true;
        assert!(release_to_update(draft, "0.8.0").unwrap().is_none());
    }

    #[cfg(windows)]
    #[test]
    fn windows_replace_keeps_a_rollback_copy() {
        let root = std::env::temp_dir().join(format!(
            "hallinta-replace-test-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&root).unwrap();
        let original = root.join("Hallinta.exe");
        let replacement = root.join("staged.exe");
        let rollback = root.join("rollback.exe");
        fs::write(&original, b"old bytes").unwrap();
        fs::write(&replacement, b"new bytes").unwrap();

        replace_file_with_rollback(&original, &replacement, Some(&rollback)).unwrap();

        assert_eq!(fs::read(&original).unwrap(), b"new bytes");
        assert_eq!(fs::read(&rollback).unwrap(), b"old bytes");
        assert!(!replacement.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
