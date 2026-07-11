# Development Mode

Running `cargo run` enables these development-build changes:

- The window title becomes `Hallinta [DEV] v<version> (<git-hash>)`.
- Log filenames include `_dev`, and session markers identify the build as `debug`.
- Signed automatic update checks and installs are disabled; manual checks report that updates require an official GitHub release build.
- On Windows, Hallinta keeps the console subsystem enabled so terminal output remains available.

## Debug-only previews

- **Actions > Preview > Preview Noita Warning** temporarily shows the invalid-Noita banner and workspace tint. **End Warning Preview** clears the simulation. This is visual-only: it does not change configured paths, the real directory-error state, or Noita sync behavior.
