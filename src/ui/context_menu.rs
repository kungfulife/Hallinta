use crate::app::HallintaApp;
use crate::models::{ConfirmAction, InputAction, Modal};
use eframe::egui;

pub fn render_context_menu(app: &mut HallintaApp, ui: &mut egui::Ui, mod_index: usize) {
    // Bounds-check: list may have mutated between menu open and render
    if mod_index >= app.current_mods.len() {
        ui.label(egui::RichText::new("(mod no longer in list)").italics());
        return;
    }
    let mod_entry = &app.current_mods[mod_index];
    let is_workshop = mod_entry.workshop_id != "0" && !mod_entry.workshop_id.is_empty();
    let workshop_id = mod_entry.workshop_id.clone();
    let mod_name = mod_entry.name.clone();

    // Toggle enabled
    let toggle_label = if mod_entry.enabled {
        "Disable"
    } else {
        "Enable"
    };
    if ui.button(toggle_label).clicked() {
        let new_state = !app.current_mods[mod_index].enabled;
        app.current_mods[mod_index].enabled = new_state;
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "{} mod: {}",
                if new_state { "Enabled" } else { "Disabled" },
                mod_name
            ),
            "ModManager",
        );
        app.save_mod_config_and_preset();
        ui.close();
    }

    ui.separator();

    if ui.button("Move to position...").clicked() {
        app.active_modal = Some(Modal::Input {
            title: format!("Move \"{}\" to position:", mod_name),
            value: (mod_index + 1).to_string(),
            hint: String::new(),
            action: InputAction::MoveModToPosition(mod_index),
        });
        ui.close();
    }

    if ui.button("Delete mod").clicked() {
        app.active_modal = Some(Modal::Confirm {
            message: format!("Delete mod \"{}\"?", mod_name),
            confirm_text: "Delete".to_string(),
            cancel_text: "Cancel".to_string(),
            action: ConfirmAction::DeleteMod(mod_index, mod_name.clone(), workshop_id.clone()),
            cancel_action: None,
            dismissable: false,
        });
        ui.close();
    }

    ui.separator();

    if is_workshop {
        if ui.button("Open Workshop Page").clicked() {
            crate::core::workshop::open_workshop_page(&workshop_id);
            ui.close();
        }
        if ui.button("Copy Workshop ID").clicked() {
            ui.ctx().copy_text(workshop_id.clone());
            ui.close();
        }
        if ui.button("Copy Workshop URL").clicked() {
            let url = format!(
                "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
                workshop_id
            );
            ui.ctx().copy_text(url);
            ui.close();
        }
    }

    if ui.button("Copy Mod Name").clicked() {
        ui.ctx().copy_text(mod_name.clone());
        ui.close();
    }

    if ui.button("Open mod_config.xml").clicked() {
        app.open_mod_config_file();
        ui.close();
    }
}
