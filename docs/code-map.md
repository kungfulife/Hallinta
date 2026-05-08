# Hallinta Code Map

Fast navigation notes for agents and maintainers.

## First Stops

- `src/main.rs`: process bootstrap, panic hook, log session start, single-instance lock, eframe launch.
- `src/app.rs`: `HallintaApp` state shape and constructor. It wires startup loading, dev sandbox seeding, initial UI scale/theme, and async channels.
- `src/tasks.rs`: async result enum sent back to the UI thread.
- `src/models.rs`: shared data models, settings structs, UI enums, modal/action payloads.

## App Behavior Modules

`src/app/` contains split `impl HallintaApp` blocks. Start here when tracing user workflows:

- `actions.rs`: preset switching, mod config writes, settings reactions, filter/sort state, active save paths, open `mod_config.xml`.
- `async_tasks.rs`: fire-and-forget task dispatchers for backups, monitor session lists, snapshots, workshop checks, and data clearing.
- `backup_actions.rs`: backup and restore entry points shown from UI controls.
- `import_export.rs`: mod list import/export and preset import/export preparation.
- `input.rs`: keyboard shortcuts, close-request handling, bulk enable/disable.
- `lifecycle.rs`: `eframe::App` update loop and shutdown cleanup.
- `modal_actions.rs`: confirm/input/checklist/missing-mod modal action handlers.
- `monitor.rs`: save monitor session lifecycle and snapshot scheduling.
- `sorting.rs`: reusable mod sort helper.
- `task_results.rs`: UI-thread handling for completed async work.
- `timers.rs`: periodic log flushing, file-watch polling, backup cleanup, auto-backup, save-monitor scans.

## Domain Logic

- `src/core/mods.rs`: `mod_config.xml` read/write/parse/serialize.
- `src/core/presets.rs`: preset JSON load/save/validation.
- `src/core/backup.rs`: backup archive creation, restore, delete, content inspection.
- `src/core/save_monitor.rs`: monitor sessions and snapshot file management.
- `src/core/platform.rs`: OS paths, Noita/Steam/Entangled detection, dev sandbox paths.
- `src/core/settings.rs`: settings load/save, app data directory, version upgrade checks.
- `src/core/logging.rs`: session logs, markers, panic-flush support.
- `src/core/workshop.rs`: Steam Workshop install checks.
- `src/core/file_watcher.rs`: `mod_config.xml` mtime checks.

## UI Modules

- `src/ui/header.rs`: top navigation, search, preset controls, global buttons.
- `src/ui/sidebar.rs`: backup/restore/monitor/settings actions outside compact mode.
- `src/ui/mod_list.rs`: main mod list, filters, sorting controls, monitor-active view.
- `src/ui/compact.rs`: compact monitor-focused layout.
- `src/ui/settings.rs`: settings view and path/appearance/backup/logging controls.
- `src/ui/modals.rs`: modal rendering; behavior lives in `src/app/modal_actions.rs`.
- `src/ui/context_menu.rs`: mod row context actions.
- `src/ui/design.rs`, `src/ui/theme.rs`: spacing, sizing, zoom, colors.

## Trace Patterns

- UI event -> `src/ui/*` renderer -> `HallintaApp` method in `src/app/*`.
- Long-running work -> `src/app/async_tasks.rs` or inline spawn -> `TaskResult` -> `src/app/task_results.rs`.
- File-system behavior -> prefer `src/core/*`; app modules should mainly orchestrate and update UI state.
- Debug data paths -> `HallintaApp::get_active_noita_dir()` / `get_active_entangled_dir()` in `src/app/actions.rs`; do not bypass them from app-level workflows.

## Docs Nearby

- `docs/dev-mode.md`: debug sandbox behavior and safety rails.
- `docs/logging.md`: log files, session markers, flush lifecycle.
- `docs/design-system.md`: visual spacing/color conventions.
- `docs/egui.md`: egui implementation notes and UI pitfalls.
