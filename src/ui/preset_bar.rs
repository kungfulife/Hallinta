use crate::app::HallintaApp;
use crate::models::{ConfirmAction, InputAction, Modal};
use eframe::egui;

pub fn render_preset_bar(app: &mut HallintaApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Preset:").strong());

        let is_locked = app.save_monitor.is_running();
        let prev_selected = app.selected_preset.clone();

        // Sorted preset list (Default first)
        let mut preset_names: Vec<String> = app.presets.keys().cloned().collect();
        preset_names.sort_by(|a, b| {
            if a == "Default" {
                std::cmp::Ordering::Less
            } else if b == "Default" {
                std::cmp::Ordering::Greater
            } else {
                a.to_lowercase().cmp(&b.to_lowercase())
            }
        });

        egui::ComboBox::from_id_salt("preset_dropdown")
            .selected_text(&app.selected_preset)
            .width(200.0)
            .show_ui(ui, |ui| {
                // "Create New Preset" option
                if ui
                    .selectable_label(false, "Create New Preset")
                    .clicked()
                    && !is_locked
                {
                    let default_name =
                        format!("Preset {}", app.presets.len() + 1);
                    app.active_modal = Some(Modal::Input {
                        title: "Enter name for new preset:".to_string(),
                        value: default_name,
                        action: InputAction::CreatePreset,
                    });
                }
                ui.separator();
                for name in &preset_names {
                    if ui
                        .selectable_label(*name == app.selected_preset, name)
                        .clicked()
                        && !is_locked
                    {
                        app.selected_preset = name.clone();
                    }
                }
            });

        // If preset changed, switch to it
        if app.selected_preset != prev_selected {
            app.switch_preset();
        }

        // Rename button (not for Default, not when locked)
        let can_modify = app.selected_preset != "Default" && !is_locked;
        ui.add_enabled_ui(can_modify, |ui| {
            if ui.button("Rename").clicked() {
                app.active_modal = Some(Modal::Input {
                    title: format!("Enter new name for \"{}\":", app.selected_preset),
                    value: app.selected_preset.clone(),
                    action: InputAction::RenamePreset,
                });
            }
        });

        // Delete button
        ui.add_enabled_ui(can_modify, |ui| {
            if ui.button("Delete").clicked() {
                app.active_modal = Some(Modal::Confirm {
                    message: format!(
                        "Are you sure you want to delete the preset \"{}\"?",
                        app.selected_preset
                    ),
                    confirm_text: "Delete".to_string(),
                    cancel_text: "Cancel".to_string(),
                    action: ConfirmAction::DeletePreset,
                    cancel_action: None,
                });
            }
        });

        // Mod count
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let total = app.current_mods.len();
            let enabled = app.current_mods.iter().filter(|m| m.enabled).count();
            ui.label(format!("{}/{} mods enabled", enabled, total));
        });
    });
}
