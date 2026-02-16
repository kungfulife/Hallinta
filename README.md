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
- Conflict resolution when external changes detected

### Preset Vault
- Browse and download presets from a developer-maintained catalog
- Search and tag-based filtering
- One-click download with JSON validation and checksum verification
- Import presets directly from Google Drive share links
- Workshop mod check: detects missing Steam Workshop mods on import
- Per-mod Steam Subscribe buttons for quick installation

### Backup & Restore
- Manual and automatic backups of Noita save data (save00, save01) and presets
- Configurable auto-backup interval
- Auto-deletion of old backups (configurable retention period)
- Selective restore with per-component options
- Upgrade backups created automatically on version change (keeps last 5)

### Application Logs
- Always-on session logging for reliable crash diagnostics
- Three viewing modes: modal overlay, fullscreen panel, separate OS window
- Level-based filtering (Debug, Info, Warn, Error)
- Search within log entries
- Auto-refresh with smart scroll (preserves position when reading history)
- Color-coded log levels with left-border indicators
- Copy filtered logs to clipboard

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
- Configurable log level controls recording verbosity
- Optional startup system diagnostics collection (disabled by default)
- Backup scheduling and retention settings
- Steam path auto-detection during initial setup
- Developer-managed catalog source (Catalog URL hidden from user settings)
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
- **Preset Vault**: Browse, search, and download curated presets
- **Settings**: Directory configuration, appearance, backup, logging, preset import checks
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

## Building

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build
```

## Latest Version

Current version: **0.7.3**

Latest update highlights:
- Redesigned logging system: logging is now always active (cannot be disabled)
- Removed the `Enable application logging` toggle from Settings
- Log level dropdown now controls recording verbosity (what detail gets captured)
- Log file and session marker are now created immediately on startup for crash resilience
- Fixed status bar not showing messages when log level was set above Info
- Standardized default log level to Info for both development and release builds
- Updated application version to `0.7.3`

For older release notes, see `UPDATEHISTORY.md`.
