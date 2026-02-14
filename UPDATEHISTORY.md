PASTE NEWEST VERSION ENTRY ABOVE THIS LINE. Keep newest at top and push older entries downward.

# Update History

## 0.5.0
- Split backend `lib.rs` into modules: `app`, `settings`, `logging`, `files`, `session`, `backup`, `models`
- Added compile-time build metadata in `build.rs` and new `get_system_info` Tauri command
- Added System Info panel and button in Settings UI
- Added shared `src/js/logUtils.js` and refactored log rendering/filter logic to reuse it
- Extracted preset save helper into `buildPresetsForSave` in `src/js/state.js`
- UI/CSS touch-ups:
  - DEV log color updated to soft purple (`#b39ddb`)
  - DEBUG select option color made more readable
  - Main title size/spacing adjusted
  - Removed redundant log message color rules and duplicate sidebar separator definition

## 0.4.8
- Added DEV log level (ordinal `-1`) with distinct color
- DEV logs always bypass log level filter
- DEV logs no longer update status bar (log viewer only)
- Updated log level ordering across event handlers, UI manager, and detached log window

## 0.4.7
- Log level dropdown labels simplified to: `DEBUG`, `INFO`, `WARN`, `ERROR`

## 0.4.6
- Replaced 4 log filter checkboxes with hierarchy dropdown in all log view modes
- Added search highlighting using `<mark class="log-highlight">`
- Preset creation switched from `prompt()` to `showInputModal()`

## 0.4.5
- Log viewer improvements: modal/fullscreen/separate window, smart scroll, detached sticky mode
- Checklist modals standardized with `.modal-content-checklist`
- Backup settings UI improvements and upgrade-backup auto-cleanup (keep 5)
- Dev mode path UX updates and startup/settings behavior fixes

## 0.4.0
- Session lock
- Dev mode sync
- Backup/restore system
- Preset import/export
- Log viewer foundation
