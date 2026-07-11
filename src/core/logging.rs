use crate::core::platform;
use crate::core::settings::get_data_dir;
use crate::models::LogSettings;
use chrono::{Local, Utc};
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex, Once};

const LOG_LEVEL_DEBUG: u8 = 10;
const LOG_LEVEL_INFO: u8 = 20;
const LOG_LEVEL_WARN: u8 = 30;
const LOG_LEVEL_ERROR: u8 = 40;
const DEFAULT_MAX_LOG_FILES: usize = 50;
const DEFAULT_MAX_LOG_BYTES: usize = 10 * 1_048_576;

#[derive(Default)]
struct WriterState {
    pending: VecDeque<String>,
    part: usize,
    file: Option<File>,
    bytes_written: u64,
}

static WRITER: LazyLock<Mutex<WriterState>> = LazyLock::new(|| Mutex::new(WriterState::default()));
static INSTANCE_ID: LazyLock<String> =
    LazyLock::new(|| Local::now().format("%Y%m%d_%H%M%S").to_string());
static SESSION_STARTED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK_INSTALLED: Once = Once::new();
static MIN_LOG_LEVEL: AtomicU8 = AtomicU8::new(LOG_LEVEL_INFO);
static MAX_LOG_FILES: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_LOG_FILES);
static MAX_LOG_BYTES: AtomicUsize = AtomicUsize::new(DEFAULT_MAX_LOG_BYTES);

fn log_file_name(version: &str, instance_id: &str, part: usize) -> String {
    let dev_tag = if cfg!(debug_assertions) { "_dev" } else { "" };
    if part == 0 {
        format!("hallinta_v{version}{dev_tag}_{instance_id}.log")
    } else {
        format!(
            "hallinta_v{version}{dev_tag}_{instance_id}_part{}.log",
            part + 1
        )
    }
}

fn log_level_rank(level: &str) -> u8 {
    match level.trim().to_uppercase().as_str() {
        "DEBUG" => LOG_LEVEL_DEBUG,
        "WARN" => LOG_LEVEL_WARN,
        "ERROR" => LOG_LEVEL_ERROR,
        _ => LOG_LEVEL_INFO,
    }
}

#[cfg(test)]
fn level_is_enabled(entry_level: &str, min_level: &str) -> bool {
    log_level_rank(entry_level) >= log_level_rank(min_level)
}

pub fn get_logs_dir() -> Result<PathBuf, String> {
    let logs_dir = get_data_dir()?.join("logs");
    std::fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create logs directory: {e}"))?;
    Ok(logs_dir)
}

pub fn get_current_log_file_path() -> Result<PathBuf, String> {
    let writer = WRITER
        .lock()
        .map_err(|e| format!("Failed to lock log writer: {e}"))?;
    Ok(get_logs_dir()?.join(log_file_name(
        &platform::get_version(),
        &INSTANCE_ID,
        writer.part,
    )))
}

fn marker_line(marker: &str) -> String {
    let build_mode = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "=== {} | Hallinta v{} ({}) [{}] | {} ===\n",
        marker,
        platform::get_version(),
        build_mode,
        platform::get_git_hash(),
        Utc::now().to_rfc3339()
    )
}

fn ensure_file(
    writer: &mut WriterState,
    logs_dir: &Path,
    version: &str,
    instance_id: &str,
) -> Result<(), String> {
    if writer.file.is_some() {
        return Ok(());
    }
    std::fs::create_dir_all(logs_dir)
        .map_err(|e| format!("Failed to create logs directory: {e}"))?;
    let path = logs_dir.join(log_file_name(version, instance_id, writer.part));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open log file {}: {e}", path.display()))?;
    writer.bytes_written = file.metadata().map(|m| m.len()).unwrap_or(0);
    writer.file = Some(file);
    Ok(())
}

