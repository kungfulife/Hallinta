use super::{HallintaApp, sort_mods};
use crate::core::{logging, mods, platform, presets, settings};
use crate::models::*;
use eframe::egui;
use std::path::PathBuf;

impl HallintaApp {
    // ── Public Actions ─────────────────────────────────────────────────

    pub fn switch_preset(&mut self) {
        self.cancel_drag_if_active();
        if let Some(preset_mods) = self.presets.get(&self.selected_preset) {
            let prev_count = self.current_mods.len();
            let new_count = preset_mods.len();
            let new_enabled = preset_mods.iter().filter(|m| m.enabled).count();
            self.current_mods = preset_mods.clone();
            self.save_mod_config_and_preset();
            let _ = logging::log(
                "INFO",
                &format!(
                    "Switched to preset: {} ({} -> {} mods, {} enabled)",
                    self.selected_preset, prev_count, new_count, new_enabled
                ),
                "PresetManager",
            );
            logging::write_session_marker(&format!("PRESET_SWITCH:{}", self.selected_preset));
            self.check_workshop_mods_async();
        } else {
            let _ = logging::log(
                "WARN",
                &format!(
                    "Preset switch requested but \"{}\" not found",
                    self.selected_preset
                ),
                "PresetManager",
            );
        }
    }

    pub fn save_mod_config_and_preset(&mut self) {
        self.presets
            .insert(self.selected_preset.clone(), self.current_mods.clone());

        let noita_dir = self.get_active_noita_dir();
        if !noita_dir.is_empty() {
            let xml = mods::mods_to_xml(&self.current_mods);
            if let Err(e) = mods::write_mod_config(&PathBuf::from(&noita_dir), &xml) {
                let _ = logging::log(
                    "ERROR",
                    &format!("Failed to write mod_config.xml at {}: {}", noita_dir, e),
                    "ModManager",
                );
            }

            let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
            if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                self.file_watcher.last_modified_time = mtime;
            }
        } else {
            let _ = logging::log(
                "WARN",
                "save_mod_config_and_preset called with empty noita_dir — preset still saved",
                "ModManager",
            );
        }

