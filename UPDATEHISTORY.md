PASTE NEWEST VERSION ENTRY ABOVE THIS LINE. Keep newest at top and push older entries downward.

# Update History

## 0.6.1
- Fixed detached log window preventing application from closing fully (now handled at Tauri level)
- Fixed log level select in settings not showing color for the selected value
- Removed Data Directory from System Info panel (already viewable in settings)
- Added descriptive helper text for retention settings (backup auto-delete, upgrade backups, save monitor snapshots)
- Added "(0 = keep all)" and "(0 = off)" hints to backup settings inputs

## 0.6.0
- Added Save Monitor: start/stop save snapshot system with configurable interval and per-preset organization
- Added Entangled Worlds inclusion flow for manual backups, Save Monitor, and upgrade backups
- Expanded upgrade preflight backup to include save00, save01, and Entangled Worlds data (blocks on failure)
- Added restore component selection for Entangled Worlds data in backup restore UI
- Added mandatory close-game warning in restore flow
- Added lifecycle logging: explicit startup/ready/closing events with session begin/end markers in log files
- Fixed shutdown to always close detached log window on app exit
- Added All/Enabled/Disabled mod filter dropdown alongside search bar
- Improved log viewer readability with local time rendering (UTC storage preserved)
- Hardened Save & Close directory-change path with confirmation prompt
- Improved drag reliability: reduced delay, better mouse vs touch distinction
- Added Save Monitor settings in settings page (snapshot interval, max snapshots per preset)
- Polished System Info close button styling

## 0.5.6
- Added `dev` tag in log filenames for dev builds (example: `hallinta_v0.5.6_dev_<instance>.log`)
- Fixed mod-item hold behavior where chosen row could visually disappear before real drag movement
- Tuned drag/click interaction thresholds to reduce accidental drag-start on click and missed single-click toggles
- Kept drag position-number sync behavior for fallback dragged element during reorder preview

## 0.5.5
- Fixed main mod-list drag preview so the dragged item's number badge updates live to the predicted drop position
- Fixed Sortable fallback clone behavior by explicitly syncing `.sortable-fallback .mod-number` during `onMove`
- Added/kept inline comments in reorder logic to document why fallback clone syncing is required
- Improved drag target clarity with candidate highlight feedback while hovering nearby rows
- Removed the temporary reorder sandbox window/UI after findings were integrated into main mod-list behavior

## 0.5.3
- Improved drag-and-drop reorder UX by removing confusing visible source ghost behavior during drag
- Added `Esc` cancel during active drag to revert mod order back to the exact pre-drag snapshot
- Prevented reorder persistence on canceled drags and no-op drags (no accidental save when nothing changed)
- Fixed dev-mode mod save path mismatch so toggles/reorders persist correctly across restarts
- Improved startup preset/mod_config synchronization behavior and added clearer startup diagnostics in logs
- Note: Mod List move/reorder UI is still being worked on

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
