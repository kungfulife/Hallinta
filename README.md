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
- Dark and light mode
- Compact Mode: independent toggle that shrinks the window and hides the mod list for a monitoring-focused layout
- Context menus for mod operations (toggle, delete, reorder, workshop links)
- Search and filter functionality
- Theme-safe custom dropdowns for Presets and Log Level (consistent light/dark list-item rendering)
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

- **Header**: `Mod List` / `Preset Vault` tabs, search bar, preset controls, settings access
- **Mod List View**: Main mod list with drag-and-drop reordering
- **Preset Vault**: Browse, search, and download presets from a configured catalog
- **Settings**: Directory configuration, appearance, backup, logging, catalog URL, Steam path

## Technical Details

- **Backend**: Rust with Tauri 2
- **Frontend**: Vanilla JavaScript (ES6 modules, no bundler)
- **Data Storage**: JSON files in platform data directories
- **File Monitoring**: Real-time mod_config.xml watching
- **Logging**: Structured session logging with file rotation

## Planned

- macOS directory detection improvements
- Enhanced Entangled Worlds multiplayer mod support
- Further Workshop integration and Google Drive link testing

## Prerequisites

### Windows
No additional system dependencies required beyond Rust and the [Tauri CLI](https://v2.tauri.app/start/prerequisites/).

### Linux (Ubuntu, Pop!_OS, Debian-based)

Install the system libraries required to **build** the application:

```bash
sudo apt-get install -y \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev \
  librsvg2-dev \
  libssl-dev \
  pkg-config \
  patchelf
```

> **Note:** For system tray support, install **one** of: `libappindicator3-dev` or `libayatana-appindicator3-dev`. Pop!_OS and newer Ubuntu versions ship with the Ayatana variant (`libayatana-appindicator3-dev`); older Ubuntu or plain Debian may use `libappindicator3-dev`. If one conflicts, use the other.

These provide the GTK, WebKit, and related libraries that Tauri v2 links against at compile time. The corresponding runtime libraries (`libgtk-3-0`, `libwebkit2gtk-4.1-0`, etc.) are needed to **run** the built application and are typically already present on desktop Linux installations.

### Linux (Arch, CachyOS, Manjaro)

```bash
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  gtk3 \
  libappindicator-gtk3 \
  librsvg \
  patchelf \
  openssl \
  pkgconf
```

### Noita on Linux
Hallinta auto-detects Noita save data under Steam's Proton prefix:
```
~/.local/share/Steam/steamapps/compatdata/881100/pfx/drive_c/users/steamuser/AppData/LocalLow/Nolla_Games_Noita/save00
```
If your Steam library is in a non-default location, use **Settings > Auto-detect** or **Browse** to set the path manually.

## Building

```bash
# Development
cargo tauri dev

# Production build
cargo tauri build
```

## Latest Version

Current version: **0.7.8**

Latest update highlights:
- Decoupled Compact Mode from Save Monitor into an independent UI toggle (header button + settings checkbox)
- Save Monitor now only blocks mutation actions without changing the layout
- Removed the in-app log viewer entirely (log files remain accessible via Settings > Open Settings Folder)
- Changed Save Monitor default snapshot interval from 15 to 3 minutes
- Renamed "Find Default" to "Auto-detect" in settings directory fields
- Updated application version to `0.7.8`

For older release notes, see `UPDATEHISTORY.md`.
