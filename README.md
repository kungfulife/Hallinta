# Hallinta — Noita Mod Manager

Hallinta is a Rust + egui desktop app for managing Noita mods, local presets, backups, and save-monitor snapshots.

> **Platform status:** Windows 11 is supported. Linux and macOS are **not supported yet**; implementation and testing after the egui rewrite are still in progress.

## Features

- Manage, search, filter, sort, and reorder Noita's `mod_config.xml`.
- Create and switch local presets; import/export mod lists and presets as local files.
- Check imported Steam Workshop mods and open Workshop actions.
- Back up and selectively restore saves, presets, and optional Entangled Worlds data.
- Capture and restore Save Monitor session snapshots.
- Use dark/light themes, adjustable scaling, compact mode, and session/crash logs.
- Apply SHA-256-verified updates from official Windows releases.

## Windows 11

Download the portable `Hallinta-x86_64-pc-windows-msvc.exe` from the [latest release](https://github.com/kungfulife/Hallinta/releases/latest).

On launch, Hallinta automatically detects Noita's save directory and loads `mod_config.xml`. If no usable directory is found—or a saved location becomes invalid—Hallinta highlights the issue and directs you to Settings. Optional Entangled Worlds and Steam paths are detected when available.

Version 0.8.1 requires one manual install; supported release builds can update in place afterward.

## Build

```bash
cargo run
cargo build --release
```

Linux/macOS builds are currently untested and unsupported.

See [`docs/code-map.md`](docs/code-map.md) for the source layout and [`docs/`](docs/) for maintainer documentation.
