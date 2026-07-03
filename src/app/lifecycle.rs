use super::HallintaApp;
use crate::core::logging;
use crate::models::View;
use eframe::egui;

impl HallintaApp {
    // ── Cleanup ────────────────────────────────────────────────────────

    pub fn cleanup_on_exit(&mut self) {
        let _ = logging::log("INFO", "Application shutting down", "App");

        logging::write_session_marker("APP_SHUTDOWN");

        let _ = logging::flush_log_buffer_sync();
        logging::write_session_end_marker();
        let _ = logging::flush_log_buffer_sync();
    }
}

impl eframe::App for HallintaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 0. Apply UI zoom (must be before any rendering)
        crate::ui::design::apply_zoom(ctx, &self.settings);

        // 0b. Apply deferred viewport resizing (queued to avoid OS min-size lag)
        if let Some(action) = self.deferred_viewport_action.take() {
            match action {
                super::DeferredViewportAction::ResizeThenMin {
                    inner_size,
                    min_size,
                } => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(inner_size));
                    self.deferred_viewport_action =
                        Some(super::DeferredViewportAction::ApplyMin { min_size });
                }
                super::DeferredViewportAction::ApplyMin { min_size } => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min_size));
                }
            }
        }

        // 1. Poll async task results
        self.poll_task_results();

        // 2. Check timers
        self.check_timers(ctx);

        // 3. Handle close request
        self.handle_close(ctx);

        // 4. Handle keyboard
        self.handle_keyboard(ctx);

        // 5. Render UI
        crate::ui::header::render_header(self, ctx);

        if !self.compact_mode && self.active_view != View::Settings {
            crate::ui::sidebar::render_sidebar(self, ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| match self.active_view {
            View::ModList => {
                if self.compact_mode {
                    crate::ui::compact::render_compact(self, ui);
                } else {
                    crate::ui::mod_list::render_mod_list(self, ui);
                }
            }
            View::Settings => {
                crate::ui::settings::render_settings(self, ui);
            }
        });

        // 6. Render modals on top
        crate::ui::modals::render_modals(self, ctx);

        self.close_after_monitor_prompt(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cleanup_on_exit();
    }
}
