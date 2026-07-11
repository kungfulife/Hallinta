use crate::app::HallintaApp;
use crate::ui::design::Design;
use eframe::egui;

pub fn render_compact(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = Design::new(ui.ctx(), &app.settings);

    // Use vertical_centered with a max width so everything aligns nicely
    let available = ui.available_size();
    let top_pad = (available.y * 0.08).clamp(d.sm, d.lg);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(top_pad);

            ui.vertical_centered(|ui| {
                let content_width = (available.x - d.lg * 2.0).clamp(240.0, 520.0);
                ui.set_width(content_width);

                // ── Preset selector ──────────────────────────────────────────────────
                ui.label(
                    egui::RichText::new("Preset")
                        .size(d.font_small)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(d.xs);

                let is_locked = app.save_monitor.is_running();
                let prev_selected = app.selected_preset.clone();

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

                ui.add_enabled_ui(!is_locked, |ui| {
                    egui::ComboBox::from_id_salt("compact_preset_selector")
                        .selected_text(&app.selected_preset)
                        .width(content_width)
                        .show_ui(ui, |ui| {
                            for name in &preset_names {
                                if ui
                                    .selectable_label(*name == app.selected_preset, name)
                                    .clicked()
                                {
                                    app.selected_preset = name.clone();
                                }
                            }
                        });
                });

                if app.selected_preset != prev_selected {
                    app.switch_preset();
                }

                ui.add_space(d.sm);

                // Mod count summary
                let total = app.current_mods.len();
                let enabled_count = app.current_mods.iter().filter(|m| m.enabled).count();
                ui.label(
                    egui::RichText::new(format!("{} / {} mods enabled", enabled_count, total))
                        .size(d.font_body)
                        .color(ui.visuals().weak_text_color()),
                );

                ui.add_space(d.lg);
                ui.separator();
                ui.add_space(d.lg);

                let btn_w = content_width;
                let btn_h = 28.0;

                // ── Monitor button ───────────────────────────────────────────────────
                if is_locked {
                    ui.colored_label(
                        d.status_ok,
                        egui::RichText::new("● MONITORING")
                            .size(d.font_body)
                            .strong(),
                    );
                    if let Some(ref session) = app.save_monitor.current_session {
                        ui.label(
                            egui::RichText::new(format!("Session: {}", session.name))
                                .size(d.font_small)
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                    ui.add_space(d.sm);
                    if ui
                        .add_sized(
                            [btn_w, btn_h],
                            egui::Button::new(
                                egui::RichText::new("Stop Monitor").size(d.font_body),
                            ),
                        )
                        .clicked()
                    {
                        app.stop_save_monitor();
                    }
                } else {
                    if ui
                        .add_sized(
                            [btn_w, btn_h],
                            egui::Button::new(
                                egui::RichText::new("Start Monitor").size(d.font_body),
                            ),
                        )
                        .clicked()
                    {
                        app.start_save_monitor();
                    }
                }

                ui.add_space(d.sm);

                // ── Secondary actions ────────────────────────────────────────────────
                ui.add_enabled_ui(app.can_start_manual_backup(), |ui| {
                    if ui
                        .add_sized(
                            [btn_w, btn_h],
                            egui::Button::new(
                                egui::RichText::new("Create Backup").size(d.font_body),
                            ),
                        )
                        .clicked()
                    {
                        app.start_backup_modal();
                    }
                });

                ui.add_space(d.xs);

                if ui
                    .add_sized(
                        [btn_w, btn_h],
                        egui::Button::new(egui::RichText::new("Manage Backups").size(d.font_body)),
                    )
                    .clicked()
                {
                    app.open_backup_manager();
                }

                ui.add_space(d.xs);

                if ui
                    .add_sized(
                        [btn_w, btn_h],
                        egui::Button::new(egui::RichText::new("Manage Sessions").size(d.font_body)),
                    )
                    .clicked()
                {
                    app.open_sessions_manager();
                }
            });
        });
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

    #[test]
    fn compact_mode_exposes_four_approved_actions() {
        let (_runtime, mut app) = test_app(Vec::new());
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| render_compact(&mut app, ui));
        });
        let labels: Vec<String> = output
            .shapes
            .iter()
            .flat_map(|shape| text_from_shape(&shape.shape))
            .collect();

        for expected in [
            "Start Monitor",
            "Create Backup",
            "Manage Backups",
            "Manage Sessions",
        ] {
            assert!(
                labels.iter().any(|label| label == expected),
                "missing {expected}"
            );
        }
        for removed in ["Restore Latest", "Restore Backup", "View Sessions"] {
            assert!(
                labels.iter().all(|label| label != removed),
                "old action still rendered: {removed}"
            );
        }
    }
}