fn write_pending(
    writer: &mut WriterState,
    logs_dir: &Path,
    max_bytes: usize,
) -> Result<(), String> {
    let version = platform::get_version();
    let instance_id = &*INSTANCE_ID;
    let max_bytes = max_bytes.max(1) as u64;

    while let Some(line) = writer.pending.front().cloned() {
        ensure_file(writer, logs_dir, &version, instance_id)?;
        let line_len = line.len() as u64;
        if writer.bytes_written > 0 && writer.bytes_written.saturating_add(line_len) > max_bytes {
            writer.file.take();
            writer.part += 1;
            writer.bytes_written = 0;
            continue;
        }

        if let Err(e) = writer
            .file
            .as_mut()
            .expect("log file initialized above")
            .write_all(line.as_bytes())
        {
            // Reopen and re-stat on the next attempt; a failed write may have
            // advanced the OS file cursor by an unknown partial amount.
            writer.file.take();
            writer.bytes_written = 0;
            return Err(format!("Failed to write log entry: {e}"));
        }
        writer.bytes_written = writer.bytes_written.saturating_add(line_len);
        writer.pending.pop_front();
    }

    if let Some(file) = writer.file.as_mut() {
        file.flush()
            .map_err(|e| format!("Failed to flush log file: {e}"))?;
    }
    Ok(())
}

fn enqueue_line(line: String) -> Result<(), String> {
    let logs_dir = get_logs_dir()?;
    let mut writer = WRITER
        .lock()
        .map_err(|e| format!("Failed to lock log writer: {e}"))?;
    writer.pending.push_back(line);
    write_pending(
        &mut writer,
        &logs_dir,
        MAX_LOG_BYTES.load(Ordering::Relaxed),
    )
}

fn enforce_log_retention(logs_dir: &Path, keep_count: usize) -> Result<(), String> {
    if !logs_dir.exists() {
        return Ok(());
    }
    let mut logs: Vec<_> = std::fs::read_dir(logs_dir)
        .map_err(|e| format!("Failed to read logs directory: {e}"))?
        .filter_map(Result::ok)
        .filter(|entry| {
            let path = entry.path();
            path.extension().is_some_and(|ext| ext == "log")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("hallinta_v"))
        })
        .collect();
    logs.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time
            .cmp(&a_time)
            .then_with(|| b.file_name().cmp(&a.file_name()))
    });
    for old_log in logs.into_iter().skip(keep_count.max(1)) {
        std::fs::remove_file(old_log.path())
            .map_err(|e| format!("Failed to remove old log {}: {e}", old_log.path().display()))?;
    }
    Ok(())
}

fn report_internal_error(context: &str, error: &str) {
    eprintln!("Hallinta logging error ({context}): {error}");
}

pub fn configure(settings: &LogSettings) {
    MIN_LOG_LEVEL.store(log_level_rank(&settings.log_level), Ordering::Relaxed);
    MAX_LOG_FILES.store(settings.max_log_files.clamp(1, 500), Ordering::Relaxed);
    MAX_LOG_BYTES.store(
        settings.max_log_size_mb.max(1).saturating_mul(1_048_576),
        Ordering::Relaxed,
    );
    if let Ok(logs_dir) = get_logs_dir()
        && let Err(e) = enforce_log_retention(&logs_dir, MAX_LOG_FILES.load(Ordering::Relaxed))
    {
        report_internal_error("retention", &e);
    }
}

pub fn init_log_session() {
    if SESSION_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Err(e) = enqueue_line(marker_line("SESSION BEGIN")) {
        report_internal_error("session start", &e);
    }
    if let Ok(logs_dir) = get_logs_dir()
        && let Err(e) = enforce_log_retention(&logs_dir, MAX_LOG_FILES.load(Ordering::Relaxed))
    {
        report_internal_error("retention", &e);
    }
}

pub fn write_session_end_marker() {
    if let Err(e) = enqueue_line(marker_line("SESSION END")) {
        report_internal_error("session end", &e);
    }
}

fn write_session_crash_marker() {
    if let Err(e) = enqueue_line(marker_line("SESSION CRASH")) {
        report_internal_error("crash marker", &e);
    }
}

fn panic_payload_to_string(panic_info: &PanicHookInfo<'_>) -> String {
    if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = panic_info.payload().downcast_ref::<String>() {
        return message.clone();
    }
    "Non-string panic payload".to_string()
}

fn log_panic_to_session(panic_info: &PanicHookInfo<'_>) {
    init_log_session();
    let thread_name = std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_string();
    let location = panic_info
        .location()
        .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
        .unwrap_or_else(|| "unknown location".to_string());
    let payload = panic_payload_to_string(panic_info);
    let backtrace = Backtrace::force_capture().to_string();

    let _ = log("ERROR", "Application panic detected", "CrashHandler");
    let _ = log(
        "ERROR",
        &format!("Panic payload: {payload}"),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic location: {location}"),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic thread: {thread_name}"),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic backtrace:\n{backtrace}"),
        "CrashHandler",
    );
    let _ = flush_log_buffer_sync();
    write_session_crash_marker();
}

