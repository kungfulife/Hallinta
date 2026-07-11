# Development Mode

Running `cargo run` enables these development-build changes:

- The window title becomes `Hallinta [DEV] v<version> (<git-hash>)`.
- Log filenames include `_dev`, and session markers identify the build as `debug`.
- Automatic update checks and installs are disabled; manual checks report that updates require an official GitHub release build.
- On Windows, Hallinta keeps the console subsystem enabled so terminal output remains available.

## Debug-only previews

- **Actions > Preview > Preview Noita Warning** temporarily shows the invalid-Noita banner and workspace tint. **End Warning Preview** clears the simulation. Neither action changes configured paths or the real directory-error state.
