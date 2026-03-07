use crate::app::HallintaApp;
use crate::models::{DragState, FilterMode};
use eframe::egui;

pub fn render_mod_list(app: &mut HallintaApp, ui: &mut egui::Ui) {
    if app.current_mods.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "No mods loaded. Check your Noita save directory in Settings.",
                )
                .size(14.0)
                .italics(),
            );
        });
        return;
    }

    let is_locked = app.save_monitor.is_running();
    let search_lower = app.search_query.to_lowercase();
    let filter = app.filter_mode;

    // Drag-to-reorder only works on the unfiltered list to avoid index confusion.
    let can_drag = !is_locked && search_lower.is_empty() && filter == FilterMode::All;

    // Build the visible subset: all indices that pass the current filter/search.
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
            search_lower.is_empty() || m.name.to_lowercase().contains(&search_lower)
        })
        .map(|(i, _)| i)
        .collect();

    // Snapshot everything we need from app to avoid borrow-checker fights inside closures.
    struct RowData {
        idx: usize,
        name: String,
        enabled: bool,
        is_workshop: bool,
        workshop_installed: Option<bool>,
    }
    let rows: Vec<RowData> = filtered_indices
        .iter()
        .map(|&idx| {
            let m = &app.current_mods[idx];
            let is_workshop = !m.workshop_id.is_empty() && m.workshop_id != "0";
            let workshop_installed = app.is_workshop_mod_installed(&m.workshop_id);
            RowData {
                idx,
                name: m.name.clone(),
                enabled: m.enabled,
                is_workshop,
                workshop_installed,
            }
        })
        .collect();

    let drag_source_idx = app.drag_state.as_ref().map(|d| d.source_index);

    // Outputs collected during the loop, applied afterwards to avoid borrow conflicts.
    let mut toggle_idx: Option<usize> = None;
    let mut drag_started: Option<usize> = None;
    // Position in `filtered_indices` to insert the dragged item before (0 = top, n = bottom).
    let mut drop_insert_pos: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let n = rows.len();
            for (row_num, row) in rows.iter().enumerate() {
                let is_drag_source = drag_source_idx == Some(row.idx);
                let is_even = row_num % 2 == 0;

                // --- Row background colour ---
                let base_fill = if is_drag_source {
                    // Dragged item: show dimmed at original slot as a "ghost".
                    ui.visuals().extreme_bg_color
                } else if row.enabled {
                    if is_even {
                        ui.visuals().widgets.active.bg_fill.linear_multiply(0.10)
                    } else {
                        ui.visuals().widgets.active.bg_fill.linear_multiply(0.05)
                    }
                } else if is_even {
                    ui.visuals().faint_bg_color
                } else {
                    egui::Color32::TRANSPARENT
                };

                // Each row gets a stable egui ID keyed on its position-index.
                // Because we use commit-on-release (no live swap), `row.idx` is stable
                // for the entire drag interaction and can safely serve as the ID.
                let frame_resp = ui
                    .push_id(row.idx, |ui| {
                        let inner = egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .corner_radius(2)
                            .fill(base_fill)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // ── Drag handle ─────────────────────────────
                                    if can_drag {
                                        let handle_color = if is_drag_source {
                                            ui.visuals().selection.bg_fill
                                        } else {
                                            ui.visuals().weak_text_color()
                                        };
                                        let handle_text =
                                            egui::RichText::new("\u{2261}") // ≡
                                                .size(18.0)
                                                .color(handle_color);
                                        let handle = ui.add(
                                            egui::Label::new(handle_text)
                                                .sense(egui::Sense::drag()),
                                        );
                                        if handle.drag_started() && app.drag_state.is_none() {
                                            drag_started = Some(row.idx);
                                        }
                                        if handle.hovered() && app.drag_state.is_none() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                        }
                                        if app.drag_state.is_some() {
                                            ui.ctx()
                                                .set_cursor_icon(egui::CursorIcon::Grabbing);
                                        }
                                    } else {
                                        ui.add_space(22.0);
                                    }

                                    ui.add_space(4.0);

                                    // ── Enable/disable checkbox ──────────────────
                                    let mut cb_enabled = row.enabled;
                                    if ui.checkbox(&mut cb_enabled, "").changed() && !is_locked {
                                        toggle_idx = Some(row.idx);
                                    }

                                    ui.add_space(2.0);

                                    // ── Mod name label ───────────────────────────
                                    let name_color = if is_drag_source || !row.enabled {
                                        ui.visuals().weak_text_color()
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    let name_label = egui::Label::new(
                                        egui::RichText::new(&row.name)
                                            .size(13.0)
                                            .color(name_color),
                                    )
                                    .sense(egui::Sense::click());
                                    let name_resp = ui.add(name_label);
                                    if name_resp.clicked()
                                        && !is_locked
                                        && app.drag_state.is_none()
                                    {
                                        toggle_idx = Some(row.idx);
                                    }

                                    // ── Workshop [W] badge ───────────────────────
                                    if row.is_workshop {
                                        ui.label(
                                            egui::RichText::new("[W]")
                                                .small()
                                                .strong()
                                                .color(egui::Color32::from_rgb(70, 130, 180)),
                                        );
                                    }

                                    // ── Missing mod indicator ────────────────────
                                    if let Some(false) = row.workshop_installed {
                                        ui.label(
                                            egui::RichText::new("[Missing]")
                                                .small()
                                                .strong()
                                                .color(egui::Color32::from_rgb(220, 60, 60)),
                                        );
                                    }
                                });
                            });
                        inner.response
                    })
                    .inner;

                // Context menu on the entire row (only when not dragging).
                if app.drag_state.is_none() {
                    let idx = row.idx;
                    frame_resp.context_menu(|ui| {
                        crate::ui::context_menu::render_context_menu(app, ui, idx);
                    });
                }

                // ── Drop-target detection ────────────────────────────────────────
                // Use mouse Y position within this row's rect to decide the insert position.
                if app.drag_state.is_some() {
                    let rect = frame_resp.rect;
                    if let Some(ptr) = ui.ctx().pointer_latest_pos() {
                        if ptr.y >= rect.top() && ptr.y < rect.bottom() {
                            let insert_before = ptr.y < rect.center().y;
                            let candidate_pos =
                                if insert_before { row_num } else { row_num + 1 };

                            // Don't show an indicator that would leave the item in place.
                            if !is_drag_source {
                                drop_insert_pos = Some(candidate_pos.min(n));
                            }

                            // Draw the drop-indicator line.
                            if let Some(pos) = drop_insert_pos {
                                // Only draw if this row "owns" the indicator.
                                let indicator_y = if insert_before {
                                    rect.top()
                                } else {
                                    rect.bottom()
                                };
                                // Verify the indicator is for this row.
                                let this_rows_pos = if insert_before { row_num } else { row_num + 1 };
                                if pos == this_rows_pos.min(n) {
                                    let stroke = egui::Stroke::new(
                                        2.0,
                                        ui.visuals().selection.bg_fill,
                                    );
                                    ui.painter().line_segment(
                                        [
                                            egui::pos2(rect.left(), indicator_y),
                                            egui::pos2(rect.right(), indicator_y),
                                        ],
                                        stroke,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Mod count footer.
            ui.add_space(4.0);
            ui.separator();
            let total = app.current_mods.len();
            let shown = rows.len();
            let count_text = if shown == total {
                format!("{} mod{}", total, if total == 1 { "" } else { "s" })
            } else {
                format!("{} of {} mods", shown, total)
            };
            ui.label(
                egui::RichText::new(count_text)
                    .small()
                    .color(ui.visuals().weak_text_color()),
            );
        });

    // ── Apply pending state changes ──────────────────────────────────────────

    // Toggle enable/disable.
    if let Some(idx) = toggle_idx {
        app.current_mods[idx].enabled = !app.current_mods[idx].enabled;
        app.save_mod_config_and_preset();
    }

    // Begin drag.
    if let Some(idx) = drag_started {
        app.drag_state = Some(DragState {
            source_index: idx,
            pre_drag_snapshot: app.current_mods.clone(),
        });
        let _ = crate::core::logging::log(
            "INFO",
            &format!("Drag started: mod at position {}", idx + 1),
            "ModList",
        );
    }

    // Commit drag on pointer release.
    if ui.input(|i| i.pointer.any_released()) {
        if let Some(drag) = app.drag_state.take() {
            if let Some(insert_pos) = drop_insert_pos {
                let src = drag.source_index;
                let n = app.current_mods.len();

                // Dropping immediately before or after the item itself is a no-op.
                let is_noop = insert_pos == src || insert_pos == src + 1;

                if !is_noop && src < n {
                    let dst = if insert_pos > src {
                        // Removal shifts subsequent indices left by one.
                        (insert_pos - 1).min(n - 1)
                    } else {
                        insert_pos.min(n - 1)
                    };
                    let item = app.current_mods.remove(src);
                    app.current_mods.insert(dst, item);
                    let _ = crate::core::logging::log(
                        "INFO",
                        &format!("Moved mod from position {} to {}", src + 1, dst + 1),
                        "ModList",
                    );
                    app.save_mod_config_and_preset();
                }
            }
            // If no drop target was selected (released outside the list), keep original order.
        }
    }

    // Escape cancels an in-progress drag.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if let Some(drag) = app.drag_state.take() {
            app.current_mods = drag.pre_drag_snapshot;
            let _ = crate::core::logging::log("INFO", "Drag cancelled", "ModList");
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
        ui.label(egui::RichText::new(format!("Preset: {}", app.selected_preset)).size(14.0));
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
