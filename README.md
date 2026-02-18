# Hallinta - Noita Mod Manager

A mod manager for Noita with preset support, backup/restore, and a structured logging system. Built with Rust + Tauri.

## Features

### Mod Management
- Load and manage mods from Noita's mod_config.xml
- Toggle mods on/off with visual indicators
- Drag-and-drop reordering with live updates
- Workshop and local mod support
- Real-time file monitoring for external changes

### Preset System
- Create, rename, and delete mod presets
- Quick switching between different mod configurations
- Import and export presets as JSON files with checksum verification
- Automatic preset synchronization with mod_config.xml
- Alphabetical preset sorting (Default always first)
- Conflict resolution when external changes detected

### Preset Vault
- Browse and download presets from a configurable catalog URL
- Search and tag-based filtering
- One-click download with JSON validation and checksum verification
- Import presets directly from Google Drive share links *(in development)*
- Workshop mod check: detects missing Steam Workshop mods on import *(in development)*
- Per-mod Steam Subscribe buttons for quick installation
- In-app guide for self-hosting a preset catalog

### Backup & Restore
- Manual and automatic backups of Noita save data (save00, save01) and presets
- Save monitoring with per-preset snapshots for crash recovery
- Save Monitor Lockdown Mode (monitoring-focused UI that blocks mod/preset mutations while active)
- Configurable auto-backup interval
- Auto-deletion of old backups (configurable retention period)
- Selective restore with per-component options
- Upgrade backups created automatically on version change (keeps last 5)

### Entangled Worlds
- Optional support for [Noita Entangled Worlds](https://github.com/IntQuant/noita_entangled_worlds) multiplayer mod directories
- Include entangled save data in backups for crash recovery
- Auto-detection on Windows and Linux

### Application Logs
- Always-on session logging for reliable crash diagnostics
- Panic crash handler now writes panic payload, location, thread, and stack trace to session logs
- Three viewing modes: modal overlay, fullscreen panel, separate OS window
- Level-based filtering (Debug, Info, Warn, Error)
- Search within log entries
- Auto-refresh with smart scroll (preserves position when reading history)
- Color-coded log levels with left-border indicators
- Copy filtered logs to clipboard
- Open current session log file or logs folder directly from the log viewer

### Smart Directory Detection
- **Noita Save Directory**: Automatically finds Windows save location
- **Entangled Worlds** (optional): Detects multiplayer mod directories on Windows and Linux
- Manual directory selection with Browse and Find Default buttons

### User Interface
- Dark and light mode
- Context menus for mod operations (toggle, delete, reorder, workshop links)
- Search and filter functionality
- Theme-safe custom dropdowns for Presets and Log Level (consistent light/dark list-item rendering)
- Responsive layout
- Status bar with application info

### Settings & Configuration
- Persistent settings stored in system data directories
- User-configurable preset catalog URL
- Configurable log level controls recording verbosity
- Optional startup system diagnostics collection (disabled by default)
- Backup scheduling and retention settings
- Steam path auto-detection during initial setup
- Version upgrade detection with automatic backups
- Settings validation and error recovery
- System Information and Open Source Libraries credits panels

## Setup

### First Run
1. Launch Hallinta
2. Application automatically detects Noita save directory (Windows)
3. If not found, use Find Default or Browse in Settings
4. Entangled Worlds directory detection is optional

### Directory Configuration
- **Settings > Noita Saved Data Directory**: Required for core functionality
- **Settings > Entangled Worlds Directory**: Optional multiplayer support
- Use Find Default to auto-locate standard installations
- Browse for custom installations

## Interface Overview

- **Header**: `Mod List` / `Preset Vault` tabs, search bar, preset controls, settings access
- **Mod List View**: Main mod list with drag-and-drop reordering
- **Preset Vault**: Browse, search, and download presets from a configured catalog
- **Settings**: Directory configuration, appearance, backup, logging, catalog URL, Steam path
- **Status Bar**: Click for application logs

## Technical Details

- **Backend**: Rust with Tauri 2
- **Frontend**: Vanilla JavaScript (ES6 modules, no bundler)
- **Data Storage**: JSON files in platform data directories
- **File Monitoring**: Real-time mod_config.xml watching
- **Logging**: Structured logging with daily file rotation and in-app viewer

## Planned

- Linux/macOS directory detection improvements
- Enhanced Entangled Worlds multiplayer mod support
- Further Workshop integration and Google Drive link testing

## Building

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build
```

## Latest Version

Current version: **0.7.6**

Latest update highlights:
- Added Save Monitor Lockdown Mode:
  - while Save Monitor runs, mod/preset/list mutation actions are blocked
  - non-monitor controls are hidden and the main UI compacts into a monitoring-focused layout
  - app can auto-start directly in monitoring mode via a new Save Monitor setting
- Added log viewer actions to open the current log file and open the logs directory
- Regrouped log viewer toolbar controls for better clarity, with fullscreen/separate-window/close actions aligned to the right
- Added panic crash logging with payload, location, thread, stack trace, and `SESSION CRASH` marker
- Backup/restore operations now show a full-screen progress overlay with animated progress feedback
- Startup theme is now applied immediately from persisted preference to prevent wrong-mode flash
- Updated application version to `0.7.6`

For older release notes, see `UPDATEHISTORY.md`.
