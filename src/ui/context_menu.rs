use crate::app::HallintaApp;
use crate::models::{ConfirmAction, InputAction, Modal};
use eframe::egui;

pub fn render_context_menu(app: &mut HallintaApp, ui: &mut egui::Ui, mod_index: usize) {
    let is_locked = app.save_monitor.is_running();
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
    if ui.button(toggle_label).clicked() && !is_locked {
        app.current_mods[mod_index].enabled = !app.current_mods[mod_index].enabled;
        app.save_mod_config_and_preset();
        ui.close();
    }

    ui.separator();

    if ui.button("Move to position...").clicked() && !is_locked {
        app.active_modal = Some(Modal::Input {
            title: format!("Move \"{}\" to position:", mod_name),
            value: (mod_index + 1).to_string(),
            action: InputAction::MoveModToPosition(mod_index),
        });
        ui.close();
    }

    if ui.button("Delete mod").clicked() && !is_locked {
        app.active_modal = Some(Modal::Confirm {
            message: format!("Delete mod \"{}\"?", mod_name),
            confirm_text: "Delete".to_string(),
            cancel_text: "Cancel".to_string(),
            action: ConfirmAction::DeleteMod(mod_index),
            cancel_action: None,
        });
        ui.close();
    }

    ui.separator();

    if is_workshop {
        if ui.button("Open Workshop Page").clicked() {
            let url = format!(
                "https://steamcommunity.com/sharedfiles/filedetails/?id={}",
                workshop_id
            );
            let _ = crate::core::platform::open_url(&url);
            ui.close();
        }
    }

    if ui.button("Open mod_config.xml").clicked() {
        app.open_mod_config_file();
        ui.close();
    }
}
