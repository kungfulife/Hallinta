use crate::app::get_version;
use crate::models::LogEntry;
use crate::settings::get_data_dir;
use chrono::{Local, Utc};
use std::backtrace::Backtrace;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic::PanicHookInfo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::sync::Once;
use tokio::fs as tokio_fs;

pub(crate) static LOG_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
pub(crate) static LOG_FILE_BUFFER: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
pub(crate) static MAX_BUFFER_SIZE: usize = 1000;
pub(crate) static INSTANCE_ID: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
});
static SESSION_STARTED: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK_INSTALLED: Once = Once::new();

/// Eagerly creates the log file and writes the SESSION BEGIN marker on startup.
pub(crate) fn init_log_session() {
    if SESSION_STARTED.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let _ = std::fs::create_dir_all(&logs_dir);
        let version = get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker(&mut file, "SESSION BEGIN");
        }
    }
}

fn log_file_name(version: &str, instance_id: &str) -> String {
    // Mark dev-build logs clearly so they are easy to separate from release logs.
    let dev_tag = if cfg!(debug_assertions) { "_dev" } else { "" };
    format!("hallinta_v{}{}_{}.log", version, dev_tag, instance_id)
}

fn write_session_marker(file: &mut std::fs::File, marker: &str) {
    let version = get_version();
    let build_mode = if cfg!(debug_assertions) { "debug" } else { "release" };
    let timestamp = Utc::now().to_rfc3339();
    let line = format!(
        "=== {} | Hallinta v{} ({}) | {} ===\n",
        marker, version, build_mode, timestamp
    );
    let _ = file.write_all(line.as_bytes());
}

pub(crate) fn write_session_end_marker() {
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let version = get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker(&mut file, "SESSION END");
        }
    }
}

pub(crate) fn write_session_crash_marker() {
    if let Ok(data_dir) = get_data_dir() {
        let logs_dir = data_dir.join("logs");
        let version = get_version();
        let instance_id = &*INSTANCE_ID;
        let log_file = logs_dir.join(log_file_name(&version, instance_id));
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
            write_session_marker(&mut file, "SESSION CRASH");
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

    let _ = add_log_entry(
        "ERROR".to_string(),
        "Application panic detected".to_string(),
        "CrashHandler".to_string(),
    );
    let _ = add_log_entry(
        "ERROR".to_string(),
        format!("Panic payload: {}", payload),
        "CrashHandler".to_string(),
    );
    let _ = add_log_entry(
        "ERROR".to_string(),
        format!("Panic location: {}", location),
        "CrashHandler".to_string(),
    );
    let _ = add_log_entry(
        "ERROR".to_string(),
        format!("Panic thread: {}", thread_name),
        "CrashHandler".to_string(),
    );
    let _ = add_log_entry(
        "ERROR".to_string(),
        format!("Panic backtrace:\n{}", backtrace),
        "CrashHandler".to_string(),
    );
    let _ = flush_log_buffer_sync();
    write_session_crash_marker();
}

pub(crate) fn install_panic_logging_hook() {
    PANIC_HOOK_INSTALLED.call_once(|| {
        let previous_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            log_panic_to_session(panic_info);
            previous_hook(panic_info);
        }));
    });
}

#[tauri::command]
pub(crate) fn add_log_entry(level: String, message: String, module: String) -> Result<(), String> {
    let normalized_level = level.to_uppercase();
    let timestamp = Utc::now().to_rfc3339();
    let entry = LogEntry {
        timestamp,
        level: normalized_level.clone(),
        message: message.clone(),
        module,
    };
    let mut buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    if buffer.len() >= MAX_BUFFER_SIZE {
        buffer.pop_front();
    }
    buffer.push_back(entry.clone());
    let mut file_buffer = LOG_FILE_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
    file_buffer.push_back(entry);

    Ok(())
}

#[tauri::command]
pub(crate) fn get_log_entries() -> Result<Vec<LogEntry>, String> {
    let buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    Ok(buffer.iter().cloned().collect())
}

#[tauri::command]
pub(crate) fn clear_log_buffer() -> Result<(), String> {
    let mut buffer = LOG_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock log buffer: {}", e))?;
    let mut file_buffer = LOG_FILE_BUFFER
        .lock()
        .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
    buffer.clear();
    file_buffer.clear();
    Ok(())
}

#[tauri::command]
pub(crate) async fn flush_log_buffer() -> Result<(), String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    if !logs_dir.exists() {
        tokio_fs::create_dir_all(&logs_dir)
            .await
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    let logs = {
        let mut file_buffer = LOG_FILE_BUFFER
            .lock()
            .map_err(|e| format!("Failed to lock file log buffer: {}", e))?;
        if file_buffer.is_empty() {
            return Ok(());
        }
        file_buffer.drain(..).collect::<Vec<_>>()
    };

    let version = get_version();
    let instance_id = &*INSTANCE_ID;
    let log_file = logs_dir.join(log_file_name(&version, instance_id));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file {}: {}", log_file.display(), e))?;

    for entry in logs {
        let log_line = format!(
            "[{}] [{}] [{}] {}\n",
            entry.timestamp, entry.level, entry.module, entry.message
        );
        file.write_all(log_line.as_bytes()).map_err(|e| {
            format!("Failed to write to log file {}: {}", log_file.display(), e)
        })?;
    }

    Ok(())
}

pub(crate) fn flush_log_buffer_sync() -> Result<(), String> {
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

    let version = get_version();
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

#[tauri::command]
pub(crate) fn get_logs_directory() -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }
    Ok(logs_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub(crate) fn get_current_log_file_path() -> Result<String, String> {
    let data_dir = get_data_dir()?;
    let logs_dir = data_dir.join("logs");
    if !logs_dir.exists() {
        std::fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    let version = get_version();
    let instance_id = &*INSTANCE_ID;
    let log_file = logs_dir.join(log_file_name(&version, instance_id));

    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("Failed to open log file {}: {}", log_file.display(), e))?;

    Ok(log_file.to_string_lossy().to_string())
}
