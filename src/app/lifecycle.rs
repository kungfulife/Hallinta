use super::HallintaApp;
use crate::core::{logging, platform};
use crate::models::View;
use eframe::egui;

impl HallintaApp {
    // ── Cleanup ────────────────────────────────────────────────────────

    pub fn cleanup_on_exit(&mut self) {
        let _ = logging::log("INFO", "Application shutting down", "App");

        // Dev mode: verify real directories are untouched
        if cfg!(debug_assertions) {
            match platform::restore_real_dirs_from_dev() {
                Ok(msg) => {
                    let _ = logging::log("INFO", &format!("[DEV] Exit: {}", msg), "DevData");
                }
                Err(e) => {
                    let _ = logging::log(
                        "WARN",
                        &format!("[DEV] Exit restore error: {}", e),
                        "DevData",
                    );
                }
            }
        }

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

        // 0b. Apply deferred min size (queued on previous frame to avoid one-behind lag)
        if let Some(min) = self.deferred_min_size.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(min));
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

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_view {
                View::ModList => {
                    if self.compact_mode {
                        crate::ui::compact::render_compact(self, ui);
                    } else if self.save_monitor.is_running() {
                        // Monitor running: show monitor status instead of mod list
                        crate::ui::mod_list::render_monitor_active(self, ui);
                    } else {
                        crate::ui::mod_list::render_mod_list(self, ui);
                    }
                }
                View::Settings => {
                    crate::ui::settings::render_settings(self, ui);
                }
            }
        });

        // 6. Render modals on top
        crate::ui::modals::render_modals(self, ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cleanup_on_exit();
    }
}
