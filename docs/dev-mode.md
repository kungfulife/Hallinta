# Dev Mode (Debug Builds)

How Hallinta isolates debug runs from real Noita save data.

## Overview

Debug builds (`cargo run`) never read from or write to the user's real Noita save files during a session. All file operations are redirected to a sandboxed `dev_data/` directory under the repo root. On exit, the real files are explicitly restored from a snapshot taken at startup. This means you can freely test mod toggling, preset switching, backup/restore, and save monitoring without touching your actual game data.

## Directory Layout

```
<repo>/dev_data/
├── save00/              # Sandbox copy of real Noita save (mod_config.xml, etc.)
├── entangled_worlds/    # Sandbox copy of Entangled Worlds save
├── .originals/          # Pristine snapshots of real files (created at startup, cleaned on exit)
│   ├── save00/mod_config.xml
│   └── entangled_worlds/mod_config.xml
├── backups/             # Backups created during dev sessions
├── logs/                # Dev session logs (filenames include _dev)
├── presets.json         # Presets storage
└── settings.json        # App settings
```

Release builds use `%LOCALAPPDATA%\Hallinta\` (Windows) or `~/.local/share/Hallinta/` (Linux) instead.

## Sandbox Lifecycle

### Startup

`platform::seed_dev_sandbox()` runs before any mod loading or UI rendering. Two phases:

**Phase 1 — Snapshot originals:**
- Copies real `mod_config.xml` → `dev_data/.originals/save00/mod_config.xml`
- Copies real entangled `mod_config.xml` → `dev_data/.originals/entangled_worlds/mod_config.xml`
- These snapshots are the restore source on exit

**Phase 2 — Seed sandbox:**

*Initial run* (no `dev_data/save00/mod_config.xml` exists):
- Full recursive copy of real Noita save → `dev_data/save00/`
- Full recursive copy of real Entangled Worlds → `dev_data/entangled_worlds/`
- If real paths can't be detected, creates an empty `mod_config.xml` placeholder

*Subsequent runs* (sandbox already populated):
- Sandbox is preserved as-is from the previous session
- Dev changes (mod toggles, reordering, etc.) persist across dev runs
- No files are overwritten from the real directories

### During Session

All reads and writes go through `get_active_noita_dir()` and `get_active_entangled_dir()`, which return the `dev_data/` paths in debug builds. This is enforced at the app level — individual modules don't need to know about dev mode.

Key functions:
- `platform::get_dev_save_dir()` → `dev_data/save00/`
- `platform::get_dev_entangled_dir()` → `dev_data/entangled_worlds/`
- `app::get_active_noita_dir()` → returns dev path in debug, settings path in release
- `app::get_active_entangled_dir()` → same pattern

### Shutdown

`platform::restore_real_dirs_from_dev()` runs during `cleanup_on_exit()`:

1. Restores real `mod_config.xml` from `dev_data/.originals/save00/mod_config.xml`
2. Restores real entangled `mod_config.xml` from `dev_data/.originals/entangled_worlds/mod_config.xml`
3. Deletes the `.originals/` directory

This guarantees the real save directories match their pre-session state, even if a code path accidentally wrote to them.

## Settings vs Active Paths

Settings UI shows the real detected paths (e.g., `C:\Users\...\AppData\Local\Noita\save00`). These are what the user would see in a release build. However, all actual file operations use the dev sandbox paths.

The settings paths still matter in dev mode because:
- They're the source for the initial sandbox seed
- They're what gets auto-detected by Browse/Auto-detect buttons
- They're where snapshots are taken from and restored to

## Resetting the Sandbox

Delete `dev_data/save00/` (or the entire `dev_data/` directory) and restart. The next run will do a fresh full copy from your real Noita save.

```sh
# Full reset
rm -rf dev_data/

# Reset just save data (keeps logs, settings, presets)
rm -rf dev_data/save00/ dev_data/entangled_worlds/
```

## Guard Rails

- `cfg!(debug_assertions)` gates all dev-specific code paths
- `get_dev_save_dir()` and `get_dev_entangled_dir()` return `Err` in release builds
- Real files are snapshotted at startup and restored on exit via `.originals/`
- Window title shows `[DEV]` in debug builds
- Log filenames include `_dev` suffix
- Dev data directory is `.gitignore`d

## Common Pitfalls

- **Empty mod list on first run:** If real Noita save path can't be detected, the sandbox gets an empty `mod_config.xml`. Fix: set the Noita path in Settings (it's used as the seed source).
- **Stale sandbox:** If your real Noita save has changed significantly and you want a fresh copy, delete `dev_data/save00/` and restart.
- **Tests must not touch dev_data:** Unit tests use isolated temp directories (via `test_tmp()` helper). They never read from or write to `dev_data/`.
- **Crash without exit:** If the dev build crashes before `cleanup_on_exit()` runs, `.originals/` will still exist. The next startup will overwrite it with a fresh snapshot, so the restore will use the correct data.
