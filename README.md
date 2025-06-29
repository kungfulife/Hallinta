# Hallinta - Noita Mod Manager

A modern mod manager for Noita with preset support, built using Rust + Tauri for cross-platform compatibility.

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
- Automatic preset synchronization with mod_config.xml
- Conflict resolution when external changes detected

### Smart Directory Detection
- **Noita Save Directory**: Automatically finds Windows save location (`%USERPROFILE%\AppData\LocalLow\Nolla_Games_Noita\save00`)
- **Entangled Worlds** *(Optional)*: Detects multiplayer mod directories
    - Windows: `%APPDATA%\Roaming\quant\entangledworlds`
    - Linux: `~/.config/entangledworlds`
- Manual directory selection with "Browse" and "Find Default" buttons

### User Interface
- Clean, modern interface with dark/light mode support
- Context menus for mod operations (toggle, delete, reorder, workshop links)
- Search/filter functionality
- Responsive design with smooth animations
- Status bar with helpful messages and Easter eggs

### Settings & Configuration
- Persistent settings stored in system data directories
- Configurable log levels (Debug, Info, Warn, Error)
- Version upgrade detection with automatic backups
- Settings validation and error recovery

## Current Status

### ✅ Completed Features
- Core mod management functionality
- Preset system with full CRUD operations
- Directory auto-detection for Noita (Windows)
- Settings UI with directory management
- File monitoring and conflict resolution
- Dark mode support
- Comprehensive logging system

### 🚧 In Progress / Planned
- **Backup System**: Infrastructure ready, UI implementation pending
    - Noita save folder backup (save00 directory)
    - Entangled Worlds data backup (data subfolder)
- **Import/Export**: Preset sharing functionality
- **Cross-platform**: Linux/macOS directory detection improvements
- **Entangled Worlds Integration**: Enhanced multiplayer mod support

## Setup

### First Run
1. Launch Hallinta
2. Application automatically detects Noita save directory (Windows)
3. If not found, use "Find Default" or "Browse" in Settings
4. Entangled Worlds directory detection is optional

### Directory Configuration
- **Settings → Noita Saved Data Directory**: Required for core functionality
- **Settings → Entangled Worlds Directory**: Optional multiplayer support
- Use "Find Default" to auto-locate standard installations
- "Browse" for custom installations

## Interface Overview

- **Header**: Search bar, preset controls, settings access
- **Main View**: Mod list with drag-and-drop reordering
- **Settings**: Directory configuration, appearance, logging
- **Status Bar**: Click for detailed application logs

## Technical Details

- **Backend**: Rust with Tauri framework
- **Frontend**: Vanilla JavaScript with modern CSS
- **Data Storage**: JSON files in system data directories
- **File Monitoring**: Real-time mod_config.xml watching
- **Logging**: Structured logging with daily file rotation

## What Makes It Different

Hallinta focuses on seamless integration with Noita's existing mod system while adding preset functionality that many players want. It respects the game's mod_config.xml format and provides conflict resolution when the file is modified externally.

**Version 0.3.5** - Active Development