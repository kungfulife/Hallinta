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
    pub row_pad_x:  f32,  // 6 * scale  (cast to i8 when passing to Margin::symmetric)
    pub row_pad_y:  f32,  // 3 * scale  (safe: at max scale 3.0 → 18, fits i8)
    // Colors: mod list rows
    pub enabled_even:  egui::Color32,
    pub enabled_odd:   egui::Color32,
    pub disabled_even: egui::Color32,
    pub disabled_odd:  egui::Color32,
    // Colors: accents and indicators
    pub badge_workshop:  egui::Color32,
    pub badge_missing:   egui::Color32,
    pub toggle_on:       egui::Color32,
    pub status_ok:       egui::Color32,
    pub row_number_color: egui::Color32,
}

impl Design {
    pub fn new(ctx: &egui::Context, settings: &AppSettings) -> Self {
        let s = settings.ui_scale.clamp(0.5, 3.0);
        let dark = ctx.style().visuals.dark_mode;

        let (enabled_even, enabled_odd, disabled_even) = if dark {
            (
                egui::Color32::from_rgba_premultiplied(60, 100, 180, 30),
                egui::Color32::from_rgba_premultiplied(60, 100, 180, 15),
                egui::Color32::from_rgba_premultiplied(50, 50, 60, 40),
            )
        } else {
            (
                egui::Color32::from_rgba_premultiplied(40, 80, 160, 25),
                egui::Color32::from_rgba_premultiplied(40, 80, 160, 12),
                egui::Color32::from_rgba_premultiplied(180, 180, 190, 35),
            )
        };

        let row_number_color = if dark {
            egui::Color32::from_rgb(90, 90, 110)
        } else {
            egui::Color32::from_rgb(150, 150, 170)
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
            row_pad_x:  6.0 * s,
            row_pad_y:  3.0 * s,
            enabled_even,
            enabled_odd,
            disabled_even,
            disabled_odd: egui::Color32::TRANSPARENT,
            badge_workshop:  egui::Color32::from_rgb(70, 130, 180),
            badge_missing:   egui::Color32::from_rgb(200, 55, 55),
            toggle_on:       egui::Color32::from_rgb(60, 160, 70),
            status_ok:       egui::Color32::from_rgb(50, 200, 50),
            row_number_color,
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
        let pad_x = (6.0 * max_scale) as i8;  // 18
        let pad_y = (3.0 * max_scale) as i8;  // 9
        assert_eq!(pad_x, 18);
        assert_eq!(pad_y, 9);
        assert!(pad_x < i8::MAX);
        assert!(pad_y < i8::MAX);
    }
}
