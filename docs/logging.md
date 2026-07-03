# Hallinta Logging

This document describes the current logging implementation in `src/core/logging.rs` and related lifecycle calls in `src/main.rs`, `src/app.rs`, and `src/app/*.rs`.

## Log Storage Location

Logs are stored under the OS-local Hallinta data directory in every build
profile:

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
3. During runtime, `log()` appends entries to an in-memory queue and flushes them to disk immediately.
4. On normal exit, `cleanup_on_exit()` in `src/app/lifecycle.rs` writes `APP_SHUTDOWN`, flushes synchronously, writes `SESSION END`, and flushes again.
5. On panic, the panic hook logs panic details, flushes synchronously, and writes `SESSION CRASH`.

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
- `RESTORE_START` / `RESTORE_START:auto=<filename>`
- `RESTORE_COMPLETE` / `RESTORE_FAILED`
- `SNAPSHOT_RESTORE_START`
- `MONITOR_START:preset=<preset>,session=<name>`
- `MONITOR_STOP:snapshots=<count>`

## What Gets Logged

The log is an audit trail for diagnosing crashes and tracking the state changes
that matter (mods enabled, presets switched, backups taken, saves restored).
Cosmetic UI knobs (filter, sort, theme, scale, compact mode, copy-to-clipboard,
keyboard-shortcut activations) are **not** logged — `settings.json` already
captures the relevant state.

The `module` field categorizes events. Key modules:
- `App` — startup, shutdown, close-prompt intent, initial preset / mod-count
  snapshot, exit-snapshot intent
- `Settings` — path changes (noita_dir, entangled_dir), save errors
- `ModManager` — single-mod toggle, bulk enable/disable, delete (with
  workshop_id), move-to-position, manual reload counts, import/export results
- `ModList` — drag committed (with name + position delta), drag cancelled by
  disruptive op, defensive aborts
- `PresetManager` — create, rename, delete, switch (with mod-count delta),
  import/export, refusals (e.g. delete Default)
- `Backup` — manual create / restore / delete with detail, upgrade-backup cleanup
- `SaveMonitor` — session start / pause, snapshot create / cleanup, interrupted-session reconcile
- `FileWatcher` — external `mod_config.xml` change detection
- `Workshop` — install check results (installed / missing counts)
- `CrashHandler` — panic payload, location, thread, backtrace
- `SystemInfo` — startup hardware/runtime snapshot (when enabled)

## Settings Integration (Current State)

`settings.json` includes:
- `max_log_files`
- `max_log_size_mb` (hardcoded to 10 MB, not user-adjustable)
- `log_level`
- `collect_system_info`

Current runtime behavior:
- `collect_system_info` is active and controls startup system-information logging.
- `log_level` filters entries before they enter the in-memory/file buffers.
- `max_log_files` is persisted but currently not enforced in `core::logging`.
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