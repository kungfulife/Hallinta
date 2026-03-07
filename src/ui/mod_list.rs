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

    // Build the visible subset.
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

    // Snapshot into plain structs to avoid borrow-checker fights inside closures.
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
    // Position in the visible list to insert before (0 = very top, n = very bottom).
    let mut drop_insert_pos: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let n = rows.len();
            for (row_num, row) in rows.iter().enumerate() {
                let is_drag_source = drag_source_idx == Some(row.idx);
                let is_even = row_num % 2 == 0;

                // ── Row background fill ─────────────────────────────────────
                let base_fill = if is_drag_source {
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

                // `row.idx` is stable throughout the drag (no live reordering), so it's a
                // safe egui ID seed.  The "ri" namespace avoids collisions with the checkbox.
                let row_interact_id = egui::Id::new(("hallinta_ri", row.idx));

                // ── Render the row frame ────────────────────────────────────
                let frame_resp = ui
                    .push_id(row.idx, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(6, 3))
                            .corner_radius(4)
                            .fill(base_fill)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // ── Row number (left) ───────────────────────────────────
                                    ui.label(
                                        egui::RichText::new(format!("{}", row.idx + 1))
                                            .size(11.0)
                                            .color(ui.visuals().weak_text_color()),
                                    );

                                    ui.add_space(4.0);

                                    // ── Mod name ────────────────────────────────────────────
                                    let name_color = if is_drag_source || !row.enabled {
                                        ui.visuals().weak_text_color()
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    ui.label(
                                        egui::RichText::new(&row.name)
                                            .size(13.0)
                                            .color(name_color),
                                    );

                                    // ── Workshop [W] badge ───────────────────────────────────
                                    if row.is_workshop {
                                        ui.label(
                                            egui::RichText::new("[W]")
                                                .small()
                                                .strong()
                                                .color(egui::Color32::from_rgb(70, 130, 180)),
                                        );
                                    }

                                    // ── Missing mod indicator ────────────────────────────────
                                    if let Some(false) = row.workshop_installed {
                                        ui.label(
                                            egui::RichText::new("[Missing]")
                                                .small()
                                                .strong()
                                                .color(egui::Color32::from_rgb(220, 60, 60)),
                                        );
                                    }

                                    // ── Toggle switch (far right) ────────────────────────────
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            draw_toggle_visual(ui, row.enabled);
                                        },
                                    );
                                });
                            })
                            .response
                    })
                    .inner;

                // ── Whole-row interaction (drag + right-click) ──────────────
                // Using click_and_drag so secondary_clicked() works for context_menu.
                let row_resp = ui.interact(
                    frame_resp.rect,
                    row_interact_id,
                    egui::Sense::click_and_drag(),
                );

                // Cursor icon
                if row_resp.hovered() {
                    if app.drag_state.is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    } else if !is_locked {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }

                // Drag start — whole row is the handle now
                if row_resp.drag_started() && can_drag && app.drag_state.is_none() {
                    drag_started = Some(row.idx);
                }

                // Row click toggles the mod (egui guarantees clicked() is false when drag was started)
                if row_resp.clicked() && !is_locked {
                    toggle_idx = Some(row.idx);
                }

                // ── Visual overlays ─────────────────────────────────────────
                let painter = ui.painter();

                // Hover border
                if row_resp.hovered() && !is_drag_source && app.drag_state.is_none() {
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0,
                        egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }

                // Active drag source: bright selection border
                if is_drag_source {
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0,
                        egui::Stroke::new(2.0, ui.visuals().selection.bg_fill),
                        egui::StrokeKind::Outside,
                    );
                }

                // ── Context menu (right-click anywhere on row) ──────────────
                if app.drag_state.is_none() {
                    let idx = row.idx;
                    row_resp.context_menu(|ui| {
                        crate::ui::context_menu::render_context_menu(app, ui, idx);
                    });
                }

                // ── Drop-target detection ────────────────────────────────────
                if app.drag_state.is_some() {
                    let rect = frame_resp.rect;
                    if let Some(ptr) = ui.ctx().pointer_latest_pos() {
                        if ptr.y >= rect.top() && ptr.y < rect.bottom() {
                            let insert_before = ptr.y < rect.center().y;
                            let candidate_pos =
                                if insert_before { row_num } else { row_num + 1 };

                            if !is_drag_source {
                                drop_insert_pos = Some(candidate_pos.min(n));
                            }

                            // Draw drop-indicator line
                            if let Some(pos) = drop_insert_pos {
                                let this_rows_pos =
                                    if insert_before { row_num } else { row_num + 1 };
                                if pos == this_rows_pos.min(n) {
                                    let indicator_y =
                                        if insert_before { rect.top() } else { rect.bottom() };
                                    painter.line_segment(
                                        [
                                            egui::pos2(rect.left(), indicator_y),
                                            egui::pos2(rect.right(), indicator_y),
                                        ],
                                        egui::Stroke::new(
                                            2.0,
                                            ui.visuals().selection.bg_fill,
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Mod count footer
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

    if let Some(idx) = toggle_idx {
        app.current_mods[idx].enabled = !app.current_mods[idx].enabled;
        app.save_mod_config_and_preset();
    }

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

    // Commit drag on pointer release
    if ui.input(|i| i.pointer.any_released()) {
        if let Some(drag) = app.drag_state.take() {
            if let Some(insert_pos) = drop_insert_pos {
                let src = drag.source_index;
                let n = app.current_mods.len();
                let is_noop = insert_pos == src || insert_pos == src + 1;
                if !is_noop && src < n {
                    let dst = if insert_pos > src {
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
            // Released outside list — keep original order (drag_state already taken/dropped)
        }
    }

    // Escape cancels drag
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if let Some(drag) = app.drag_state.take() {
            app.current_mods = drag.pre_drag_snapshot;
            let _ = crate::core::logging::log("INFO", "Drag cancelled", "ModList");
        }
    }
}

/// Shown when the save monitor is active, blocking mod list / modpacks access.
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
            egui::RichText::new("Mod list and Modpacks are locked while monitor is running.")
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

fn draw_toggle_visual(ui: &mut egui::Ui, enabled: bool) {
    let desired_size = egui::vec2(30.0, 16.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let bg = if enabled {
        egui::Color32::from_rgb(60, 160, 70)
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };

    let painter = ui.painter();
    painter.rect_filled(rect, rect.height() / 2.0, bg);

    let r = rect.height() / 2.0 - 2.0;
    let cx = if enabled {
        rect.right() - rect.height() / 2.0
    } else {
        rect.left() + rect.height() / 2.0
    };
    painter.circle_filled(egui::pos2(cx, rect.center().y), r, egui::Color32::WHITE);
}
