# Repository Guidelines

- Repo: https://github.com/kungfulife/Hallinta

## Project Structure & Module Organization

- Source code: `src/`
  - `src/main.rs`: process bootstrap (panic hook, log session init, single-instance lock, eframe startup).
  - `src/app.rs`: main application state, async task orchestration, timers, and action handlers.
  - `src/core/`: domain logic (`mods`, `presets`, `backup`, `save_monitor`, `gallery`, `workshop`, `settings`, `logging`, `platform`).
  - `src/ui/`: egui rendering modules (`header`, `sidebar`, `mod_list`, `preset_bar`, `gallery`, `settings`, `modals`, `theme`).
  - `src/models.rs`: shared data models and UI enums.
  - `src/tasks.rs`: async task result messages.
- Tests: unit tests are colocated in `#[cfg(test)]` modules.
- Docs: `docs/`.
- Build outputs: `target/`.
- Runtime data directory:
  - Debug builds: `<repo>/dev_data`
  - Release builds: local app data `Hallinta` directory via `dirs::data_local_dir()`.

## Build, Test, and Development Commands

- `cargo run`: run in debug mode.
- `cargo run --release`: run optimized release build.
- `cargo build`: compile debug build.
- `cargo build --release`: compile release build.
- `cargo test`: run all tests.
- `cargo fmt --all`: format code.
- `cargo fmt --all --check`: formatting check for CI.
- `cargo clippy --all-targets --all-features -- -D warnings`: lint with warnings denied.

## Coding Style & Naming Conventions

- Language: Rust (edition 2024).
- Add brief code comments for tricky or non-obvious logic.
- Keep files concise.
- Aim to keep files under ~700 LOC; guideline only (not a hard guardrail). Split/refactor when it improves clarity or testability.
- Naming: use **Hallinta** for product/app/docs headings; use `hallinta` for CLI command, package/binary, paths, and config keys.
- Feature naming: use `Preset` for local saved configurations and **Modpacks** for remote catalog/download UX.

## Release Channels (Naming)

- Debug/dev channel:
  - Window title includes `[DEV]`.
  - Log filenames include `_dev` suffix.
  - Data is stored in `dev_data/` under repo root.
- Release channel:
  - No `[DEV]` title marker.
  - Data is stored in OS local app data (`Hallinta`).

## Testing Guidelines

- Add or update unit tests for logic changes in the same module when feasible.
- Use isolated temp directories for filesystem tests; do not rely on user machine state.
- Prefer round-trip tests for serialization/import/export changes.
- For bug fixes, include regression coverage.
- Before handoff on substantial changes, run at least `cargo test` (and `cargo fmt --check` when formatting may be affected).

## Agent-Specific Notes

- When adding a new `AGENTS.md` anywhere in the repo, also add a `CLAUDE.md` symlink pointing to it (example: `ln -s AGENTS.md CLAUDE.md`).
- When working on a GitHub Issue or PR, print the full URL at the end of the task.
- When answering questions, respond with high-confidence answers only: verify in code; do not guess.
- Patching dependencies requires explicit approval; do not do this by default.
- Version location: `Cargo.toml`.
- **Multi-agent safety:** do **not** create/apply/drop `git stash` entries unless explicitly requested (this includes `git pull --rebase --autostash`). Assume other agents may be working; keep unrelated WIP untouched and avoid cross-cutting state changes.
- **Multi-agent safety:** when the user says "push", you may `git pull --rebase` to integrate latest changes (never discard other agents' work). When the user says "commit", scope to your changes only. When the user says "commit all", commit everything in grouped chunks.
- **Multi-agent safety:** do **not** create/remove/modify `git worktree` checkouts (or edit `.worktrees/*`) unless explicitly requested.
- **Multi-agent safety:** do **not** switch branches / check out a different branch unless explicitly requested.
- **Multi-agent safety:** running multiple agents is OK as long as each agent has its own session.
- **Multi-agent safety:** when you see unrecognized files, keep going; focus on your changes and commit only those.
- **Multi-agent safety:** focus reports on your edits; avoid guard-rail disclaimers unless truly blocked; when multiple agents touch the same file, continue if safe; end with a brief "other files present" note only if relevant.
- Bug investigations: read source code of relevant cargo dependencies and all related local code before concluding; aim for high-confidence root cause.
- Code style: add brief comments for tricky logic; keep files under ~500 LOC when feasible (split/refactor as needed).
- Release guardrails: do not change version numbers without operator's explicit consent.

## Changelog Release Notes

- Write concise, user-facing bullets grouped by feature area.
- Mention data format changes and migration impacts explicitly.
- Separate behavior fixes from UI-only wording changes where possible.
