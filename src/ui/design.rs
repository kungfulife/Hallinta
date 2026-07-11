use crate::models::AppSettings;
use eframe::egui;

/// Centralized design tokens: all sizes, spacing, fonts, and colors.
/// Instantiate with `Design::new(ctx, settings)` at the top of each render function.
///
/// UI scaling is handled globally via `ctx.set_zoom_factor()` — all values here
/// are base (unscaled) logical pixels. egui multiplies them by the zoom factor
/// automatically, so every widget (including those that don't use Design) scales.
pub struct Design {
    // Spacing (base logical pixels — zoom handles scaling)
    pub xs: f32, // 2
    pub sm: f32, // 4
    pub md: f32, // 8
    pub lg: f32, // 16
    // Font sizes
    pub font_small: f32,   // 11
    pub font_body: f32,    // 13
    pub font_tab: f32,     // 15
    pub font_heading: f32, // 18
    // Widget sizes
    pub toggle_w: f32,     // 30
    pub toggle_h: f32,     // 16
    pub sidebar_w: f32,    // 160
    pub search_w: f32,     // 150
    pub row_pad_x: f32,    // 8
    pub row_pad_y: f32,    // 5
    pub row_number_w: f32, // 24 — fixed-width gutter for row numbers
    // Colors: mod list rows
    pub enabled_even: egui::Color32,
    pub enabled_odd: egui::Color32,
    pub disabled_even: egui::Color32,
    pub disabled_odd: egui::Color32,
    pub disabled_text: egui::Color32,
    // Colors: accents and indicators
    pub badge_workshop: egui::Color32,
    pub badge_missing: egui::Color32,
    pub toggle_on: egui::Color32,
    pub status_ok: egui::Color32,
    pub row_number_color: egui::Color32,
    // Colors: mod list panel background
    pub mod_list_bg: egui::Color32,
    // Colors: drag ghost row
    pub drag_ghost_fill: egui::Color32,
    pub drag_ghost_border: egui::Color32,
    // Colors: header tabs & filters
    pub tab_bg: egui::Color32,
    pub tab_bg_selected: egui::Color32,
    pub tab_text: egui::Color32,
    pub tab_text_selected: egui::Color32,
    pub filter_bg: egui::Color32,
    pub filter_bg_selected: egui::Color32,
    // Colors: settings
    pub settings_focus_bg: egui::Color32,
    pub settings_focus_border: egui::Color32,
    pub helper_text_bg: egui::Color32,
    pub helper_text_color: egui::Color32,
    pub warning_fill: egui::Color32,
    pub warning_border: egui::Color32,
    pub warning_text: egui::Color32,
    pub warning_workspace_fill: egui::Color32,
}

