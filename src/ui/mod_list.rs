use crate::app::HallintaApp;
use crate::models::{DragState, FilterMode};
use eframe::egui;

pub fn render_mod_list(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);

    if app.current_mods.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new(
                    "No mods loaded. Check your Noita save directory in Settings.",
                )
                .size(d.font_heading)
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

    let drag_current_idx = app.drag_state.as_ref().map(|d| d.current_index);

    // Outputs collected during the loop, applied afterwards to avoid borrow conflicts.
    let mut toggle_idx: Option<usize> = None;
    let mut drag_started: Option<usize> = None;
    // Live-reorder target: row index the dragged item should move to this frame.
    let mut drag_move_to: Option<usize> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            for (row_num, row) in rows.iter().enumerate() {
                let is_ghost = drag_current_idx == Some(row.idx);
                let is_even = row_num % 2 == 0;

                // ── Row background fill ─────────────────────────────────────
                let base_fill = if is_ghost {
                    d.drag_ghost_fill
                } else if row.enabled {
                    if is_even { d.enabled_even } else { d.enabled_odd }
                } else if is_even {
                    d.disabled_even
                } else {
                    d.disabled_odd
                };

                // `row.idx` is stable throughout the drag (live reordering keeps the
                // same items), so it's a safe egui ID seed.
                let row_interact_id = egui::Id::new(("hallinta_ri", row.idx));

                // ── Render the row frame ────────────────────────────────────
                let frame_resp = ui
                    .push_id(row.idx, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                d.row_pad_x as i8,
                                d.row_pad_y as i8,
                            ))
                            .corner_radius(4.0 * d.scale)
                            .fill(base_fill)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    // ── Row number (fixed-width gutter) ──────────────────────
                                    // During a drag, show the live position; otherwise the
                                    // true index.
                                    let display_num = if app.drag_state.is_some() {
                                        row_num + 1
                                    } else {
                                        row.idx + 1
                                    };
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(d.row_number_w, ui.available_height()),
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{}", display_num))
                                                    .size(d.font_small)
                                                    .color(d.row_number_color),
                                            );
                                        },
                                    );

                                    ui.add_space(d.md);

                                    // ── Mod name ────────────────────────────────────────────
                                    let name_color = if is_ghost {
                                        ui.visuals().weak_text_color()
                                    } else if !row.enabled {
                                        d.disabled_text
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    let name_style = if !row.enabled {
                                        egui::RichText::new(&row.name)
                                            .size(d.font_body)
                                            .color(name_color)
                                            .italics()
                                    } else {
                                        egui::RichText::new(&row.name)
                                            .size(d.font_body)
                                            .color(name_color)
                                    };
                                    ui.label(name_style);

                                    // ── Workshop badge ───────────────────────────────────
                                    if row.is_workshop {
                                        draw_badge(ui, "W", d.badge_workshop, &d);
                                    }

                                    // ── Missing mod indicator ────────────────────────────────
                                    if let Some(false) = row.workshop_installed {
                                        draw_badge(ui, "Missing", d.badge_missing, &d);
                                    }

                                    // ── Toggle switch (far right) ────────────────────────────
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            draw_toggle_visual(ui, row.enabled, &d);
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

                // Hover: tinted fill + border for clear feedback
                if row_resp.hovered() && !is_ghost && app.drag_state.is_none() {
                    painter.rect_filled(
                        frame_resp.rect,
                        4.0 * d.scale,
                        d.row_hover,
                    );
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0 * d.scale,
                        egui::Stroke::new(1.0 * d.scale, ui.visuals().widgets.hovered.bg_stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }

                // Ghost row: accent border so the dragged item stands out
                if is_ghost {
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0 * d.scale,
                        egui::Stroke::new(1.5 * d.scale, d.drag_ghost_border),
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

                // ── Live-reorder detection ───────────────────────────────────
                // When hovering a non-ghost row during a drag, signal that the
                // dragged item should move to this position.
                if app.drag_state.is_some() && !is_ghost {
                    let rect = frame_resp.rect;
                    if let Some(ptr) = ui.ctx().pointer_latest_pos()
                        && ptr.y >= rect.top() && ptr.y < rect.bottom()
                    {
                        drag_move_to = Some(row_num);
                    }
                }

                // Row gap for visual breathing room
                ui.add_space(d.xs);
            }

            // Mod count footer
            ui.add_space(d.sm);
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
                    .size(d.font_small)
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
            current_index: idx,
            pre_drag_snapshot: app.current_mods.clone(),
        });
        let _ = crate::core::logging::log(
            "INFO",
            &format!("Drag started: mod at position {}", idx + 1),
            "ModList",
        );
    }

    // Live-reorder: move the dragged item to the hovered position each frame
    if let Some(target) = drag_move_to {
        if let Some(drag) = &mut app.drag_state {
            if target != drag.current_index {
                let item = app.current_mods.remove(drag.current_index);
                app.current_mods.insert(target, item);
                drag.current_index = target;
            }
        }
    }

    // Commit drag on pointer release — list is already in the right order
    if ui.input(|i| i.pointer.any_released())
        && let Some(drag) = app.drag_state.take()
    {
        let snapshot_idx = drag.pre_drag_snapshot
            .iter()
            .position(|m| m.name == app.current_mods[drag.current_index].name);
        let changed = snapshot_idx.map_or(true, |orig| orig != drag.current_index);
        if changed {
            let _ = crate::core::logging::log(
                "INFO",
                &format!("Moved mod to position {}", drag.current_index + 1),
                "ModList",
            );
            app.save_mod_config_and_preset();
        }
    }

    // Escape cancels drag — restore from snapshot
    if ui.input(|i| i.key_pressed(egui::Key::Escape))
        && let Some(drag) = app.drag_state.take()
    {
        app.current_mods = drag.pre_drag_snapshot;
        let _ = crate::core::logging::log("INFO", "Drag cancelled", "ModList");
    }
}

