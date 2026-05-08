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
- `APP_INITIALIZED:v<version>`
- `APP_SHUTDOWN`
- `PRESET_SWITCH:<preset_name>`
- `BACKUP_START`
- `BACKUP_OK:<filename>`
- `BACKUP_FAILED`
- `AUTO_BACKUP_START:interval=<N>min`
- `RESTORE_START` / `RESTORE_START:auto=<filename>`
- `RESTORE_COMPLETE` / `RESTORE_FAILED`
- `SNAPSHOT_RESTORE_START`
- `MONITOR_START:preset=<preset>,session=<name>`
- `MONITOR_STOP:snapshots=<count>`

## Modules Currently Logged

The `module` field categorizes events. Key modules:
- `App` — startup, shutdown, close-prompt intent, initial state snapshot
- `Settings` — theme/scale/window-mode/path changes, save errors
- `ModManager` — toggle, bulk enable/disable, delete, move, drag commit, reload, import/export
- `ModList` — drag start/cancel/commit, abort-on-disruptive-op
- `PresetManager` — create, rename, delete, switch, import, export
- `Backup` — create/restore/delete, auto-cleanup, auto-backup
- `SaveMonitor` — session start/pause/end, snapshot create, snapshot cleanup
- `FileWatcher` — external `mod_config.xml` change detection
- `Workshop` — workshop install check results, page/URL/ID copy actions
- `UI` — keyboard shortcut activations, filter/sort changes
- `CrashHandler` — panic payload, location, thread, backtrace
- `DevData` — dev sandbox seed/restore (debug builds only)

## Settings Integration (Current State)

`settings.json` includes:
- `max_log_files`
- `max_log_size_mb` (hardcoded to 10 MB, not user-adjustable)
- `log_level`
- `auto_save`
- `collect_system_info`

Current runtime behavior:
- `collect_system_info` is active and controls startup system-information logging.
- `max_log_files`, `log_level`, and `auto_save` are persisted but currently not enforced in `core::logging`.
- `max_log_size_mb` is kept at 10 MB default and not exposed in the settings UI.

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