impl Design {
    pub fn new(ctx: &egui::Context, _settings: &AppSettings) -> Self {
        let dark = ctx.global_style().visuals.dark_mode;

        // Enabled rows: purple-tinted alternating stripes (Noita magical energy)
        // Disabled rows: neutral gray alternating stripes
        let (enabled_even, enabled_odd, disabled_even, disabled_odd) = if dark {
            (
                egui::Color32::from_rgba_premultiplied(80, 50, 150, 65),
                egui::Color32::from_rgba_premultiplied(70, 42, 130, 35),
                egui::Color32::from_rgba_premultiplied(58, 52, 68, 50),
                egui::Color32::from_rgba_premultiplied(52, 48, 60, 25),
            )
        } else {
            (
                egui::Color32::from_rgba_premultiplied(80, 50, 140, 40),
                egui::Color32::from_rgba_premultiplied(80, 50, 140, 20),
                egui::Color32::from_rgba_premultiplied(130, 125, 145, 25),
                egui::Color32::from_rgba_premultiplied(120, 115, 135, 12),
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
            font_small: 11.0,
            font_body: 13.0,
            font_tab: 15.0,
            font_heading: 18.0,
            toggle_w: 30.0,
            toggle_h: 16.0,
            sidebar_w: 160.0,
            search_w: 150.0,
            row_pad_x: 8.0,
            row_pad_y: 5.0,
            row_number_w: 24.0,
            enabled_even,
            enabled_odd,
            disabled_even,
            disabled_odd,
            disabled_text,
            badge_workshop: egui::Color32::from_rgb(100, 80, 175),
            badge_missing: egui::Color32::from_rgb(200, 55, 55),
            toggle_on: egui::Color32::from_rgb(60, 160, 70),
            status_ok: egui::Color32::from_rgb(50, 200, 50),
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
            // Tabs use the same lavender family as helper text.
            tab_bg: if dark {
                egui::Color32::from_rgb(50, 40, 75)
            } else {
                egui::Color32::from_rgb(225, 218, 240)
            },
            tab_bg_selected: if dark {
                egui::Color32::from_rgb(75, 50, 130)
            } else {
                egui::Color32::from_rgb(200, 185, 230)
            },
            tab_text: if dark {
                egui::Color32::from_rgb(185, 178, 200)
            } else {
                egui::Color32::from_rgb(70, 50, 110)
            },
            tab_text_selected: if dark {
                egui::Color32::from_rgb(230, 225, 245)
            } else {
                egui::Color32::from_rgb(50, 30, 90)
            },
            // Filters: "All", "Enabled", "Disabled" — lighter tone
            filter_bg: if dark {
                egui::Color32::from_rgb(45, 38, 65)
            } else {
                egui::Color32::from_rgb(235, 230, 248)
            },
            filter_bg_selected: if dark {
                egui::Color32::from_rgb(65, 45, 110)
            } else {
                egui::Color32::from_rgb(215, 205, 238)
            },
            settings_focus_bg: if dark {
                egui::Color32::from_rgba_premultiplied(110, 70, 200, 25)
            } else {
                egui::Color32::from_rgba_premultiplied(100, 65, 160, 18)
            },
            settings_focus_border: if dark {
                egui::Color32::from_rgb(130, 85, 220)
            } else {
                egui::Color32::from_rgb(100, 65, 160)
            },
            helper_text_bg: if dark {
                egui::Color32::from_rgb(50, 40, 75)
            } else {
                egui::Color32::from_rgb(225, 218, 240)
            },
            helper_text_color: if dark {
                egui::Color32::from_rgb(185, 178, 200)
            } else {
                egui::Color32::from_rgb(70, 50, 110)
            },
            warning_fill: if dark {
                egui::Color32::from_rgb(59, 45, 18)
            } else {
                egui::Color32::from_rgb(255, 242, 207)
            },
            warning_border: if dark {
                egui::Color32::from_rgb(215, 169, 61)
            } else {
                egui::Color32::from_rgb(168, 106, 0)
            },
            warning_text: if dark {
                egui::Color32::from_rgb(244, 216, 137)
            } else {
                egui::Color32::from_rgb(112, 70, 0)
            },
            warning_workspace_fill: if dark {
                egui::Color32::from_rgb(35, 30, 23)
            } else {
                egui::Color32::from_rgb(255, 249, 236)
            },
        }
    }

    pub fn font(&self, size: f32) -> egui::FontId {
        egui::FontId::proportional(size)
    }
}

/// Apply the UI zoom factor from settings. Call once per frame in the update loop
/// and once on startup. This scales ALL egui widgets uniformly.
/// Scale offset: stored value 1.25 = user-visible "1.0×".
pub const SCALE_OFFSET: f32 = 0.25;
pub const SCALE_INTERNAL_MIN: f32 = 1.0;
pub const SCALE_INTERNAL_MAX: f32 = 2.25;
pub const SCALE_INTERNAL_DEFAULT: f32 = 1.25;

// Base window sizes at zoom 1.0 (unscaled logical pixels).
// These are multiplied by the actual zoom factor to get the final size.
// At default zoom 1.25: min = 680*1.25 × 520*1.25 = 850×650
pub const BASE_MIN_NORMAL: (f32, f32) = (680.0, 520.0);
pub const BASE_MIN_COMPACT: (f32, f32) = (300.0, 380.0);
pub const BASE_SIZE_NORMAL: (f32, f32) = (880.0, 640.0);
pub const BASE_SIZE_COMPACT: (f32, f32) = (400.0, 420.0);

/// Compute the scaled minimum window size for the current UI scale.
/// Multiplies the base (zoom-1.0 logical pixels) by the zoom factor.
pub fn scaled_min_size(base: (f32, f32), ui_scale: f32) -> egui::Vec2 {
    let scale = ui_scale.clamp(SCALE_INTERNAL_MIN, SCALE_INTERNAL_MAX);
    egui::vec2(base.0 * scale, base.1 * scale)
}

/// Compute the scaled window size for the current UI scale.
pub fn scaled_size(base: (f32, f32), ui_scale: f32) -> egui::Vec2 {
    let scale = ui_scale.clamp(SCALE_INTERNAL_MIN, SCALE_INTERNAL_MAX);
    egui::vec2(base.0 * scale, base.1 * scale)
}

pub fn apply_zoom(ctx: &egui::Context, settings: &AppSettings) {
    let scale = settings
        .ui_scale
        .clamp(SCALE_INTERNAL_MIN, SCALE_INTERNAL_MAX);
    if (ctx.zoom_factor() - scale).abs() > 0.001 {
        ctx.set_zoom_factor(scale);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaled_sizes_use_clamped_ui_scale() {
        let base = (10.0, 20.0);

        assert_eq!(
            super::scaled_size(base, 0.5),
            eframe::egui::vec2(10.0, 20.0)
        );
        assert_eq!(
            super::scaled_min_size(base, 5.0),
            eframe::egui::vec2(22.5, 45.0)
        );
        assert_eq!(
            super::scaled_size(base, 1.25),
            eframe::egui::vec2(12.5, 25.0)
        );
    }

    #[test]
    fn compact_min_height_prevents_action_clipping() {
        assert!(
            super::BASE_MIN_COMPACT.1 >= 300.0,
            "compact minimum height must fit preset controls plus all action buttons"
        );
    }
}
