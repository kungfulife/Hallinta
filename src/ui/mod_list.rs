use crate::app::HallintaApp;
use crate::models::{DragState, FilterMode};
use eframe::egui;

pub fn render_mod_list(app: &mut HallintaApp, ui: &mut egui::Ui) {
    if app.current_mods.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No mods loaded. Check your Noita save directory in Settings.")
                    .size(14.0)
                    .italics(),
            );
        });
        return;
    }

    let is_locked = app.save_monitor.is_running();
    let search_lower = app.search_query.to_lowercase();
    let filter = app.filter_mode;

    // Build filtered list of indices
    let filtered_indices: Vec<usize> = app
        .current_mods
        .iter()
        .enumerate()
        .filter(|(_, m)| match filter {
            FilterMode::All => true,
            FilterMode::Enabled => m.enabled,
            FilterMode::Disabled => !m.enabled,
        })
        .filter(|(_, m)| {
            if search_lower.is_empty() {
                true
            } else {
                m.name.to_lowercase().contains(&search_lower)
            }
        })
        .map(|(i, _)| i)
        .collect();

    // Snapshot data needed for rendering (avoid borrow conflicts)
    let mod_snapshots: Vec<(usize, String, bool, bool, Option<bool>)> = filtered_indices
        .iter()
        .map(|&idx| {
            let m = &app.current_mods[idx];
            let is_workshop = m.workshop_id != "0" && !m.workshop_id.is_empty();
            let installed = app.is_workshop_mod_installed(&m.workshop_id);
            (idx, m.name.clone(), m.enabled, is_workshop, installed)
        })
        .collect();

    let drag_source = app.drag_state.as_ref().map(|d| d.current_index);
    let mut swap: Option<(usize, usize)> = None;
    let mut toggle_idx: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            for (row_num, &(idx, ref name, enabled, is_workshop, installed)) in
                mod_snapshots.iter().enumerate()
            {
                let is_drag_source = drag_source == Some(idx);
                let is_even = row_num % 2 == 0;

                // Row background
                let base_fill = if is_drag_source {
                    ui.visuals().selection.bg_fill.linear_multiply(0.3)
                } else if enabled {
                    if is_even {
                        ui.visuals().widgets.active.bg_fill.linear_multiply(0.08)
                    } else {
                        ui.visuals().widgets.active.bg_fill.linear_multiply(0.04)
                    }
                } else if is_even {
                    ui.visuals().faint_bg_color
                } else {
                    egui::Color32::TRANSPARENT
                };

                let row_response = egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(6, 3))
                    .corner_radius(2)
                    .fill(base_fill)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            // Drag handle
                            if !is_locked {
                                let handle_text =
                                    egui::RichText::new("\u{2261}") // ≡
                                        .size(18.0)
                                        .color(ui.visuals().weak_text_color());
                                let handle = ui
                                    .add(egui::Label::new(handle_text).sense(egui::Sense::drag()));

                                if handle.drag_started() && app.drag_state.is_none() {
                                    app.drag_state = Some(DragState {
                                        current_index: idx,
                                        pre_drag_snapshot: app.current_mods.clone(),
                                    });
                                }

                                if handle.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                }
                                if handle.dragged() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    if let Some(ref drag) = app.drag_state {
                                        if drag.current_index != idx {
                                            swap = Some((drag.current_index, idx));
                                        }
                                    }
                                }
                            } else {
                                // Spacer when locked
                                ui.add_space(20.0);
                            }

                            ui.add_space(4.0);

                            // Enabled checkbox
                            let mut cb_enabled = enabled;
                            if ui.checkbox(&mut cb_enabled, "").changed() && !is_locked {
                                toggle_idx = Some(idx);
                            }

                            ui.add_space(2.0);

                            // Mod name with styling
                            let name_text = if enabled {
                                egui::RichText::new(name).size(13.0)
                            } else {
                                egui::RichText::new(name)
                                    .size(13.0)
                                    .color(ui.visuals().weak_text_color())
                            };
                            let text_response = ui.add(
                                egui::Label::new(name_text).sense(egui::Sense::click()),
                            );

                            // Badges
                            if is_workshop {
                                let badge_color = egui::Color32::from_rgb(70, 130, 180);
                                ui.label(
                                    egui::RichText::new("[W]")
                                        .small()
                                        .color(badge_color)
                                        .strong(),
                                );
                            }

                            // Missing mod indicator
                            if let Some(false) = installed {
                                ui.label(
                                    egui::RichText::new("[Missing]")
                                        .small()
                                        .color(egui::Color32::from_rgb(220, 60, 60))
                                        .strong(),
                                );
                            }

                            // Left-click toggles
                            if text_response.clicked() && !is_locked && app.drag_state.is_none() {
                                toggle_idx = Some(idx);
                            }

                            // Context menu
                            text_response.context_menu(|ui| {
                                crate::ui::context_menu::render_context_menu(app, ui, idx);
                            });
                        });
                    });

                // Drop target indicator: show a line between items during drag
                if app.drag_state.is_some() && !is_drag_source {
                    let rect = row_response.response.rect;
                    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                        if rect.contains(pointer_pos) {
                            let line_y = if pointer_pos.y < rect.center().y {
                                rect.top()
                            } else {
                                rect.bottom()
                            };
                            let stroke = egui::Stroke::new(2.0, ui.visuals().selection.bg_fill);
                            ui.painter().line_segment(
                                [
                                    egui::pos2(rect.left(), line_y),
                                    egui::pos2(rect.right(), line_y),
                                ],
                                stroke,
                            );
                        }
                    }
                }
            }
        });

    // Apply toggle
    if let Some(idx) = toggle_idx {
        app.current_mods[idx].enabled = !app.current_mods[idx].enabled;
        app.save_mod_config_and_preset();
    }

    // Process swap from drag
    if let Some((from, to)) = swap {
        if from < app.current_mods.len() && to < app.current_mods.len() {
            let item = app.current_mods.remove(from);
            app.current_mods.insert(to, item);
            if let Some(ref mut drag) = app.drag_state {
                drag.current_index = to;
            }
        }
    }

    // Handle drag end
    if ui.input(|i| i.pointer.any_released()) && app.drag_state.is_some() {
        app.drag_state = None;
        app.save_mod_config_and_preset();
    }

    // Escape cancels drag
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if let Some(drag) = app.drag_state.take() {
            app.current_mods = drag.pre_drag_snapshot;
        }
    }
}