/// Shown when the save monitor is active, blocking mod list / modpacks access.
pub fn render_monitor_active(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);

    // True vertical centering: pad top by half available height minus estimated content height
    let content_h = d.font_display + d.font_heading + d.font_body * 3.0 + d.lg * 5.0;
    let top_pad = ((ui.available_height() - content_h) / 2.0).max(d.lg);
    ui.add_space(top_pad);

    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new("Save Monitor Active")
                .size(d.font_display)
                .strong(),
        );
        ui.add_space(d.md);

        ui.colored_label(
            d.status_ok,
            egui::RichText::new("Running").size(d.font_heading).strong(),
        );

        ui.add_space(d.md);
        ui.label(egui::RichText::new(format!("Preset: {}", app.selected_preset)).size(d.font_body));
        ui.label(
            egui::RichText::new(format!(
                "Snapshots taken: {}",
                app.save_monitor.snapshot_count
            ))
            .size(d.font_body),
        );
        ui.label(
            egui::RichText::new(format!(
                "Interval: {} min | Max: {} | Keep every {}th",
                app.settings.save_monitor_settings.interval_minutes,
                app.settings.save_monitor_settings.max_snapshots_per_preset,
                app.settings.save_monitor_settings.keep_every_nth,
            ))
            .size(d.font_small)
            .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(d.lg);

        ui.label(
            egui::RichText::new("Mod list and Modpacks are locked while monitor is running.")
                .italics()
                .size(d.font_small)
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(d.md);

        if ui
            .button(egui::RichText::new("Stop Monitor").size(d.font_body))
            .clicked()
        {
            app.stop_save_monitor();
        }
    });
}

fn draw_toggle_visual(ui: &mut egui::Ui, enabled: bool, d: &crate::ui::design::Design) {
    let desired_size = egui::vec2(d.toggle_w, d.toggle_h);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    if !ui.is_rect_visible(rect) {
        return;
    }

    let bg = if enabled {
        d.toggle_on
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };

    let painter = ui.painter();
    painter.rect_filled(rect, rect.height() / 2.0, bg);

    // Knob position
    let r = rect.height() / 2.0 - 2.0 * d.scale;
    let cx = if enabled {
        rect.right() - rect.height() / 2.0
    } else {
        rect.left() + rect.height() / 2.0
    };
    let center = egui::pos2(cx, rect.center().y);

    // Subtle shadow: slightly darker circle offset by 1 logical pixel
    painter.circle_filled(
        egui::pos2(cx + d.scale * 0.5, rect.center().y + d.scale * 0.5),
        r,
        egui::Color32::from_black_alpha(40),
    );
    painter.circle_filled(center, r, egui::Color32::WHITE);
}

/// Draws a small rounded pill badge with the given background color and white text.
fn draw_badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, d: &crate::ui::design::Design) {
    let pad = egui::vec2(d.sm, d.xs);
    // Use a custom child UI with a Frame so egui handles text measurement internally.
    egui::Frame::NONE
        .fill(bg)
        .inner_margin(egui::Margin {
            left: pad.x as i8,
            right: pad.x as i8,
            top: pad.y as i8,
            bottom: pad.y as i8,
        })
        .corner_radius(99)
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(d.font_small)
                    .color(egui::Color32::WHITE)
                    .strong(),
            );
        });
}
