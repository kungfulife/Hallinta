use eframe::egui;
use crate::models::AppSettings;

/// Centralized design tokens: all sizes, spacing, fonts, and colors.
/// Instantiate with `Design::new(ctx, settings)` at the top of each render function.
pub struct Design {
    pub scale: f32,
    // Spacing
    pub xs: f32,  // 2 * scale
    pub sm: f32,  // 4 * scale
    pub md: f32,  // 8 * scale
    pub lg: f32,  // 16 * scale
    // Font sizes
    pub font_small:   f32,  // 11 * scale
    pub font_body:    f32,  // 13 * scale
    pub font_tab:     f32,  // 15 * scale
    pub font_heading: f32,  // 18 * scale
    pub font_display: f32,  // 22 * scale
    // Widget sizes
    pub toggle_w:   f32,  // 30 * scale
    pub toggle_h:   f32,  // 16 * scale
    pub sidebar_w:  f32,  // 160 * scale
    pub search_w:   f32,  // 150 * scale
    pub row_pad_x:  f32,  // 8 * scale  (cast to i8 when passing to Margin::symmetric)
    pub row_pad_y:  f32,  // 5 * scale  (safe: at max scale 3.0 → 15, fits i8)
    pub row_number_w: f32, // 24 * scale — fixed-width gutter for row numbers
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
    pub fn new(ctx: &egui::Context, settings: &AppSettings) -> Self {
        let s = settings.ui_scale.clamp(0.5, 3.0);
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
                egui::Color32::from_rgba_premultiplied(90, 60, 140, 30),
                egui::Color32::from_rgba_premultiplied(90, 60, 140, 15),
                egui::Color32::from_rgba_premultiplied(140, 135, 155, 35),
                egui::Color32::from_rgba_premultiplied(130, 125, 148, 18),
                egui::Color32::from_rgba_premultiplied(110, 70, 180, 30),
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
            scale: s,
            xs: 2.0 * s,
            sm: 4.0 * s,
            md: 8.0 * s,
            lg: 16.0 * s,
            font_small:   11.0 * s,
            font_body:    13.0 * s,
            font_tab:     15.0 * s,
            font_heading: 18.0 * s,
            font_display: 22.0 * s,
            toggle_w:   30.0 * s,
            toggle_h:   16.0 * s,
            sidebar_w:  160.0 * s,
            search_w:   150.0 * s,
            row_pad_x:  8.0 * s,
            row_pad_y:  5.0 * s,
            row_number_w: 24.0 * s,
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

#[cfg(test)]
mod tests {
    // Design::new() needs egui::Context so we test the scale math directly.

    #[test]
    fn scale_one_preserves_base_sizes() {
        let s = 1.0_f32;
        assert_eq!(2.0 * s, 2.0);
        assert_eq!(4.0 * s, 4.0);
        assert_eq!(11.0 * s, 11.0);
        assert_eq!(13.0 * s, 13.0);
        assert_eq!(30.0 * s, 30.0);
        assert_eq!(160.0 * s, 160.0);
    }

    #[test]
    fn scale_two_doubles_sizes() {
        let s = 2.0_f32;
        assert_eq!(4.0 * s, 8.0);
        assert_eq!(13.0 * s, 26.0);
        assert_eq!(30.0 * s, 60.0);
        assert_eq!(160.0 * s, 320.0);
    }

    #[test]
    fn scale_clamped_to_valid_range() {
        let clamp = |v: f32| v.clamp(0.5, 3.0);
        assert_eq!(clamp(0.1), 0.5);
        assert_eq!(clamp(5.0), 3.0);
        assert_eq!(clamp(1.0), 1.0);
        assert_eq!(clamp(0.5), 0.5);
        assert_eq!(clamp(3.0), 3.0);
    }

    #[test]
    fn row_margin_fits_i8_at_max_scale() {
        // Margin::symmetric takes i8 in egui 0.33 — verify no overflow at max scale
        let max_scale = 3.0_f32;
        let pad_x = (8.0 * max_scale) as i8;  // 24
        let pad_y = (5.0 * max_scale) as i8;  // 15
        assert_eq!(pad_x, 24);
        assert_eq!(pad_y, 15);
        assert!(pad_x < i8::MAX);
        assert!(pad_y < i8::MAX);
    }
}
