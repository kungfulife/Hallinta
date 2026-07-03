use super::HallintaApp;
use crate::core::logging;
use crate::models::View;
use eframe::egui;

impl HallintaApp {
    // ── Cleanup ────────────────────────────────────────────────────────

    fn run_frame_logic(&mut self, ctx: &egui::Context) {
        // Apply UI zoom before the next render pass.
        crate::ui::design::apply_zoom(ctx, &self.settings);

        // Apply deferred viewport resizing (queued to avoid OS min-size lag).
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

        self.poll_task_results();
        self.check_timers(ctx);
        self.handle_close(ctx);
        self.handle_keyboard(ctx);
        self.close_after_monitor_prompt(ctx);
    }

    pub fn cleanup_on_exit(&mut self) {
        let _ = logging::log("INFO", "Application shutting down", "App");

        logging::write_session_marker("APP_SHUTDOWN");

        let _ = logging::flush_log_buffer_sync();
        logging::write_session_end_marker();
        let _ = logging::flush_log_buffer_sync();
    }
}

impl eframe::App for HallintaApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.run_frame_logic(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        crate::ui::header::render_header(self, ui);

        if !self.compact_mode && self.active_view != View::Settings {
            crate::ui::sidebar::render_sidebar(self, ui);
        }

        egui::CentralPanel::default().show(ui, |ui| match self.active_view {
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

        crate::ui::modals::render_modals(self, &ctx);
        // Modal button actions can request app close during rendering.
        self.close_after_monitor_prompt(&ctx);
    }

    fn on_exit(&mut self) {
        self.cleanup_on_exit();
    }
}
