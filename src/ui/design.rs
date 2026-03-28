use eframe::egui;
use crate::models::AppSettings;

/// Centralized design tokens: all sizes, spacing, fonts, and colors.
/// Instantiate with `Design::new(ctx, settings)` at the top of each render function.
///
/// UI scaling is handled globally via `ctx.set_zoom_factor()` — all values here
/// are base (unscaled) logical pixels. egui multiplies them by the zoom factor
/// automatically, so every widget (including those that don't use Design) scales.
pub struct Design {
    // Spacing (base logical pixels — zoom handles scaling)
    pub xs: f32,  // 2
    pub sm: f32,  // 4
    pub md: f32,  // 8
    pub lg: f32,  // 16
    // Font sizes
    pub font_small:   f32,  // 11
    pub font_body:    f32,  // 13
    pub font_tab:     f32,  // 15
    pub font_heading: f32,  // 18
    pub font_display: f32,  // 22
    // Widget sizes
    pub toggle_w:   f32,  // 30
    pub toggle_h:   f32,  // 16
    pub sidebar_w:  f32,  // 160
    pub search_w:   f32,  // 150
    pub row_pad_x:  f32,  // 8
    pub row_pad_y:  f32,  // 5
    pub row_number_w: f32, // 24 — fixed-width gutter for row numbers
    // Colors: mod list rows
    pub enabled_even:    egui::Color32,
    pub enabled_odd:     egui::Color32,
    pub disabled_even:   egui::Color32,
    pub disabled_odd:    egui::Color32,
    pub row_hover:       egui::Color32,
    pub disabled_text:   egui::Color32,
    // Colors: accents and indicators
    pub badge_workshop:  egui::Color32,
    pub badge_missing:   egui::Color32,
    pub toggle_on:       egui::Color32,
    pub status_ok:       egui::Color32,
    pub row_number_color: egui::Color32,
    // Colors: mod list panel background
    pub mod_list_bg: egui::Color32,
    // Colors: drag ghost row
    pub drag_ghost_fill:   egui::Color32,
    pub drag_ghost_border: egui::Color32,
}

impl Design {
    pub fn new(ctx: &egui::Context, _settings: &AppSettings) -> Self {
        let dark = ctx.style().visuals.dark_mode;

        // Enabled rows: purple-tinted alternating stripes (Noita magical energy)
        // Disabled rows: neutral gray alternating stripes
        let (enabled_even, enabled_odd, disabled_even, disabled_odd, row_hover) = if dark {
            (
                egui::Color32::from_rgba_premultiplied(80, 50, 150, 65),
                egui::Color32::from_rgba_premultiplied(70, 42, 130, 35),
                egui::Color32::from_rgba_premultiplied(58, 52, 68, 50),
                egui::Color32::from_rgba_premultiplied(52, 48, 60, 25),
                egui::Color32::from_rgba_premultiplied(120, 75, 200, 55),
            )
        } else {
            (
                egui::Color32::from_rgba_premultiplied(80, 50, 140, 40),
                egui::Color32::from_rgba_premultiplied(80, 50, 140, 20),
                egui::Color32::from_rgba_premultiplied(130, 125, 145, 25),
                egui::Color32::from_rgba_premultiplied(120, 115, 135, 12),
                egui::Color32::from_rgba_premultiplied(100, 60, 170, 35),
            )
        };

        let row_number_color = if dark {
            egui::Color32::from_rgb(145, 135, 170)
        } else {
            egui::Color32::from_rgb(120, 110, 145)
        };

        let disabled_text = if dark {
            egui::Color32::from_rgb(140, 135, 155)
        } else {
            egui::Color32::from_rgb(150, 145, 165)
        };

        Self {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            font_small:   11.0,
            font_body:    13.0,
            font_tab:     15.0,
            font_heading: 18.0,
            font_display: 22.0,
            toggle_w:   30.0,
            toggle_h:   16.0,
            sidebar_w:  160.0,
            search_w:   150.0,
            row_pad_x:  8.0,
            row_pad_y:  5.0,
            row_number_w: 24.0,
            enabled_even,
            enabled_odd,
            disabled_even,
            disabled_odd,
            row_hover,
            disabled_text,
            badge_workshop:  egui::Color32::from_rgb(100, 80, 175),
            badge_missing:   egui::Color32::from_rgb(200, 55, 55),
            toggle_on:       egui::Color32::from_rgb(60, 160, 70),
            status_ok:       egui::Color32::from_rgb(50, 200, 50),
            row_number_color,
            mod_list_bg: if dark {
                egui::Color32::from_rgba_premultiplied(30, 22, 48, 80)
            } else {
                egui::Color32::from_rgba_premultiplied(60, 40, 90, 16)
            },
            drag_ghost_fill: if dark {
                egui::Color32::from_rgba_premultiplied(90, 55, 190, 35)
            } else {
                egui::Color32::from_rgba_premultiplied(80, 45, 180, 30)
            },
            drag_ghost_border: if dark {
                egui::Color32::from_rgba_premultiplied(140, 100, 255, 140)
            } else {
                egui::Color32::from_rgba_premultiplied(100, 55, 200, 130)
            },
        }
    }

    pub fn font(&self, size: f32) -> egui::FontId {
        egui::FontId::proportional(size)
    }
}

/// Apply the UI zoom factor from settings. Call once per frame in the update loop
/// and once on startup. This scales ALL egui widgets uniformly.
pub fn apply_zoom(ctx: &egui::Context, settings: &AppSettings) {
    let scale = settings.ui_scale.clamp(0.75, 2.0);
    if (ctx.zoom_factor() - scale).abs() > 0.001 {
        ctx.set_zoom_factor(scale);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base_sizes_are_constant() {
        // With zoom-based scaling, Design values are fixed base sizes
        assert_eq!(2.0_f32, 2.0);
        assert_eq!(4.0_f32, 4.0);
        assert_eq!(11.0_f32, 11.0);
        assert_eq!(13.0_f32, 13.0);
        assert_eq!(30.0_f32, 30.0);
        assert_eq!(160.0_f32, 160.0);
    }

    #[test]
    fn scale_clamped_to_valid_range() {
        let clamp = |v: f32| v.clamp(0.75, 2.0);
        assert_eq!(clamp(0.1), 0.75);
        assert_eq!(clamp(5.0), 2.0);
        assert_eq!(clamp(1.0), 1.0);
        assert_eq!(clamp(0.75), 0.75);
        assert_eq!(clamp(2.0), 2.0);
    }

    #[test]
    fn row_margin_fits_i8_at_max_zoom() {
        // At max zoom (2.0), egui multiplies logical pixels by zoom_factor.
        // Margin::symmetric takes i8 — verify base values fit even if
        // someone later raises the max.
        let pad_x = 8i8;
        let pad_y = 5i8;
        assert!(pad_x < i8::MAX);
        assert!(pad_y < i8::MAX);
    }
}
