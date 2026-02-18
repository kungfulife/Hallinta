PASTE NEWEST VERSION ENTRY ABOVE THIS LINE. Keep newest at top and push older entries downward.

# Update History

## 0.7.6
- Added Save Monitor Lockdown Mode:
  - blocks mod list/preset/gallery mutation actions while Save Monitor is running
  - suppresses mod-list-related logging during lockdown mode
  - supports startup in monitoring mode via new `Start in Save Monitoring Mode on launch` setting
  - compacts the main app layout and hides non-monitor controls while active
- In Settings, lockdown mode now hides non-monitor sections and keeps Save Monitor controls visible
- Added full-screen backup progress overlay with animated progress indicator for manual backup, restore, and scheduled auto-backup
- Added panic crash diagnostics logging (payload, location, thread, stack trace) and `SESSION CRASH` session marker
- Added log viewer actions to open the current session log file and logs directory (`Open File`, `Open Folder`)
- Regrouped log viewer controls and right-aligned fullscreen/separate-window/close actions
- Startup dark/light theme now applies immediately from persisted preference
- Added backend commands: `open_file`, `get_current_log_file_path`, `get_logs_directory`
- Updated application version to `0.7.6`

## 0.7.5
- Preset dropdown is now sorted alphabetically (Default always first)
- Catalog URL is now user-configurable in Settings > Preset Vault
- Added `?` help button with a guide for self-hosting a preset catalog
- Preset Vault shows a friendly message when no catalog URL is configured
- Settings now returns to Preset Vault view when opened from the vault
- Added "beta" badge to Import by Link and workshop detection note in missing mods modal
- Updated application version to `0.7.5`

## 0.7.4
- Consolidated scrollbar styling into shared `.themed-scrollbar` and `.themed-scrollbar-compact` utility classes in `base.css`, replacing three duplicated scrollbar blocks across `mod-list.css`, `settings.css`, and `components.css`
- Added consistent themed scrollbar styling to previously unstyled scrollable areas: gallery grid, log content (modal, fullscreen, and detached window), backup selection list, and missing mods list
- Removed dead CSS for native `<select>` elements that were replaced by `SelectEnhancer` (preset dropdown in `presets.css`, mod filter in `header.css`)
- Removed unused `@keyframes glow` and `.slide-in` animation from `animations.css`
- Merged duplicate `.ux-select-menu` rule blocks in `components.css` into a single block
- Removed unused `logAutoRefreshInterval` and `logFilters` properties from `state.js`
- Simplified `updateLogLevelSelectColor()` by removing dead inline color assignment on the hidden native select element
- Refactored `changeView()` in `UIManager` to extract repeated animation cleanup into shared `fadeInControls`/`fadeOutControls` helpers
- Fixed Escape key propagation in `SelectEnhancer`: pressing Escape now only stops event propagation when a dropdown is actually open, preventing unintended view/settings cancellation
- Updated application version to `0.7.4`

## 0.7.3
- Redesigned logging system: logging is now always active (cannot be disabled)
- Removed the `Enable application logging` toggle from Settings
- Log level dropdown now controls recording verbosity (what detail gets captured)
- Log file and session marker are now created immediately on startup for crash resilience
- Fixed status bar not showing messages when log level was set above Info
- Standardized default log level to Info for both development and release builds
- Updated application version to `0.7.3`

## 0.7.2
- Applied the same custom scrollbar styling used across the app to the Settings `System Information` and `Open Source Libraries` views
- Added an `Enable application logging` toggle in Settings
- Set application logging to default OFF for new and migrated settings
- Added a themed top bar for log headers (main-view style gradient/border/shadow) via new log-toolbar styling.
- Made log top buttons larger and more prominent (bigger height, padding, radius, font weight, hover lift).
- Applied this to:
  - In-app log modal header
  - In-app fullscreen log header
  - Detached log window header
- Added mobile wrapping behavior so toolbar controls stay usable on smaller widths.
- Updated application version to `0.7.2`

## 0.7.1
- Replaced the old Gallery header button with a tab-style view switcher (`Mod List` and `Preset Vault`)
- Refreshed settings layout into readable section cards with improved spacing and navigation flow
- Improved log-level hover/focus reactivity so the selector responds like interactive buttons
- Added explicit `DEBUG` logs when Escape keybind actions trigger (modals/panels/reorder/view exits)
- Set `Collect startup system details in logs` to default OFF and clarified non-technical helper copy
- Removed user-facing Catalog URL setting; catalog source is now developer-managed in code
- Added startup-time persistence for auto-detected Steam path when missing
- Fixed startup header spacing/clipping so the top-right Settings button remains fully visible
- Improved select UX polish in dark mode (log level + preset dropdown readability/feedback)
- Rebuilt Preset and Log Level dropdowns using a custom themed select component so list items render correctly in both light and dark mode
- Added compact scrollbar styling for the Preset dropdown list and main Mod List panel
- Tuned Log Level dropdown list behavior to stay non-scrolling while preserving per-option level coloring
- Applied the same styled scrollbar treatment to Settings and refined scrollbar visuals for a cleaner, more premium look
- Increased Preset dropdown width and aligned top-bar control heights for better visual consistency
- Slightly increased Mod List text size and widened its scrollbar for readability
- Updated application version to `0.7.1`

## 0.7.0
- Added Preset Gallery: browsable tab that fetches preset listings from a developer-maintained Google Drive catalog
- Added search and tag-based filtering for gallery presets
- Added one-click preset download with JSON validation and checksum verification
- Added "Import by Link" flow for downloading presets directly from Google Drive share URLs
- Added Workshop Mod Check: detects missing Steam Workshop mods when importing presets (gallery or local)
- Added missing mods modal with per-mod Steam Subscribe buttons (`steam://subscribe/{id}`)
- Added Steam path auto-detection via Windows registry with common-path fallback
- Added Gallery settings section (Catalog URL, Steam path with auto-detect button)
- Enhanced preset export format with SHA-256 checksum and updated version field (backward-compatible)
- Added checksum verification on preset import (warns on mismatch, does not block)
- Added `reqwest`, `sha2`, and `winreg` backend dependencies
- Added reqwest, sha2, and winreg to Open Source Libraries panel
- Updated Zip to 8.0.0

## 0.6.2
- Clarified System Information terminology: replaced "Target Triple" wording with "Build Target Platform"
- Expanded System Information details with runtime diagnostics (OS family, logical CPU cores, clock snapshot, app/exe directories, configured game paths)
- Added a new `Collect system details in startup logs` toggle under Application Settings
- Added startup diagnostics logging block (build/runtime/toolchain/path snapshot) when enabled
- Added Open Source Libraries panel with direct Cargo dependency credits and crate links
- Polished settings wording/UI labels for clarity (System Information button text and helper copy)

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
