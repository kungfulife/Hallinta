use crate::app::HallintaApp;
use crate::models::{UpdatePhase, UpdateStatus};
use eframe::egui;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ReleaseNoteBlock {
    Heading(String),
    Bullet(String),
    Paragraph(String),
}

fn parse_release_notes(notes: &str) -> Vec<ReleaseNoteBlock> {
    fn flush(current: &mut Option<ReleaseNoteBlock>, blocks: &mut Vec<ReleaseNoteBlock>) {
        if let Some(block) = current.take() {
            blocks.push(block);
        }
    }

    fn append(current: &mut Option<ReleaseNoteBlock>, text: &str) {
        match current {
            Some(ReleaseNoteBlock::Bullet(value)) | Some(ReleaseNoteBlock::Paragraph(value)) => {
                value.push(' ');
                value.push_str(text);
            }
            Some(ReleaseNoteBlock::Heading(_)) => unreachable!(),
            None => *current = Some(ReleaseNoteBlock::Paragraph(text.to_owned())),
        }
    }

    let mut blocks = Vec::new();
    let mut current = None;
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut current, &mut blocks);
            continue;
        }

        let without_hashes = trimmed.trim_start_matches('#');
        let is_heading = without_hashes.len() < trimmed.len()
            && without_hashes
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
        if is_heading {
            flush(&mut current, &mut blocks);
            blocks.push(ReleaseNoteBlock::Heading(without_hashes.trim().to_owned()));
        } else if let Some(item) = line.trim_start().strip_prefix("- ") {
            flush(&mut current, &mut blocks);
            current = Some(ReleaseNoteBlock::Bullet(item.trim().to_owned()));
        } else {
            append(&mut current, trimmed);
        }
    }
    flush(&mut current, &mut blocks);
    blocks
}

pub fn render(app: &mut HallintaApp, ctx: &egui::Context) {
    let status = app.update_state.status.clone();
    match status {
        UpdateStatus::Available(info) => {
            let d = crate::ui::design::Design::new(ctx, &app.settings);
            let release_url = crate::core::updater::release_url(&info.version);
            egui::Window::new(format!("Hallinta v{} is available", info.version))
                .collapsible(false)
                .resizable(true)
                .default_width(470.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    if ui
                        .link(
                            egui::RichText::new("View release on GitHub")
                                .strong()
                                .underline()
                                .color(d.update_link),
                        )
                        .clicked()
                    {
                        let _ = crate::core::platform::open_url(&release_url);
                    }
                    if !info.notes.trim().is_empty() {
                        ui.add_space(d.sm);
                        ui.label(
                            egui::RichText::new("What's new")
                                .strong()
                                .size(d.font_tab)
                                .color(d.update_notes_heading),
                        );
                        ui.add_space(d.sm);
                        render_release_notes(ui, &d, &info.notes);
                    }
                    ui.add_space(d.sm);
                    ui.horizontal(|ui| {
                        if ui.button("Update & Restart").clicked() {
                            app.begin_update(info.clone());
                        }
                        if ui
                            .button("Dismiss")
                            .on_hover_text(
                                "Stop auto-prompting for this version. A newer release will ask again. You can still check from Settings.",
                            )
                            .clicked()
                        {
                            app.dismiss_update_status();
                        }
                    });
                });
        }
        UpdateStatus::Checking { manual: true } => {
            egui::Window::new("Checking for updates")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.spinner();
                    ui.label("Contacting GitHub Releases…");
                });
        }
        UpdateStatus::Failed { message, retryable } => {
            egui::Window::new("Hallinta Update")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.horizontal(|ui| {
                        if retryable && ui.button("Retry").clicked() {
                            app.check_for_updates(true);
                        }
                        if ui.button("Close").clicked() {
                            app.dismiss_update_status();
                        }
                    });
                });
        }
        UpdateStatus::Running { phase, message } => render_lock(ctx, phase, &message),
        UpdateStatus::Idle | UpdateStatus::Checking { manual: false } => {}
    }
}

fn render_release_notes(ui: &mut egui::Ui, d: &crate::ui::design::Design, notes: &str) {
    egui::Frame::NONE
        .inner_margin(egui::Margin::same(6))
        .corner_radius(4.0)
        .fill(d.update_notes_bg)
        .stroke(egui::Stroke::new(1.0, d.update_notes_border))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(180.0)
                .show(ui, |ui| {
                    for block in parse_release_notes(notes) {
                        match block {
                            ReleaseNoteBlock::Heading(text) => {
                                ui.add_space(d.sm);
                                ui.label(
                                    egui::RichText::new(text)
                                        .strong()
                                        .size(d.font_body)
                                        .color(d.update_notes_heading),
                                );
                            }
                            ReleaseNoteBlock::Bullet(text) => {
                                ui.horizontal_top(|ui| {
                                    ui.label(
                                        egui::RichText::new("•")
                                            .strong()
                                            .color(d.update_notes_bullet),
                                    );
                                    ui.add(egui::Label::new(text).wrap());
                                });
                            }
                            ReleaseNoteBlock::Paragraph(text) => {
                                ui.add(egui::Label::new(text).wrap());
                            }
                        }
                    }
                });
        });
}

fn render_lock(ctx: &egui::Context, phase: UpdatePhase, message: &str) {
    let rect = ctx.content_rect();
    egui::Area::new(egui::Id::new("update_input_blocker"))
        .order(egui::Order::Foreground)
        .fixed_pos(rect.min)
        .show(ctx, |ui| {
            let response = ui.allocate_rect(
                egui::Rect::from_min_size(egui::Pos2::ZERO, rect.size()),
                egui::Sense::click_and_drag(),
            );
            ui.painter()
                .rect_filled(response.rect, 0.0, egui::Color32::from_black_alpha(190));
        });
    egui::Window::new("Updating Hallinta")
        .id(egui::Id::new("update_progress_window"))
        .order(egui::Order::Tooltip)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_min_width(410.0);
            ui.heading(phase_label(phase));
            ui.label(message);
            ui.spinner();
            ui.label("Controls are temporarily disabled to protect application data.");
        });
}

fn phase_label(phase: UpdatePhase) -> &'static str {
    match phase {
        UpdatePhase::Preparing => "Preparing",
        UpdatePhase::Snapshotting => "Protecting monitor data",
        UpdatePhase::Installing => "Installing signed update",
        UpdatePhase::Restarting => "Restarting",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_bullets_and_paragraphs() {
        let notes = "## Interface\n\n- Added a clearer prompt.\n\nPlain summary.";
        assert_eq!(
            parse_release_notes(notes),
            vec![
                ReleaseNoteBlock::Heading("Interface".into()),
                ReleaseNoteBlock::Bullet("Added a clearer prompt.".into()),
                ReleaseNoteBlock::Paragraph("Plain summary.".into()),
            ]
        );
    }

    #[test]
    fn joins_wrapped_bullet_and_paragraph_lines() {
        let notes = "- Signed archives remain available\n  for portable installs.\n\nFirst paragraph line\ncontinues here.";
        assert_eq!(
            parse_release_notes(notes),
            vec![
                ReleaseNoteBlock::Bullet(
                    "Signed archives remain available for portable installs.".into()
                ),
                ReleaseNoteBlock::Paragraph("First paragraph line continues here.".into()),
            ]
        );
    }

    #[test]
    fn treats_indented_list_items_as_bullets() {
        assert_eq!(
            parse_release_notes("  - Keep detected setup."),
            vec![ReleaseNoteBlock::Bullet("Keep detected setup.".into())]
        );
    }
}
