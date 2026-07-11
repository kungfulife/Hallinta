use crate::app::HallintaApp;
use crate::models::{NoitaSyncState, View};
use eframe::egui;

pub fn render(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let Some(message) = app.visible_noita_directory_error().map(str::to_owned) else {
        return;
    };
    let configuration_only = app.noita_sync_state == NoitaSyncState::ConfigurationOnly;
    let (title, helper) = if configuration_only {
        (
            "Configuration-only mode",
            "Preset edits are saved locally; Noita syncing is paused.",
        )
    } else {
        (
            "Warning preview",
            "Visual preview only; Noita syncing remains active.",
        )
    };
    let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
    let frame = egui::Frame::side_top_panel(ui.style())
        .fill(d.warning_fill)
        .stroke(egui::Stroke::new(1.0, d.warning_border))
        .inner_margin(egui::Margin::symmetric(10, 7));

    egui::Panel::top("noita_directory_warning")
        .frame(frame)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(title).strong().color(d.warning_text));
                ui.label(egui::RichText::new(message).color(d.warning_text));
                ui.label(egui::RichText::new(helper).color(d.warning_text));
                if app.active_view != View::Settings && ui.button("Open Settings").clicked() {
                    app.active_view = View::Settings;
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use crate::app::test_support::test_app;
    use crate::models::NoitaSyncState;
    use eframe::egui;

    fn text_from_shape(shape: &egui::epaint::Shape) -> Vec<String> {
        match shape {
            egui::epaint::Shape::Text(text) => vec![text.galley.text().to_string()],
            egui::epaint::Shape::Vec(shapes) => shapes.iter().flat_map(text_from_shape).collect(),
            _ => Vec::new(),
        }
    }

    #[test]
    fn warning_explains_the_problem_and_offers_settings() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.noita_directory_error = Some("mod_config.xml is missing".to_string());
        app.noita_sync_state = NoitaSyncState::ConfigurationOnly;
        let ctx = egui::Context::default();

        let output = ctx.run_ui(Default::default(), |ui| super::render(&mut app, ui));
        let labels: Vec<String> = output
            .shapes
            .iter()
            .flat_map(|shape| text_from_shape(&shape.shape))
            .collect();

        assert!(
            labels
                .iter()
                .any(|label| label == "Configuration-only mode")
        );
        assert!(
            labels
                .iter()
                .any(|label| label == "mod_config.xml is missing")
        );
        assert!(labels.iter().any(|label| label == "Open Settings"));
        assert!(
            labels.iter().any(|label| {
                label == "Preset edits are saved locally; Noita syncing is paused."
            })
        );
    }

    #[test]
    fn debug_preview_does_not_claim_that_sync_is_paused() {
        let (_runtime, mut app) = test_app(Vec::new());
        app.toggle_noita_warning_preview();
        let ctx = egui::Context::default();

        let output = ctx.run_ui(Default::default(), |ui| super::render(&mut app, ui));
        let labels: Vec<String> = output
            .shapes
            .iter()
            .flat_map(|shape| text_from_shape(&shape.shape))
            .collect();

        assert!(labels.iter().any(|label| label == "Warning preview"));
        assert!(
            labels
                .iter()
                .any(|label| label == "Visual preview only; Noita syncing remains active.")
        );
        assert!(
            !labels
                .iter()
                .any(|label| label == "Configuration-only mode")
        );
        assert!(
            !labels
                .iter()
                .any(|label| label.contains("Noita syncing is paused"))
        );
    }
}
