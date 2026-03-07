# Hallinta - Noita Mod Manager

A mod manager for Noita with preset support, backup/restore, and a structured logging system. Built with Rust + egui (eframe).

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

### Modpacks
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
- Save Monitor blocks mod/preset mutations while running (independent of UI layout)
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
- Panic crash handler writes panic payload, location, thread, and stack trace to session logs
- Log files accessible via Settings > Open Settings Folder
- Configurable log level controls recording verbosity

### Smart Directory Detection
- **Noita Save Directory**: Automatically finds save location on Windows and Linux (Proton/Steam)
- **Entangled Worlds** (optional): Detects multiplayer mod directories on Windows and Linux
- Manual directory selection with Browse and Auto-detect buttons

### User Interface
- Native desktop GUI powered by egui/eframe
- Dark and light mode
- Compact Mode: independent toggle that shrinks the window and hides the mod list for a monitoring-focused layout
- Context menus for mod operations (toggle, delete, reorder, workshop links)
- Search and filter functionality
- Responsive layout

### Settings & Configuration
- Persistent settings stored in system data directories
- User-configurable preset catalog URL
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
3. If not found, use Auto-detect or Browse in Settings
4. Entangled Worlds directory detection is optional

### Directory Configuration
- **Settings > Noita Saved Data Directory**: Required for core functionality
- **Settings > Entangled Worlds Directory**: Optional multiplayer support
- Use Auto-detect to auto-locate standard installations
- Browse for custom installations

## Interface Overview

- **Header**: `Mod List` / `Modpacks` tabs, search bar, preset controls, settings access
- **Mod List View**: Main mod list with drag-and-drop reordering
- **Modpacks**: Browse, search, and download presets from a configured catalog
- **Settings**: Directory configuration, appearance, backup, logging, catalog URL, Steam path

## Technical Details

- **Language**: Rust
- **GUI Framework**: eframe/egui 0.30
- **Data Storage**: JSON files in platform data directories
- **File Monitoring**: Real-time mod_config.xml watching
- **Logging**: Structured session logging with file rotation

## Planned

- macOS directory detection improvements
- Enhanced Entangled Worlds multiplayer mod support
- Further Workshop integration and Google Drive link testing

## Prerequisites

Unknown, planning for none aside from compiling within Rust within all platforms (Mac, Linux, Windows)

### Noita on Linux
Hallinta auto-detects Noita save data under Steam's Proton prefix:
```
~/.local/share/Steam/steamapps/compatdata/881100/pfx/drive_c/users/steamuser/AppData/LocalLow/Nolla_Games_Noita/save00
```
If your Steam library is in a non-default location, use **Settings > Auto-detect** or **Browse** to set the path manually.

## Building

```bash
# Development
cargo run

# Production build
cargo build --release
```
