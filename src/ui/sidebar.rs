use crate::app::HallintaApp;
use eframe::egui;

pub fn render_sidebar(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
    egui::Panel::right("sidebar_panel")
        .resizable(false)
        .default_size(d.sidebar_w)
        .min_size(d.sidebar_w)
        .max_size(d.sidebar_w)
        .show(ui, |ui| {
            ui.add_space(d.md);
            let btn_width = (ui.available_width() - d.md * 2.0).max(1.0);
            let pair_width = ((btn_width - d.xs) / 2.0).max(1.0);
            let button_height = ui.spacing().interact_size.y;
            let button_size = [btn_width, button_height];
            let pair_size = [pair_width, button_height];
            let is_locked = app.save_monitor.is_running();
            let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

            ui.label(egui::RichText::new("Actions").size(d.font_heading).strong());
            ui.add_space(d.md);

            section_label(ui, "Session & Safety", &d);
            if is_locked {
                if ui
                    .add_sized(button_size, egui::Button::new("Stop Monitor"))
                    .on_hover_text("Stop monitoring; the session remains available to resume")
                    .clicked()
                {
                    app.stop_save_monitor();
                }
                ui.colored_label(
                    d.status_ok,
                    egui::RichText::new("Monitoring").small().strong(),
                );
                if let Some(session) = &app.save_monitor.current_session {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&session.name)
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        )
                        .truncate(),
                    )
                    .on_hover_text(format!("Current session: {}", session.name));
                }
            } else {
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(button_size, egui::Button::new("Start Monitor"))
                        .on_hover_text("Start auto-snapshotting Noita saves")
                        .clicked()
                    {
                        app.start_save_monitor();
                    }
                });
            }

            ui.add_enabled_ui(app.can_start_manual_backup(), |ui| {
                if ui
                    .add_sized(button_size, egui::Button::new("Create Backup"))
                    .on_hover_text("Name and create a manual backup (Ctrl+B)")
                    .clicked()
                {
                    app.start_backup_modal();
                }
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d.xs;
                if ui
                    .add_sized(pair_size, egui::Button::new("Backups"))
                    .on_hover_text("Manage, restore, and delete manual backups")
                    .clicked()
                {
                    app.open_backup_manager();
                }
                if ui
                    .add_sized(pair_size, egui::Button::new("Sessions"))
                    .on_hover_text("Manage monitor sessions and restore snapshots")
                    .clicked()
                {
                    app.open_sessions_manager();
                }
            });

            ui.add_space(d.md);
            ui.separator();
            ui.add_space(d.md);

            section_label(ui, "Mod Files", &d);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d.xs;
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Import"))
                        .on_hover_text("Import a mod list and apply it")
                        .clicked()
                    {
                        app.import_mod_list();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Export"))
                        .on_hover_text("Export the currently enabled mods")
                        .clicked()
                    {
                        app.export_mod_list();
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d.xs;
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Open XML"))
                        .on_hover_text("Open the live mod_config.xml in the default editor")
                        .clicked()
                    {
                        app.open_mod_config_file();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Reload"))
                        .on_hover_text("Reload mod_config.xml from disk (F5)")
                        .clicked()
                    {
                        app.reload_mods_explicit();
                    }
                });
            });

            ui.add_space(d.md);
            section_label(ui, "Presets", &d);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = d.xs;
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Import"))
                        .on_hover_text("Import presets from a Hallinta preset file")
                        .clicked()
                    {
                        app.import_presets();
                    }
                });
                ui.add_enabled_ui(app.can_export_presets(), |ui| {
                    if ui
                        .add_sized(pair_size, egui::Button::new("Export"))
                        .on_hover_text("Export one or more presets")
                        .clicked()
                    {
                        app.start_export_presets();
                    }
                });
            });
        });
}

fn section_label(ui: &mut egui::Ui, label: &str, d: &crate::ui::design::Design) {
    ui.label(
        egui::RichText::new(label)
            .size(d.font_small)
            .strong()
            .color(ui.visuals().weak_text_color()),
    );
    ui.add_space(d.xs);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::test_app;

    fn text_from_shape(shape: &egui::epaint::Shape) -> Vec<String> {
        match shape {
            egui::epaint::Shape::Text(text) => vec![text.galley.text().to_string()],
            egui::epaint::Shape::Vec(shapes) => shapes.iter().flat_map(text_from_shape).collect(),
            _ => Vec::new(),
        }
    }

    fn positioned_text_from_shape(shape: &egui::epaint::Shape) -> Vec<(String, f32)> {
        match shape {
            egui::epaint::Shape::Text(text) => {
                vec![(text.galley.text().to_string(), text.pos.y)]
            }
            egui::epaint::Shape::Vec(shapes) => {
                shapes.iter().flat_map(positioned_text_from_shape).collect()
            }
            _ => Vec::new(),
        }
    }

    fn dense_sidebar_text_positions() -> Vec<(String, f32)> {
        let (_runtime, mut app) = test_app(Vec::new());
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| render_sidebar(&mut app, ui));
        });
        output
            .shapes
            .iter()
            .flat_map(|shape| positioned_text_from_shape(&shape.shape))
            .collect()
    }

    #[test]
    fn normal_sidebar_uses_dense_control_console() {
        let (_runtime, mut app) = test_app(Vec::new());
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| render_sidebar(&mut app, ui));
        });
        let labels: Vec<String> = output
            .shapes
            .iter()
            .flat_map(|shape| text_from_shape(&shape.shape))
            .collect();

        for expected in [
            "Session & Safety",
            "Start Monitor",
            "Create Backup",
            "Backups",
            "Sessions",
            "Mod Files",
            "Import",
            "Export",
            "Open XML",
            "Reload",
            "Presets",
        ] {
            assert!(
                labels.iter().any(|label| label == expected),
                "missing {expected}; labels: {labels:?}"
            );
        }
        for removed in [
            "Restore Latest",
            "Restore Backup",
            "Create Manual Backup",
            "View Sessions",
            "Clear All Data",
        ] {
            assert!(
                labels.iter().all(|label| label != removed),
                "old action still rendered: {removed}"
            );
        }
    }

    #[test]
    fn dense_sidebar_pair_rows_share_vertical_baseline() {
        let positioned = dense_sidebar_text_positions();
        let positions = |label: &str| {
            positioned
                .iter()
                .filter_map(|(text, y)| (text == label).then_some(*y))
                .collect::<Vec<_>>()
        };

        for (left, right) in [
            (positions("Backups"), positions("Sessions")),
            (positions("Import"), positions("Export")),
            (positions("Open XML"), positions("Reload")),
        ] {
            assert_eq!(left.len(), right.len());
            for (left_y, right_y) in left.into_iter().zip(right) {
                assert!(
                    (left_y - right_y).abs() < 0.1,
                    "pair is vertically misaligned: left={left_y}, right={right_y}; {positioned:?}"
                );
            }
        }
    }
}
