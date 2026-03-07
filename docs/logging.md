# Hallinta Logging

This document describes the current logging implementation in `src/core/logging.rs` and related lifecycle calls in `src/main.rs` and `src/app.rs`.

## Log Storage Location

- Debug builds: `<repo>/dev_data/logs/`
- Release builds:
  - Windows: `%LOCALAPPDATA%\Hallinta\logs\`
  - Linux: `~/.local/share/Hallinta/logs/`

Location is resolved through `core::settings::get_data_dir()`.

## File Naming

Each app launch writes to a per-session log file:

`hallinta_v<version><_dev?>_<YYYYMMDD_HHMMSS>.log`

Examples:
- `hallinta_v0.8.0_dev_20260307_004210.log` (debug)
- `hallinta_v0.8.0_20260307_004210.log` (release)

## Log Formats

### Structured entry lines

`[<RFC3339 UTC timestamp>] [<LEVEL>] [<MODULE>] <message>`

Example:
`[2026-03-07T05:41:03.123456Z] [INFO] [App] Application started`

### Session marker lines

`=== <MARKER> | Hallinta v<version> (<debug|release>) | <RFC3339 UTC timestamp> ===`

Markers are written directly to the session file.

## Lifecycle and Flush Behavior

1. `main()` installs the panic hook via `install_panic_logging_hook()`.
2. `main()` starts the session via `init_log_session()` (`SESSION BEGIN` marker).
3. During runtime, `log()` appends entries to an in-memory queue.
4. `HallintaApp::check_timers()` calls `flush_log_buffer()` every 5 seconds.
5. On normal exit, `cleanup_on_exit()` writes `APP_SHUTDOWN`, flushes synchronously, writes `SESSION END`, and flushes again.
6. On panic, the panic hook logs panic details, flushes synchronously, and writes `SESSION CRASH`.

## In-Memory Buffers

- UI/event buffer (`LOG_BUFFER`) keeps up to `MAX_BUFFER_SIZE = 1000` entries.
- File buffer (`LOG_FILE_BUFFER`) queues entries waiting to be flushed to disk.

## Session Markers Currently Used

Core markers:
- `SESSION BEGIN`
- `SESSION END`
- `SESSION CRASH`

App markers:
- `APP_INITIALIZED`
- `APP_SHUTDOWN`
- `PRESET_SWITCH:<preset_name>`
- `BACKUP_START`
- `AUTO_BACKUP_START`
- `RESTORE_START`
- `MONITOR_START`
- `MONITOR_STOP`

## Settings Integration (Current State)

`settings.json` includes:
- `max_log_files`
- `max_log_size_mb`
- `log_level`
- `auto_save`
- `collect_system_info`

Current runtime behavior:
- `collect_system_info` is active and controls startup system-information logging.
- `max_log_files`, `max_log_size_mb`, `log_level`, and `auto_save` are persisted but currently not enforced in `core::logging`.

## Privacy Notes

Logs may include:
- Local filesystem paths
- Preset names
- Backup/snapshot filenames
- Optional system details (when enabled)

Hallinta currently does not upload logs or telemetry.

## Operational Tips

- Open the data directory from the app with `Settings > Open Settings Folder`.
- For crash investigations, include the most recent session log and keep marker lines intact.
