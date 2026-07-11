# Development Mode

Running `cargo run` enables these development-build changes:

- The window title becomes `Hallinta [DEV] v<version> (<git-hash>)`.
- Log filenames include `_dev`, and session markers identify the build as `debug`.
- Automatic update checks and installs are disabled; manual checks report that updates require an official GitHub release build.
- On Windows, Hallinta keeps the console subsystem enabled so terminal output remains available.