pub fn install_panic_logging_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            log_panic_to_session(panic_info);
            previous_hook(panic_info);
        }));
    });
}

pub fn log(level: &str, message: &str, module: &str) -> Result<(), String> {
    let normalized_level = level.to_uppercase();
    if log_level_rank(&normalized_level) < MIN_LOG_LEVEL.load(Ordering::Relaxed) {
        return Ok(());
    }
    let line = format!(
        "[{}] [{}] [{}] {}\n",
        Utc::now().to_rfc3339(),
        normalized_level,
        module,
        message
    );
    let result = enqueue_line(line);
    if let Err(e) = &result {
        report_internal_error("write", e);
    }
    result
}

pub fn flush_log_buffer() -> Result<(), String> {
    let logs_dir = get_logs_dir()?;
    let mut writer = WRITER
        .lock()
        .map_err(|e| format!("Failed to lock log writer: {e}"))?;
    write_pending(
        &mut writer,
        &logs_dir,
        MAX_LOG_BYTES.load(Ordering::Relaxed),
    )
}

pub fn flush_log_buffer_sync() -> Result<(), String> {
    flush_log_buffer()
}

pub fn write_session_marker(marker: &str) {
    if let Err(e) = enqueue_line(marker_line(marker)) {
        report_internal_error("session marker", &e);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should follow epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hallinta-logging-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temp log directory should be created");
        path
    }

    #[test]
    fn debug_entries_are_filtered_at_info_level() {
        assert!(!super::level_is_enabled("DEBUG", "INFO"));
    }

    #[test]
    fn debug_entries_are_enabled_at_debug_level() {
        assert!(super::level_is_enabled("DEBUG", "DEBUG"));
    }

    #[test]
    fn failed_log_open_keeps_pending_lines() {
        let root = temp_dir("failure");
        let not_a_dir = root.join("not-a-directory");
        fs::write(&not_a_dir, b"file").expect("blocking file should write");
        let mut writer = super::WriterState::default();
        writer.pending.push_back("one\n".to_string());

        assert!(super::write_pending(&mut writer, &not_a_dir, 1024).is_err());
        assert_eq!(writer.pending.len(), 1);

        drop(writer);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn writer_rotates_before_size_limit() {
        let root = temp_dir("rotate");
        let mut writer = super::WriterState::default();
        writer.pending.push_back("12345\n".to_string());
        writer.pending.push_back("67890\n".to_string());

        super::write_pending(&mut writer, &root, 8).expect("pending lines should write");

        assert!(
            root.join(super::log_file_name(
                &crate::core::platform::get_version(),
                &super::INSTANCE_ID,
                0
            ))
            .exists()
        );
        assert!(
            root.join(super::log_file_name(
                &crate::core::platform::get_version(),
                &super::INSTANCE_ID,
                1
            ))
            .exists()
        );
        assert!(writer.pending.is_empty());
        drop(writer);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn writer_preserves_pending_line_order() {
        let root = temp_dir("order");
        let mut writer = super::WriterState::default();
        for line in ["first\n", "second\n", "third\n"] {
            writer.pending.push_back(line.to_string());
        }

        super::write_pending(&mut writer, &root, 1024).expect("pending lines should write");
        let path = root.join(super::log_file_name(
            &crate::core::platform::get_version(),
            &super::INSTANCE_ID,
            0,
        ));
        drop(writer);

        assert_eq!(
            fs::read_to_string(path).expect("log should read"),
            "first\nsecond\nthird\n"
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn retention_keeps_newest_log_files() {
        let root = temp_dir("retention");
        for name in [
            "hallinta_v0.8.0_1.log",
            "hallinta_v0.8.0_2.log",
            "hallinta_v0.8.0_3.log",
        ] {
            fs::write(root.join(name), name.as_bytes()).expect("fixture log should write");
        }

        super::enforce_log_retention(&root, 2).expect("retention should succeed");

        assert!(!root.join("hallinta_v0.8.0_1.log").exists());
        assert!(root.join("hallinta_v0.8.0_2.log").exists());
        assert!(root.join("hallinta_v0.8.0_3.log").exists());
        fs::remove_dir_all(root).ok();
    }
}
