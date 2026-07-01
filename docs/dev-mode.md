# Dev Mode (Debug Builds)

Debug builds (`cargo run`) keep Hallinta's own runtime data under the repo-local
`dev_data/` directory:

```text
<repo>/dev_data/
├── backups/
├── logs/
├── presets.json
└── settings.json
```

Release builds use the OS-local Hallinta data directory instead:

- Windows: `%LOCALAPPDATA%\Hallinta\`
- Linux: `~/.local/share/Hallinta/`

## Noita Save Paths

Debug builds do not redirect Noita or Entangled Worlds save paths. The paths in
Settings are the paths Hallinta reads and writes in both debug and release
builds.

For risky backup, restore, or mod-toggle testing, point Settings at a manually
maintained copy of your Noita `save00` directory. The app will not create or
refresh a save sandbox automatically.

## Debug Markers

- Window title includes `[DEV]`.
- Log filenames include `_dev`.
- App-owned debug data is stored under `dev_data/`.
- Unit tests must use isolated temp directories for filesystem writes.

## Stale Sandbox Cleanup

Older debug builds created `dev_data/save00/`, `dev_data/entangled_worlds/`,
and `dev_data/.originals/`. These folders are no longer used. They can be
removed manually while keeping Hallinta's debug settings, logs, backups, and
presets:

```powershell
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue dev_data\save00, dev_data\entangled_worlds, dev_data\.originals
```

If your debug `settings.json` still points Noita or Entangled Worlds at those
old `dev_data` folders, update the paths in Settings with Auto-detect or Browse
after cleanup.
