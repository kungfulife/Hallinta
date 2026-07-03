# Debug Builds

Debug builds (`cargo run`) use the same OS-local Hallinta data directory as
release builds. This keeps settings, presets, logs, backups, and monitor
snapshots in one predictable application data location across build profiles.

- Windows: `%LOCALAPPDATA%\Hallinta\`
- Linux: `~/.local/share/Hallinta/`

## Noita Save Paths

Debug builds do not redirect Noita or Entangled Worlds save paths. The paths in
Settings are the paths Hallinta reads and writes in every build profile.

For risky backup, restore, or mod-toggle testing, point Settings at a manually
maintained copy of your Noita `save00` directory. The app will not create or
refresh a save sandbox automatically.

## Debug Markers

- Window title includes `[DEV]`.
- Log filenames include `_dev`.
- Unit tests must use isolated temp directories for filesystem writes.

## Local Migration Note

If you previously ran older debug builds with a repo-local runtime folder, copy
any settings, presets, backups, or logs you still want into the OS-local
Hallinta data directory manually before pruning that old folder.
