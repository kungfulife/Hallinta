use crate::app::HallintaApp;
use crate::models::{ExternalModChangeSummary, ModEntry, Modal, NoitaSyncState};
use eframe::egui;

pub(super) fn render_reconciliation(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    file_mods: Vec<ModEntry>,
    summary: &ExternalModChangeSummary,
    error: Option<String>,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut apply_selected = false;
    let mut create_preset = false;

    egui::Window::new("Noita Configuration Detected")
        .collapsible(false)
        .resizable(false)
        .default_width(460.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label(
                "Hallinta found mod_config.xml after configuration-only editing. Choose which setup to continue with.",
            );
            if let Some(error) = &error {
                ui.add_space(d.sm);
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.add_space(d.md);

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(d.sm as i8))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Use selected preset").strong());
                    ui.label(format!(
                        "Replace Noita's file with \"{}\" ({} mods, {} enabled).",
                        app.selected_preset, summary.current_total, summary.current_enabled
                    ));
                    if ui.button("Apply Preset to Noita").clicked() {
                        apply_selected = true;
                    }
                });
            ui.add_space(d.sm);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(d.sm as i8))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Keep detected Noita setup").strong());
                    ui.label(format!(
                        "Leave Noita unchanged and save its {} mods ({} enabled) as a new preset.",
                        summary.disk_total, summary.disk_enabled
                    ));
                    if ui.button("Save as New Preset").clicked() {
                        create_preset = true;
                    }
                });
        });

    if apply_selected {
        if let Err(error) = app.apply_selected_preset_to_noita()
            && app.noita_sync_state == NoitaSyncState::ReconciliationPending
        {
            app.active_modal = Some(Modal::NoitaReconciliation {
                file_mods,
                summary: summary.clone(),
                error: Some(error),
            });
        }
    } else if create_preset {
        app.active_modal = Some(Modal::DetectedNoitaPresetName {
            file_mods,
            value: app.default_detected_preset_name(),
            error: None,
        });
    } else {
        app.active_modal = Some(Modal::NoitaReconciliation {
            file_mods,
            summary: summary.clone(),
            error,
        });
    }
}

pub(super) fn render_preset_name(
    app: &mut HallintaApp,
    ctx: &egui::Context,
    file_mods: Vec<ModEntry>,
    mut value: String,
    error: Option<String>,
) {
    let d = crate::ui::design::Design::new(ctx, &app.settings);
    let mut save = false;
    let mut back = false;

    egui::Window::new("Save Detected Setup")
        .collapsible(false)
        .resizable(false)
        .default_width(380.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label("Name the new preset created from Noita's current mod_config.xml.");
            ui.add_space(d.sm);
            let response = ui.add(egui::TextEdit::singleline(&mut value).desired_width(330.0));
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                save = true;
            }
            if let Some(error) = &error {
                ui.add_space(d.xs);
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            ui.add_space(d.sm);
            ui.horizontal(|ui| {
                if ui.button("Save Preset").clicked() {
                    save = true;
                }
                if ui.button("Back").clicked() {
                    back = true;
                }
            });
        });

    if save {
        if let Err(error) = app.save_detected_noita_as_preset(&value)
            && app.noita_sync_state == NoitaSyncState::ReconciliationPending
        {
            app.active_modal = Some(Modal::DetectedNoitaPresetName {
                file_mods,
                value,
                error: Some(error),
            });
        }
    } else if back {
        app.show_noita_reconciliation(file_mods);
    } else {
        app.active_modal = Some(Modal::DetectedNoitaPresetName {
            file_mods,
            value,
            error,
        });
    }
}
