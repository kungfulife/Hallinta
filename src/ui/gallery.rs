use crate::app::HallintaApp;
use eframe::egui;

pub fn render_gallery(app: &mut HallintaApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Modpacks").heading().strong().size(20.0));
    ui.add_space(8.0);

    // Search and tag filter
    ui.horizontal(|ui| {
        ui.label("Search:");
        ui.text_edit_singleline(&mut app.gallery_state.search_query);

        if ui.button("Refresh Catalog").clicked() && !app.gallery_state.loading {
            app.fetch_catalog();
        }
    });

    // Show error if any
    if let Some(ref error) = app.gallery_state.error {
        ui.colored_label(egui::Color32::RED, error);
        ui.add_space(4.0);
    }

    // Show loading state
    if app.gallery_state.loading {
        ui.spinner();
        ui.label("Loading catalog...");
        return;
    }

    let catalog = match &app.gallery_state.catalog {
        Some(c) => c.clone(),
        None => {
            ui.label("No catalog loaded. Click 'Refresh Catalog' to fetch.");
            return;
        }
    };

    // Collect all tags for filter chips
    let mut all_tags: Vec<String> = catalog
        .presets
        .iter()
        .flat_map(|p| p.tags.iter().cloned())
        .collect();
    all_tags.sort();
    all_tags.dedup();

    if !all_tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label("Tags:");
            for tag in &all_tags {
                let selected = app.gallery_state.selected_tags.contains(tag);
                if ui.selectable_label(selected, tag).clicked() {
                    if selected {
                        app.gallery_state.selected_tags.retain(|t| t != tag);
                    } else {
                        app.gallery_state.selected_tags.push(tag.clone());
                    }
                }
            }
            if !app.gallery_state.selected_tags.is_empty() {
                if ui.small_button("Clear").clicked() {
                    app.gallery_state.selected_tags.clear();
                }
            }
        });
        ui.add_space(4.0);
    }

    // Filter presets
    let query = app.gallery_state.search_query.to_lowercase();
    let selected_tags = &app.gallery_state.selected_tags;
    let filtered: Vec<_> = catalog
        .presets
        .iter()
        .filter(|p| {
            if !query.is_empty() {
                let matches = p.name.to_lowercase().contains(&query)
                    || p.description.to_lowercase().contains(&query)
                    || p.author.to_lowercase().contains(&query);
                if !matches {
                    return false;
                }
            }
            if !selected_tags.is_empty() {
                // OR-based: any selected tag matches
                if !p.tags.iter().any(|t| selected_tags.contains(t)) {
                    return false;
                }
            }
            true
        })
        .collect();

    ui.label(format!("{} preset(s) found", filtered.len()));
    ui.add_space(4.0);

    // Grid of preset cards
    egui::ScrollArea::vertical().show(ui, |ui| {
        for preset in &filtered {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.strong(&preset.name);
                            ui.label(format!("by {}", preset.author));
                            if !preset.description.is_empty() {
                                ui.label(&preset.description);
                            }
                            ui.horizontal(|ui| {
                                ui.label(format!("{} mods", preset.mod_count));
                                if !preset.tags.is_empty() {
                                    ui.label(format!("Tags: {}", preset.tags.join(", ")));
                                }
                            });
                        });

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let is_locked = app.save_monitor.is_running();
                                ui.add_enabled_ui(!is_locked, |ui| {
                                    if ui.button("Download & Import").clicked() {
                                        app.download_and_import_preset(
                                            &preset.download_url,
                                            &preset.checksum,
                                        );
                                    }
                                });
                            },
                        );
                    });
                });
            ui.add_space(4.0);
        }
    });
}
