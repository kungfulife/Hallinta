use crate::app::HallintaApp;
use crate::models::{DragState, FilterMode, SortMode};
use eframe::egui;

const MONITOR_EDIT_NOTICE: &str = "Monitoring active - edit mods carefully.";

pub fn render_mod_list(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);

    if app.save_monitor.is_running() {
        render_monitor_edit_notice(app, ui, &d);
        ui.add_space(d.sm);
    }

    if app.current_mods.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No mods loaded. Check your Noita save directory in Settings.")
                    .size(d.font_heading)
                    .italics(),
            );
        });
        return;
    }

    let search_lower = app.search_query.to_lowercase();
    let filter = app.filter_mode;
    let sort = app.sort_mode;

    // Drag-to-reorder only works on the unfiltered, unsorted list to avoid index confusion.
    let can_drag =
        search_lower.is_empty() && filter == FilterMode::All && sort == SortMode::Default;

    // Build the visible subset (filtered).
    let mut filtered_indices: Vec<usize> = app
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
                return true;
            }
            m.name.to_lowercase().contains(&search_lower)
                || m.workshop_id.to_lowercase().contains(&search_lower)
        })
        .map(|(i, _)| i)
        .collect();

    // Apply visual sort over the filtered subset (does not mutate underlying data).
    match sort {
        SortMode::Default => {}
        SortMode::NameAsc => filtered_indices.sort_by(|&a, &b| {
            app.current_mods[a]
                .name
                .to_lowercase()
                .cmp(&app.current_mods[b].name.to_lowercase())
        }),
        SortMode::NameDesc => filtered_indices.sort_by(|&a, &b| {
            app.current_mods[b]
                .name
                .to_lowercase()
                .cmp(&app.current_mods[a].name.to_lowercase())
        }),
        SortMode::EnabledFirst => filtered_indices.sort_by(|&a, &b| {
            app.current_mods[b]
                .enabled
                .cmp(&app.current_mods[a].enabled)
        }),
        SortMode::DisabledFirst => filtered_indices.sort_by(|&a, &b| {
            app.current_mods[a]
                .enabled
                .cmp(&app.current_mods[b].enabled)
        }),
    }

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
    let mut enable_all = false;
    let mut disable_all = false;
    let mut apply_sort = false;

    // Subtle tinted panel background to frame the mod list
    egui::Frame::NONE
        .fill(d.mod_list_bg)
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(d.sm as i8, d.sm as i8))
        .show(ui, |ui| {
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
                            .corner_radius(4.0)
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
                    } else {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }

                // Drag start — whole row is the handle now
                if row_resp.drag_started() && can_drag && app.drag_state.is_none() {
                    drag_started = Some(row.idx);
                }

                // Row click toggles the mod (egui guarantees clicked() is false when drag was started)
                if row_resp.clicked() {
                    toggle_idx = Some(row.idx);
                }

                // ── Visual overlays ─────────────────────────────────────────
                let painter = ui.painter();

                // Hover: tinted fill + border for clear feedback
                if row_resp.hovered() && !is_ghost && app.drag_state.is_none() {
                    painter.rect_filled(
                        frame_resp.rect,
                        4.0,
                        d.row_hover,
                    );
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0,
                        egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),
                        egui::StrokeKind::Outside,
                    );
                }

                // Ghost row: accent border so the dragged item stands out
                if is_ghost {
                    painter.rect_stroke(
                        frame_resp.rect,
                        4.0,
                        egui::Stroke::new(1.5, d.drag_ghost_border),
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
            ui.horizontal(|ui| {
                let total = app.current_mods.len();
                let enabled_count = app.current_mods.iter().filter(|m| m.enabled).count();
                let shown = rows.len();
                let count_text = if shown == total {
                    format!("{} enabled / {} total", enabled_count, total)
                } else {
                    format!(
                        "{} shown · {} enabled / {} total",
                        shown, enabled_count, total
                    )
                };
                ui.label(
                    egui::RichText::new(count_text)
                        .size(d.font_small)
                        .color(ui.visuals().weak_text_color()),
                );

                if total > 0 {
                    ui.separator();
                    if ui.small_button("Enable All")
                        .on_hover_text("Enable every mod (Ctrl+E)")
                        .clicked()
                    {
                        enable_all = true;
                    }
                    if ui.small_button("Disable All")
                        .on_hover_text("Disable every mod (Ctrl+D)")
                        .clicked()
                    {
                        disable_all = true;
                    }
                    if app.sort_mode != SortMode::Default {
                        ui.separator();
                        if ui.small_button("Apply Sort to Order")
                            .on_hover_text(
                                "Persist the current visual sort as the actual mod_config.xml order",
                            )
                            .clicked()
                        {
                            apply_sort = true;
                        }
                    }
                }
            });
        });
    }); // mod list background frame

    // ── Apply pending state changes ──────────────────────────────────────────

    if let Some(idx) = toggle_idx {
        let new_state = !app.current_mods[idx].enabled;
        app.current_mods[idx].enabled = new_state;
        let _ = crate::core::logging::log(
            "INFO",
            &format!(
                "{} mod: {}",
                if new_state { "Enabled" } else { "Disabled" },
                app.current_mods[idx].name
            ),
            "ModManager",
        );
        app.save_mod_config_and_preset();
    }

    if enable_all {
        app.bulk_set_enabled(true);
    }

    if disable_all {
        app.bulk_set_enabled(false);
    }

    if apply_sort {
        app.apply_sort_to_order();
    }

    if let Some(idx) = drag_started {
        app.drag_state = Some(DragState {
            current_index: idx,
            pre_drag_snapshot: app.current_mods.clone(),
        });
    }

    // Live-reorder: move the dragged item to the hovered position each frame
    if let Some(target) = drag_move_to
        && let Some(drag) = &mut app.drag_state
        && target != drag.current_index
    {
        let item = app.current_mods.remove(drag.current_index);
        app.current_mods.insert(target, item);
        drag.current_index = target;
    }

    // Commit drag on pointer release — list is already in the right order
    if ui.input(|i| i.pointer.any_released())
        && let Some(drag) = app.drag_state.take()
    {
        if drag.current_index >= app.current_mods.len() {
            // List shrank during drag (shouldn't happen, but defensive). Restore snapshot.
            let _ = crate::core::logging::log(
                "WARN",
                "Drag commit aborted: index out of range, restoring snapshot",
                "ModList",
            );
            app.current_mods = drag.pre_drag_snapshot;
        } else {
            let moved_name = app.current_mods[drag.current_index].name.clone();
            let snapshot_idx = drag
                .pre_drag_snapshot
                .iter()
                .position(|m| m.name == moved_name);
            let changed = snapshot_idx.is_none_or(|orig| orig != drag.current_index);
            if changed {
                let from = snapshot_idx
                    .map(|i| (i + 1).to_string())
                    .unwrap_or_else(|| "?".to_string());
                let _ = crate::core::logging::log(
                    "INFO",
                    &format!(
                        "Reordered \"{}\": position {} -> {}",
                        moved_name,
                        from,
                        drag.current_index + 1
                    ),
                    "ModList",
                );
                app.save_mod_config_and_preset();
            }
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

fn render_monitor_edit_notice(app: &HallintaApp, ui: &mut egui::Ui, d: &crate::ui::design::Design) {
    egui::Frame::NONE
        .fill(d.helper_text_bg)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(d.md as i8, d.sm as i8))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(MONITOR_EDIT_NOTICE)
                        .strong()
                        .size(d.font_body)
                        .color(d.helper_text_color),
                );
                if app.file_watcher.pending_external_mods.is_some() {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(
                            "Disk changes will be reviewed when monitoring pauses.",
                        )
                        .size(d.font_small)
                        .color(ui.visuals().weak_text_color()),
                    );
                }
            });
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
    let r = rect.height() / 2.0 - 2.0;
    let cx = if enabled {
        rect.right() - rect.height() / 2.0
    } else {
        rect.left() + rect.height() / 2.0
    };
    let center = egui::pos2(cx, rect.center().y);

    // Subtle shadow: slightly darker circle offset by 1 logical pixel
    painter.circle_filled(
        egui::pos2(cx + 0.5, rect.center().y + 0.5),
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

#[cfg(test)]
mod tests {
    use super::MONITOR_EDIT_NOTICE;

    #[test]
    fn monitor_mode_mod_list_has_passive_edit_notice() {
        assert_eq!(
            MONITOR_EDIT_NOTICE,
            "Monitoring active - edit mods carefully."
        );
    }
}