/// Shown when the save monitor is active, blocking mod list / vault access.
pub fn render_monitor_active(app: &mut HallintaApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);

        ui.label(
            egui::RichText::new("Save Monitor Active")
                .heading()
                .strong()
                .size(22.0),
        );
        ui.add_space(12.0);

        ui.colored_label(
            egui::Color32::from_rgb(50, 200, 50),
            egui::RichText::new("Running").size(16.0).strong(),
        );

        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(format!(
                "Preset: {}",
                app.selected_preset
            ))
            .size(14.0),
        );
        ui.label(
            egui::RichText::new(format!(
                "Snapshots taken: {}",
                app.save_monitor.snapshot_count
            ))
            .size(14.0),
        );
        ui.label(
            egui::RichText::new(format!(
                "Interval: {} min | Max: {} | Keep every {}th",
                app.settings.save_monitor_settings.interval_minutes,
                app.settings.save_monitor_settings.max_snapshots_per_preset,
                app.settings.save_monitor_settings.keep_every_nth,
            ))
            .size(12.0)
            .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(20.0);

        ui.label(
            egui::RichText::new("Mod list and preset vault are locked while monitor is running.")
                .italics()
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(16.0);

        if ui
            .button(egui::RichText::new("Stop Monitor").size(14.0))
            .clicked()
        {
            app.stop_save_monitor();
        }
    });
}
