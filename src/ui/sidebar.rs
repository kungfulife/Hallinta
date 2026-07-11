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
            // Side panel frame already provides horizontal margin — use the full
            // content width so pair halves stay wide enough for "Open XML".
            // Shrinking further made the left half overflow and shift the right column.
            let btn_width = ui.available_width().max(1.0);
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
                ui.add_enabled_ui(app.can_start_save_monitor(), |ui| {
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

            pair_row(ui, d.xs, pair_size, |ui, size| {
                if ui
                    .add_sized(size, egui::Button::new("Backups"))
                    .on_hover_text("Manage, restore, and delete manual backups")
                    .clicked()
                {
                    app.open_backup_manager();
                }
                if ui
                    .add_sized(size, egui::Button::new("Sessions"))
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
            pair_row(ui, d.xs, pair_size, |ui, size| {
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Import"))
                        .on_hover_text("Import a mod list and apply it")
                        .clicked()
                    {
                        app.import_mod_list();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Export"))
                        .on_hover_text("Export the currently enabled mods")
                        .clicked()
                    {
                        app.export_mod_list();
                    }
                });
            });
            pair_row(ui, d.xs, pair_size, |ui, size| {
                ui.add_enabled_ui(!backup_busy && app.is_noita_sync_live(), |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Open XML"))
                        .on_hover_text("Open the live mod_config.xml in the default editor")
                        .clicked()
                    {
                        app.open_mod_config_file();
                    }
                });
                ui.add_enabled_ui(!backup_busy, |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Reload"))
                        .on_hover_text("Reload mod_config.xml from disk (F5)")
                        .clicked()
                    {
                        app.reload_mods_explicit();
                    }
                });
            });

            ui.add_space(d.md);
            section_label(ui, "Presets", &d);
            pair_row(ui, d.xs, pair_size, |ui, size| {
                ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Import"))
                        .on_hover_text("Import presets from a Hallinta preset file")
                        .clicked()
                    {
                        app.import_presets();
                    }
                });
                ui.add_enabled_ui(app.can_export_presets(), |ui| {
                    if ui
                        .add_sized(size, egui::Button::new("Export"))
                        .on_hover_text("Export one or more presets")
                        .clicked()
                    {
                        app.start_export_presets();
                    }
                });
            });

            #[cfg(debug_assertions)]
            {
                ui.add_space(d.md);
                ui.separator();
                ui.add_space(d.md);
                section_label(ui, "Preview", &d);
                let preview_label = if app.preview_noita_directory_warning {
                    "End Warning Preview"
                } else {
                    "Preview Noita Warning"
                };
                if ui
                    .add_sized(button_size, egui::Button::new(preview_label))
                    .on_hover_text(
                        "Temporarily simulate an invalid Noita directory without changing Settings",
                    )
                    .clicked()
                {
                    app.toggle_noita_warning_preview();
                }
            }
        });
}

/// Two equal-width action buttons on one row.
fn pair_row(
    ui: &mut egui::Ui,
    gap: f32,
    pair_size: [f32; 2],
    add_contents: impl FnOnce(&mut egui::Ui, [f32; 2]),
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        add_contents(ui, pair_size);
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

    /// (label, text center x, text top y) — centers equal button centers when labels are centered.
    fn positioned_text_from_shape(shape: &egui::epaint::Shape) -> Vec<(String, f32, f32)> {
        match shape {
            egui::epaint::Shape::Text(text) => {
                let center_x = text.pos.x + text.galley.size().x * 0.5;
                vec![(text.galley.text().to_string(), center_x, text.pos.y)]
            }
            egui::epaint::Shape::Vec(shapes) => {
                shapes.iter().flat_map(positioned_text_from_shape).collect()
            }
            _ => Vec::new(),
        }
    }

    fn dense_sidebar_text_positions() -> Vec<(String, f32, f32)> {
        let (_runtime, mut app) = test_app(Vec::new());
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            ui.set_width(900.0);
            ui.set_height(700.0);
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
            "Preview Noita Warning",
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
        let ys = |label: &str| {
            positioned
                .iter()
                .filter_map(|(text, _, y)| (text == label).then_some(*y))
                .collect::<Vec<_>>()
        };

        for (left, right) in [
            (ys("Backups"), ys("Sessions")),
            (ys("Import"), ys("Export")),
            (ys("Open XML"), ys("Reload")),
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

    #[test]
    fn dense_sidebar_pair_columns_share_horizontal_centers() {
        let positioned = dense_sidebar_text_positions();
        let centers = |label: &str| {
            positioned
                .iter()
                .filter_map(|(text, cx, _)| (text == label).then_some(*cx))
                .collect::<Vec<_>>()
        };

        let left_refs = centers("Backups");
        let right_refs = centers("Sessions");
        assert_eq!(left_refs.len(), 1);
        assert_eq!(right_refs.len(), 1);
        let left_ref = left_refs[0];
        let right_ref = right_refs[0];

        // Longer labels (especially "Open XML") used to overflow the half-width slot
        // and shift their column centers away from shorter pair rows.
        for label in ["Import", "Open XML"] {
            for cx in centers(label) {
                assert!(
                    (cx - left_ref).abs() < 0.5,
                    "{label} left-column center {cx} != {left_ref}; {positioned:?}"
                );
            }
        }
        for label in ["Export", "Reload"] {
            for cx in centers(label) {
                assert!(
                    (cx - right_ref).abs() < 0.5,
                    "{label} right-column center {cx} != {right_ref}; {positioned:?}"
                );
            }
        }
    }
}