        if let Err(e) = presets::save_presets(&self.presets) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save presets.json: {}", e),
                "PresetManager",
            );
        }
        self.settings.selected_preset = self.selected_preset.clone();
        if let Err(e) = settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save settings.json: {}", e),
                "Settings",
            );
        }
    }

    /// Persist current settings to disk.
    pub fn save_current_settings(&mut self) {
        if let Err(e) = settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save settings.json: {}", e),
                "Settings",
            );
        }
    }

    /// Called when dark_mode checkbox toggles reactively.
    pub fn on_dark_mode_changed(&mut self, ctx: &egui::Context) {
        self.dark_mode = self.settings.dark_mode;
        crate::ui::theme::apply_theme(ctx, self.settings.dark_mode);
        self.save_current_settings();
    }

    /// Called when compact_mode checkbox toggles in settings.
    pub fn on_compact_mode_changed(&mut self, ctx: &egui::Context) {
        let was_compact = self.compact_mode;
        self.compact_mode = self.settings.compact_mode;
        if was_compact != self.compact_mode {
            let scale = self.settings.ui_scale;
            if self.compact_mode {
                let current_size = ctx.input(|i| i.content_rect().size());
                if current_size.x > 500.0 {
                    self.normal_window_size = Some(current_size);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
                ));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                    crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
                ));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                    crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale),
                ));
                let size = self.normal_window_size.unwrap_or_else(|| {
                    crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_NORMAL, scale)
                });
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
            }
        }
        self.save_current_settings();
    }

    /// Called when UI scale changes — resize window proportionally and update min size.
    ///
    /// Viewport commands are deferred (processed next frame), so lowering `MinInnerSize`
    /// causes a "one behind" lag — the OS still enforces the old (larger) minimum.
    /// Fix: immediately clear the min to (1,1) so the OS stops clamping, resize the
    /// window in the same batch, then queue the correct minimum for the next frame
    /// via `deferred_min_size`.
    pub fn on_ui_scale_changed(&mut self, ctx: &egui::Context, prev_scale: f32) {
        let scale = self.settings.ui_scale;
        let base_min = if self.compact_mode {
            crate::ui::design::BASE_MIN_COMPACT
        } else {
            crate::ui::design::BASE_MIN_NORMAL
        };

        let new_min = crate::ui::design::scaled_min_size(base_min, scale);

        // Scale the current window size proportionally to the scale change
        let current_size = ctx.input(|i| i.content_rect().size());
        let ratio = scale / prev_scale;
        let new_w = (current_size.x * ratio).max(new_min.x);
        let new_h = (current_size.y * ratio).max(new_min.y);

        // Phase 1 (this frame's command batch):
        // Clear the minimum so the OS stops enforcing the old (possibly larger) value,
        // then resize the window in the same batch.
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(1.0, 1.0)));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(new_w, new_h)));

        // Phase 2 (next frame): apply the correct minimum.
        self.deferred_min_size = Some(new_min);

        // Update stored normal size so compact→normal restores correctly
        if !self.compact_mode {
            self.normal_window_size = Some(egui::vec2(new_w, new_h));
        }

        self.save_current_settings();
    }

    /// Called when noita_dir text field loses focus with a new value.
    pub fn on_noita_dir_changed(&mut self) {
        let _ = logging::log(
            "INFO",
            &format!("Noita save dir changed: {}", self.settings.noita_dir),
            "Settings",
        );
        self.cancel_drag_if_active();
        self.reload_mods();
        self.check_workshop_mods_async();
        self.save_current_settings();
    }

    pub fn toggle_compact_mode(&mut self, ctx: &egui::Context) {
        self.compact_mode = !self.compact_mode;
        self.settings.compact_mode = self.compact_mode;
        let _ = settings::save_settings(&self.settings);

        let scale = self.settings.ui_scale;
        if self.compact_mode {
            // Save current size before shrinking
            let current_size = ctx.input(|i| i.content_rect().size());
            if current_size.x > 500.0 {
                self.normal_window_size = Some(current_size);
            }
            // Must lower min-size BEFORE setting inner size, or the OS clamps it.
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
            ));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
            ));
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(
                crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale),
            ));
            let size = self.normal_window_size.unwrap_or_else(|| {
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_NORMAL, scale)
            });
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
    }

    pub fn reload_mods(&mut self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let dir = PathBuf::from(&noita_dir);
        if let Ok(xml) = mods::read_mod_config(&dir)
            && let Ok(file_mods) = mods::parse_mods_from_xml(&xml)
        {
            self.current_mods = file_mods;
            self.presets
                .insert(self.selected_preset.clone(), self.current_mods.clone());
            let _ = presets::save_presets(&self.presets);
        }
        let config_path = dir.join("mod_config.xml");
        if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
            self.file_watcher.last_modified_time = mtime;
        }
    }

    /// User-initiated reload (logged + workshop check).
    pub fn reload_mods_explicit(&mut self) {
        self.cancel_drag_if_active();
        let before = self.current_mods.len();
        self.reload_mods();
        let after = self.current_mods.len();
        let _ = logging::log(
            "INFO",
            &format!("Manual reload: {} -> {} mod(s)", before, after),
            "ModManager",
        );
        self.check_workshop_mods_async();
    }

    /// Abort an in-flight drag and restore the pre-drag mod order.
    /// Used whenever a disruptive op (filter/sort/reload/external accept) would otherwise
    /// leave drag indices stale and cause panics or silent data corruption.
    pub fn cancel_drag_if_active(&mut self) {
        if let Some(drag) = self.drag_state.take() {
            self.current_mods = drag.pre_drag_snapshot;
            let _ = logging::log("INFO", "Drag cancelled (disruptive op)", "ModList");
        }
    }

    pub fn set_filter_mode(&mut self, mode: FilterMode) {
        if self.filter_mode == mode {
            return;
        }
        self.cancel_drag_if_active();
        self.filter_mode = mode;
        self.settings.last_filter_mode = mode.as_str().to_string();
        self.save_current_settings();
    }

    pub fn set_sort_mode(&mut self, mode: SortMode) {
        if self.sort_mode == mode {
            return;
        }
        self.cancel_drag_if_active();
        self.sort_mode = mode;
        self.settings.last_sort_mode = mode.as_str().to_string();
        self.save_current_settings();
    }

    /// Persist the current visual sort order back into the actual mod list / preset.
    /// Used when user explicitly clicks "Apply Sort to Order" — destructive op.
    pub fn apply_sort_to_order(&mut self) {
        if self.sort_mode == SortMode::Default {
            return;
        }
        if self.save_monitor.is_running() {
            return;
        }
        let mut sorted = self.current_mods.clone();
        sort_mods(&mut sorted, self.sort_mode);
        let mode = self.sort_mode;
        self.current_mods = sorted;
        self.sort_mode = SortMode::Default;
        self.settings.last_sort_mode = SortMode::Default.as_str().to_string();
        self.save_mod_config_and_preset();
        let _ = logging::log(
            "INFO",
            &format!("Applied sort to file order: {}", mode.label()),
            "ModManager",
        );
    }

    pub fn get_active_noita_dir(&self) -> String {
        if cfg!(debug_assertions)
            && let Ok(dev_dir) = platform::get_dev_save_dir()
        {
            return dev_dir.to_string_lossy().to_string();
        }
        self.settings.noita_dir.clone()
    }

    pub(super) fn get_active_entangled_dir(&self) -> Option<String> {
        if cfg!(debug_assertions) {
            return platform::get_dev_entangled_dir()
                .ok()
                .map(|p| p.to_string_lossy().to_string());
        }

        let trimmed = self.settings.entangled_dir.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Check if a workshop mod is installed based on cached workshop status.
    pub fn is_workshop_mod_installed(&self, workshop_id: &str) -> Option<bool> {
        if workshop_id.is_empty() || workshop_id == "0" {
            return Some(true); // Local mod
        }
        self.backup_state
            .workshop_status
            .iter()
            .find(|(id, _)| id == workshop_id)
            .map(|(_, installed)| *installed)
    }

    // ── Open mod_config.xml ───────────────────────────────────────────

    pub fn open_mod_config_file(&self) {
        let noita_dir = self.get_active_noita_dir();
        if noita_dir.is_empty() {
            return;
        }
        let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
        if mods::check_file_exists(&config_path) {
            let _ = platform::open_file(&config_path);
        }
    }
}
