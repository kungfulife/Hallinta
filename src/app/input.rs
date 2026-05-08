use super::HallintaApp;
use crate::core::logging;
use crate::models::{ConfirmAction, Modal, View};
use eframe::egui;

impl HallintaApp {
    // ── Keyboard Handling ──────────────────────────────────────────────

    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let modal_open = self.active_modal.is_some();
        let monitor_running = self.save_monitor.is_running();
        let backup_busy = self.backup_state.in_progress || self.backup_state.restoring;

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if modal_open {
                if !matches!(self.active_modal, Some(Modal::Progress { .. })) {
                    self.active_modal = None;
                }
            } else if self.active_view == View::Settings {
                self.active_view = View::ModList;
            }
        }

        // Skip remaining shortcuts if a modal/text input is consuming keys
        if modal_open {
            return;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        let ctrl = ctx.input(|i| i.modifiers.command_only());

        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.focus_search_requested = true;
            self.active_view = View::ModList;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F5)) && !typing {
            self.reload_mods_explicit();
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::B))
            && !backup_busy
            && !monitor_running
            && self.active_view == View::ModList
        {
            self.start_backup_modal();
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::E))
            && !monitor_running
            && !typing
            && self.active_view == View::ModList
        {
            self.bulk_set_enabled(true);
        }
        if ctrl
            && ctx.input(|i| i.key_pressed(egui::Key::D))
            && !monitor_running
            && !typing
            && self.active_view == View::ModList
        {
            self.bulk_set_enabled(false);
        }
        if ctrl && ctx.input(|i| i.key_pressed(egui::Key::Comma)) {
            self.active_view = if self.active_view == View::Settings {
                View::ModList
            } else {
                View::Settings
            };
        }
    }

    /// Enable or disable every mod. Single audit log line; same path used by
    /// both keyboard shortcut and footer button so the trail is uniform.
    pub fn bulk_set_enabled(&mut self, enabled: bool) {
        if self.save_monitor.is_running() {
            return;
        }
        let total = self.current_mods.len();
        if total == 0 {
            return;
        }
        let was_set = self
            .current_mods
            .iter()
            .filter(|m| m.enabled == enabled)
            .count();
        for m in &mut self.current_mods {
            m.enabled = enabled;
        }
        let _ = logging::log(
            "INFO",
            &format!(
                "{} all mods ({} now {}, {} were already)",
                if enabled { "Enabled" } else { "Disabled" },
                total,
                if enabled { "enabled" } else { "disabled" },
                was_set,
            ),
            "ModManager",
        );
        self.save_mod_config_and_preset();
    }

    // ── Close Handling ─────────────────────────────────────────────────

    pub(super) fn handle_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }

        if self.save_monitor.is_running() && !self.close_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_requested = true;
            let _ = logging::log(
                "INFO",
                "Close requested while monitor running — prompting for final snapshot",
                "App",
            );
            self.active_modal = Some(Modal::Confirm {
                message: "Save Monitor is running. Take a final snapshot before closing?"
                    .to_string(),
                confirm_text: "Snapshot & Close".to_string(),
                cancel_text: "Close Without Snapshot".to_string(),
                action: ConfirmAction::ExitWithSnapshot,
                cancel_action: Some(ConfirmAction::ExitWithoutSnapshot),
            });
        }
    }
}
