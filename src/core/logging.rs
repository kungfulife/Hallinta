use crate::core::platform;
use crate::core::settings::get_data_dir;
use crate::models::{LogEntry, LogSettings};
use chrono::{Local, Utc};
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Mutex, Once};

static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
static LOG_FILE_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
const MAX_BUFFER_SIZE: usize = 1000;
static INSTANCE_ID: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| Local::now().format("%Y%m%d_%H%M%S").to_string());
static SESSION_STARTED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK_INSTALLED: Once = Once::new();
static MIN_LOG_LEVEL: AtomicU8 = AtomicU8::new(LOG_LEVEL_INFO);

const LOG_LEVEL_DEBUG: u8 = 10;
const LOG_LEVEL_INFO: u8 = 20;
const LOG_LEVEL_WARN: u8 = 30;
const LOG_LEVEL_ERROR: u8 = 40;

fn log_file_name(version: &str, instance_id: &str) -> String {
    let dev_tag = if cfg!(debug_assertions) { "_dev" } else { "" };
    format!("hallinta_v{}{}_{}.log", version, dev_tag, instance_id)
}

fn write_session_marker_to_file(file: &mut std::fs::File, marker: &str) {
    let version = platform::get_version();
    let git_hash = platform::get_git_hash();
    let build_mode = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let timestamp = Utc::now().to_rfc3339();
    let line = format!(
        "=== {} | Hallinta v{} ({}) [{}] | {} ===\n",
        marker, version, build_mode, git_hash, timestamp
    );
    let _ = file.write_all(line.as_bytes());
}

pub fn configure(settings: &LogSettings) {
    MIN_LOG_LEVEL.store(log_level_rank(&settings.log_level), Ordering::Relaxed);
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

/// Create the log file and write the SESSION BEGIN marker.
pub fn init_log_session() {
    if SESSION_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let _ = std::fs::create_dir_all(&logs_dir);
        let version = platform::get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker_to_file(&mut file, "SESSION BEGIN");
        }
    }
}

pub fn write_session_end_marker() {
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let version = platform::get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker_to_file(&mut file, "SESSION END");
        }
    }
}

fn write_session_crash_marker() {
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let version = platform::get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker_to_file(&mut file, "SESSION CRASH");
        }
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
        &format!("Panic payload: {}", payload),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic location: {}", location),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic thread: {}", thread_name),
        "CrashHandler",
    );
    let _ = log(
        "ERROR",
        &format!("Panic backtrace:\n{}", backtrace),
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
    let timestamp = Utc::now().to_rfc3339();
    let entry = LogEntry {
        timestamp,
        level: normalized_level,
        message: message.to_string(),
        module: module.to_string(),
    };

    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        if buffer.len() >= MAX_BUFFER_SIZE {
            buffer.pop_front();
        }
        buffer.push_back(entry.clone());
    }
    if let Ok(mut file_buffer) = LOG_FILE_BUFFER.lock() {
        file_buffer.push_back(entry);
    }
    // Write through: logs hit disk when emitted, not on a timer.
    let _ = flush_log_buffer();
    Ok(())
}

pub fn flush_log_buffer() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let logs = {
        let mut file_buffer = LOG_FILE_BUFFER
            .lock()
            .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
        if file_buffer.is_empty() {
            return Ok(());
        }
        file_buffer.drain(..).collect::<Vec<_>>()
    };

    let version = platform::get_version();
    let instance_id = &*INSTANCE_ID;
    let log_file = logs_dir.join(log_file_name(&version, instance_id));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    for entry in logs {
        let log_line = format!(
            "[{}] [{}] [{}] {}\n",
            entry.timestamp, entry.level, entry.module, entry.message
        );
        file.write_all(log_line.as_bytes())
            .map_err(|e| format!("Failed to write to log file: {}", e))?;
    }

    Ok(())
}

pub fn flush_log_buffer_sync() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let logs = {
        if let Ok(mut file_buffer) = LOG_FILE_BUFFER.lock() {
            file_buffer.drain(..).collect::<Vec<_>>()
        } else {
            Vec::new()
        }
    };

    if logs.is_empty() {
        return Ok(());
    }

    let version = platform::get_version();
    let instance_id = &*INSTANCE_ID;
    let log_file = logs_dir.join(log_file_name(&version, instance_id));
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
        for entry in logs {
            let log_line = format!(
                "[{}] [{}] [{}] {}\n",
                entry.timestamp, entry.level, entry.module, entry.message
            );
            let _ = file.write_all(log_line.as_bytes());
        }
    }

    Ok(())
}

pub fn write_session_marker(marker: &str) {
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let version = platform::get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker_to_file(&mut file, marker);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn debug_entries_are_filtered_at_info_level() {
        assert!(!super::level_is_enabled("DEBUG", "INFO"));
    }

    #[test]
    fn debug_entries_are_enabled_at_debug_level() {
        assert!(super::level_is_enabled("DEBUG", "DEBUG"));
    }
}
