use super::{DeferredViewportAction, HallintaApp, sort_mods};
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
        let _ = self.try_save_mod_config_and_preset();
    }

    pub(crate) fn try_save_mod_config_and_preset(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        self.presets
            .insert(self.selected_preset.clone(), self.current_mods.clone());

        let noita_dir = self.settings.noita_dir.clone();
        if !noita_dir.is_empty() {
            let xml = mods::mods_to_xml(&self.current_mods);
            match mods::write_mod_config(&PathBuf::from(&noita_dir), &xml) {
                Ok(()) => {
                    self.file_watcher.pending_external_mods = None;
                    let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
                    if let Ok(mtime) = mods::get_file_modified_time(&config_path) {
                        self.file_watcher.last_modified_time = mtime;
                    }
                }
                Err(e) => {
                    let message = format!("Failed to write mod_config.xml at {noita_dir}: {e}");
                    let _ = logging::log("ERROR", &message, "ModManager");
                    errors.push(message);
                }
            }
        } else {
            let _ = logging::log(
                "WARN",
                "save_mod_config_and_preset called with empty noita_dir — preset still saved",
                "ModManager",
            );
        }

        if let Err(e) = presets::save_presets(&self.presets) {
            let message = format!("Failed to save presets.json: {e}");
            let _ = logging::log("ERROR", &message, "PresetManager");
            errors.push(message);
        }
        self.settings.selected_preset = self.selected_preset.clone();
        if let Err(e) = settings::save_settings(&self.settings) {
            let message = format!("Failed to save settings.json: {e}");
            let _ = logging::log("ERROR", &message, "Settings");
            errors.push(message);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    /// Persist current settings to disk.
    pub fn save_current_settings(&mut self) {
        logging::configure(&self.settings.log_settings);
        if let Err(e) = settings::save_settings(&self.settings) {
            let _ = logging::log(
                "ERROR",
                &format!("Failed to save settings.json: {}", e),
                "Settings",
            );
        }
    }

    /// Called when the theme preference changes.
    pub fn on_dark_mode_changed(&mut self, ctx: &egui::Context) {
        self.dark_mode = self.settings.dark_mode;
        crate::ui::theme::apply_theme(ctx, self.settings.dark_mode);
        self.save_current_settings();
    }

    /// Called when compact mode changes from settings restoration/reset flows.
    pub fn on_compact_mode_changed(&mut self, ctx: &egui::Context) {
        let was_compact = self.compact_mode;
        self.compact_mode = self.settings.compact_mode;
        if was_compact != self.compact_mode {
            self.apply_compact_mode_viewport(ctx);
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
        if (self.settings.ui_scale - prev_scale).abs() < 0.001 {
            return;
        }

        let scale = self.settings.ui_scale;
        let base_min = if self.compact_mode {
            crate::ui::design::BASE_MIN_COMPACT
        } else {
            crate::ui::design::BASE_MIN_NORMAL
        };

        let new_min = crate::ui::design::scaled_min_size(base_min, scale);
        let current_size = ctx.input(|i| i.content_rect().size());
        let ratio = scale / prev_scale;
        let new_w = (current_size.x * ratio).max(new_min.x);
        let new_h = (current_size.y * ratio).max(new_min.y);
        let new_size = egui::vec2(new_w, new_h);

        self.capture_viewport_center(ctx);
        self.queue_viewport_resize(ctx, new_size, new_min);

        if !self.compact_mode {
            self.normal_window_size = Some(new_size);
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
        self.apply_compact_mode_viewport(ctx);
    }

    fn apply_compact_mode_viewport(&mut self, ctx: &egui::Context) {
        let scale = self.settings.ui_scale;
        self.capture_viewport_center(ctx);

        if self.compact_mode {
            let current_size = ctx.input(|i| i.content_rect().size());
            if current_size.x > 500.0 {
                self.normal_window_size = Some(current_size);
            }
            self.queue_viewport_resize(
                ctx,
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_COMPACT, scale),
                crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_COMPACT, scale),
            );
        } else {
            let min = crate::ui::design::scaled_min_size(crate::ui::design::BASE_MIN_NORMAL, scale);
            let size = self.normal_window_size.unwrap_or_else(|| {
                crate::ui::design::scaled_size(crate::ui::design::BASE_SIZE_NORMAL, scale)
            });
            self.queue_viewport_resize(ctx, size, min);
        }
    }

    fn capture_viewport_center(&mut self, ctx: &egui::Context) {
        self.pending_viewport_center = ctx.input(|i| {
            let viewport = i.viewport();
            let outer = viewport.outer_rect?;
            let monitor = viewport.monitor_size?;
            if monitor.x <= 0.0 || monitor.y <= 0.0 {
                return None;
            }
            let center = outer.center();
            Some((center.x / monitor.x, center.y / monitor.y))
        });
    }

    fn queue_viewport_resize(
        &mut self,
        ctx: &egui::Context,
        inner_size: egui::Vec2,
        min_size: egui::Vec2,
    ) {
        let current = ctx.input(|i| i.content_rect().size());
        if (current - inner_size).length_sq() < 4.0 && self.deferred_viewport_action.is_none() {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
            self.queue_viewport_reposition(ctx, inner_size);
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(1.0, 1.0)));
        self.deferred_viewport_action = Some(DeferredViewportAction::ResizeThenMin {
            inner_size,
            min_size,
        });
    }

    pub(super) fn queue_viewport_reposition(
        &mut self,
        ctx: &egui::Context,
        inner_size: egui::Vec2,
    ) {
        let Some((frac_x, frac_y)) = self.pending_viewport_center.take() else {
            return;
        };
        let Some(outer_pos) = ctx.input(|i| {
            let viewport = i.viewport();
            let monitor = viewport.monitor_size?;
            let outer = viewport.outer_rect?;
            let inner = viewport.inner_rect.unwrap_or(i.content_rect());
            let decoration = egui::vec2(
                (outer.width() - inner.width()).max(0.0),
                (outer.height() - inner.height()).max(0.0),
            );
            let target_outer_w = inner_size.x + decoration.x;
            let target_outer_h = inner_size.y + decoration.y;
            let center_x = frac_x * monitor.x;
            let center_y = frac_y * monitor.y;
            let x =
                (center_x - target_outer_w * 0.5).clamp(0.0, (monitor.x - target_outer_w).max(0.0));
            let y =
                (center_y - target_outer_h * 0.5).clamp(0.0, (monitor.y - target_outer_h).max(0.0));
            Some(egui::pos2(x, y))
        }) else {
            return;
        };
        self.deferred_viewport_action = Some(DeferredViewportAction::Reposition { outer_pos });
    }

    pub fn reload_mods(&mut self) {
        let noita_dir = self.settings.noita_dir.clone();
        if noita_dir.trim().is_empty() {
            self.noita_directory_error = Some(super::noita_directory_error_message(&noita_dir, ""));
            return;
        }
        let dir = PathBuf::from(&noita_dir);
        match mods::load_mod_config(&dir) {
            Ok(file_mods) => {
                self.noita_directory_error = None;
                self.current_mods = file_mods;
                self.file_watcher.pending_external_mods = None;
                self.presets
                    .insert(self.selected_preset.clone(), self.current_mods.clone());
                let _ = presets::save_presets(&self.presets);
            }
            Err(error) => {
                self.noita_directory_error =
                    Some(super::noita_directory_error_message(&noita_dir, &error));
                return;
            }
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

    pub fn visible_noita_directory_error(&self) -> Option<&str> {
        #[cfg(debug_assertions)]
        if self.preview_noita_directory_warning {
            return Some(super::NOITA_WARNING_PREVIEW_MESSAGE);
        }
        self.noita_directory_error.as_deref()
    }

    #[cfg(debug_assertions)]
    pub fn toggle_noita_warning_preview(&mut self) {
        self.preview_noita_directory_warning = !self.preview_noita_directory_warning;
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

    pub(super) fn configured_entangled_dir(&self) -> Option<String> {
        let trimmed = self.settings.entangled_dir.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    /// Check workshop install state based on cached workshop status.
    pub fn workshop_mod_install_state(&self, workshop_id: &str) -> WorkshopInstallState {
        if workshop_id.is_empty() || workshop_id == "0" {
            return WorkshopInstallState::Installed;
        }
        self.backup_state
            .workshop_status
            .iter()
            .find(|(id, _)| id == workshop_id)
            .map(|(_, state)| *state)
            .unwrap_or(WorkshopInstallState::Unknown)
    }

    // ── Open mod_config.xml ───────────────────────────────────────────

    pub fn open_mod_config_file(&self) {
        let noita_dir = self.settings.noita_dir.clone();
        if noita_dir.is_empty() {
            return;
        }
        let config_path = PathBuf::from(&noita_dir).join("mod_config.xml");
        if mods::check_file_exists(&config_path) {
            let _ = platform::open_file(&config_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{mod_entry, test_app};
    use super::*;

    #[test]
    fn reload_mods_clears_pending_external_mods_after_loading_disk() {
        let dir = std::env::temp_dir().join(format!(
            "hallinta_reload_pending_test_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("test dir should be created");
        let disk_mods = vec![mod_entry("Alpha", false, "1")];
        mods::write_mod_config(&dir, &mods::mods_to_xml(&disk_mods))
            .expect("test mod_config should be written");

        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.settings.noita_dir = dir.to_string_lossy().to_string();
        app.file_watcher.pending_external_mods = Some(disk_mods.clone());

        app.reload_mods();

        assert!(
            app.file_watcher.pending_external_mods.is_none(),
            "manual reload should reconcile pending external-change state"
        );
        assert_eq!(app.current_mods.len(), 1);
        assert!(!app.current_mods[0].enabled);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn apply_sort_to_order_is_not_monitor_locked() {
        let (_runtime, mut app) = test_app(vec![
            mod_entry("Beta", false, "2"),
            mod_entry("Alpha", true, "1"),
        ]);
        app.save_monitor.running = true;
        app.sort_mode = SortMode::NameAsc;

        app.apply_sort_to_order();

        assert_eq!(app.current_mods[0].name, "Alpha");
        assert_eq!(app.current_mods[1].name, "Beta");
        assert_eq!(app.sort_mode, SortMode::Default);
    }

    #[test]
    fn mod_config_failure_still_updates_other_persistence_state() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.selected_preset = "Changed".to_string();
        app.settings.selected_preset = "Default".to_string();
        app.settings.noita_dir = std::env::temp_dir()
            .join(format!("hallinta-missing-save-dir-{}", std::process::id()))
            .to_string_lossy()
            .to_string();

        let error = app
            .try_save_mod_config_and_preset()
            .expect_err("missing mod directory should fail one persistence sink");

        assert!(error.contains("mod_config.xml"));
        assert_eq!(app.settings.selected_preset, "Changed");
    }

    #[test]
    fn reload_records_an_empty_noita_directory_error() {
        let (_runtime, mut app) = test_app(vec![mod_entry("Alpha", true, "1")]);
        app.settings.noita_dir.clear();

        app.reload_mods();

        assert_eq!(
            app.noita_directory_error.as_deref(),
            Some("Noita save directory was not found.")
        );
    }

    #[test]
    fn warning_preview_does_not_change_the_real_error_or_saved_path() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.settings.noita_dir = "C:/Noita/save00".to_string();
        app.noita_directory_error = Some("real error".to_string());

        app.toggle_noita_warning_preview();

        assert!(app.preview_noita_directory_warning);
        assert_eq!(app.settings.noita_dir, "C:/Noita/save00");
        assert_eq!(app.noita_directory_error.as_deref(), Some("real error"));
        assert_eq!(
            app.visible_noita_directory_error(),
            Some(
                "Preview: Hallinta could not load mod_config.xml from the detected Noita save directory."
            )
        );

        app.toggle_noita_warning_preview();

        assert!(!app.preview_noita_directory_warning);
        assert_eq!(app.visible_noita_directory_error(), Some("real error"));
    }
}
