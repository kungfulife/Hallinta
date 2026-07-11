use crate::app::HallintaApp;
use crate::models::{UpdatePhase, UpdateStatus};
use eframe::egui;

pub fn render(app: &mut HallintaApp, ctx: &egui::Context) {
    let status = app.update_state.status.clone();
    match status {
        UpdateStatus::Available(info) => {
            egui::Window::new(format!("Hallinta v{} is available", info.version))
                .collapsible(false)
                .resizable(true)
                .default_width(470.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Published through the official Hallinta GitHub release.");
                    if !info.notes.trim().is_empty() {
                        ui.separator();
                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                            ui.label(&info.notes);
                        });
                    }
                    ui.separator();
                    ui.label("Hallinta will finish any active Save Monitor snapshot, verify the signed official release, install it, and restart.");
                    ui.horizontal(|ui| {
                        if ui.button("Update & Restart").clicked() {
                            app.begin_update(info.clone());
                        }
                        if ui.button("Later").clicked() {
                            app.dismiss_update_status();
                        }
                    });
                });
        }
        UpdateStatus::Checking { manual: true } => {
            egui::Window::new("Checking for updates")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.spinner();
                    ui.label("Contacting GitHub Releases…");
                });
        }
        UpdateStatus::Failed { message, retryable } => {
            egui::Window::new("Hallinta Update")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.horizontal(|ui| {
                        if retryable && ui.button("Retry").clicked() {
                            app.check_for_updates(true);
                        }
                        if ui.button("Close").clicked() {
                            app.dismiss_update_status();
                        }
                    });
                });
        }
        UpdateStatus::Running { phase, message } => render_lock(ctx, phase, &message),
        UpdateStatus::Idle | UpdateStatus::Checking { manual: false } => {}
    }
}

fn render_lock(ctx: &egui::Context, phase: UpdatePhase, message: &str) {
    let rect = ctx.content_rect();
    egui::Area::new(egui::Id::new("update_input_blocker"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(
                egui::Rect::from_min_size(egui::Pos2::ZERO, rect.size()),
                egui::Sense::click_and_drag(),
            );
            ui.painter()
                .rect_filled(response.rect, 0.0, egui::Color32::from_black_alpha(190));
        });
    egui::Window::new("Updating Hallinta")
        .id(egui::Id::new("update_progress_window"))
        .order(egui::Order::Tooltip)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(410.0);
            ui.heading(phase_label(phase));
            ui.label(message);
            ui.spinner();
            ui.label("Controls are temporarily disabled to protect application data.");
        });
}

fn phase_label(phase: UpdatePhase) -> &'static str {
    match phase {
        UpdatePhase::Preparing => "Preparing",
        UpdatePhase::Snapshotting => "Protecting monitor data",
        UpdatePhase::Installing => "Installing signed update",
        UpdatePhase::Restarting => "Restarting",
    }
}
