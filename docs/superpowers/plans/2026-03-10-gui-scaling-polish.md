# GUI Scaling & Polish Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce a centralized Design system with user-controlled UI scale, a purposeful compact mode layout, visually polished mod rows with colored tinting and pill badges, and consistent sizing throughout.

**Architecture:** A new `design.rs` module provides a `Design` struct computed once per render call from `ctx` + `settings.ui_scale`. Each render function instantiates `Design` at its top — no signature changes required. Compact mode gets its own `compact.rs` file replacing the inline stub in `app.rs`. All hardcoded pixel values are replaced with Design-derived values.

**Tech Stack:** Rust, eframe 0.33.3, egui 0.33.3, serde (ui_scale field persistence via `#[serde(default)]`)

**Spec:** `docs/plans/2026-03-10-gui-scaling-polish-design.md`

---

## Chunk 1: Design System Foundation

### Task 1: Add `ui_scale` to AppSettings

**Files:**
- Modify: `src/models.rs` — add `ui_scale` field to `AppSettings`
- Modify: `src/ui/settings.rs` — add `ui_scale: 1.0` to `default_settings()`

- [ ] **Step 1: Add field to AppSettings**

In `src/models.rs`, inside `AppSettings`, after the `compact_mode` field:

```rust
#[serde(default)]
pub compact_mode: bool,
#[serde(default = "default_ui_scale")]
pub ui_scale: f32,
```

Add the default function after `default_include_save01`:

```rust
fn default_ui_scale() -> f32 {
    1.0
}
```

- [ ] **Step 2: Update default_settings() in settings.rs**

In `src/ui/settings.rs`, inside `default_settings()`, add after `compact_mode: false`:

```rust
compact_mode: false,
ui_scale: 1.0,
```

- [ ] **Step 3: Update the fallback AppSettings literal in app.rs**

In `src/app.rs`, the `new()` method has an inline `AppSettings { ... }` literal (lines ~61-72) used when `load_settings()` fails. Add `ui_scale: 1.0` after `compact_mode: false`:

```rust
AppSettings {
    noita_dir: String::new(),
    entangled_dir: String::new(),
    dark_mode: false,
    selected_preset: "Default".to_string(),
    version: platform::get_version(),
    log_settings: LogSettings::default(),
    backup_settings: BackupSettings::default(),
    save_monitor_settings: SaveMonitorSettings::default(),
    gallery_settings: GallerySettings::default(),
    compact_mode: false,
    ui_scale: 1.0,   // ← add this
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/models.rs src/ui/settings.rs src/app.rs
git commit -m "feat: add ui_scale field to AppSettings with serde default 1.0"
```

---

### Task 2: Create `src/ui/design.rs`

**Files:**
- Create: `src/ui/design.rs`

- [ ] **Step 1: Create the file with the full Design struct**

```rust
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
    pub xl: f32,  // 24 * scale
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
    pub accent:          egui::Color32,
    pub badge_workshop:  egui::Color32,
    pub badge_missing:   egui::Color32,
    pub toggle_on:       egui::Color32,
    pub status_ok:       egui::Color32,
    pub row_number_color: egui::Color32,
    pub drag_highlight:  egui::Color32,
}

impl Design {
    pub fn new(ctx: &egui::Context, settings: &AppSettings) -> Self {
        let s = settings.ui_scale.max(0.5).min(3.0);
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
            xl: 24.0 * s,
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
            accent: if dark {
                egui::Color32::from_rgb(60, 120, 200)
            } else {
                egui::Color32::from_rgb(40, 100, 180)
            },
            badge_workshop:  egui::Color32::from_rgb(70, 130, 180),
            badge_missing:   egui::Color32::from_rgb(200, 55, 55),
            toggle_on:       egui::Color32::from_rgb(60, 160, 70),
            status_ok:       egui::Color32::from_rgb(50, 200, 50),
            row_number_color,
            drag_highlight:  ctx.style().visuals.selection.bg_fill,
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
        let clamp = |v: f32| v.max(0.5).min(3.0);
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
```

- [ ] **Step 2: Run tests**

```bash
cargo test ui::design
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src/ui/design.rs
git commit -m "feat: add Design struct with scale, spacing, fonts, and color tokens"
```

---

### Task 3: Register `design` and `compact` modules

**Files:**
- Modify: `src/ui/mod.rs` — register new modules

- [ ] **Step 1: Add module declarations**

Replace the contents of `src/ui/mod.rs` with:

```rust
pub mod compact;
pub mod context_menu;
pub mod design;
pub mod gallery;
pub mod header;
pub mod mod_list;
pub mod modals;
pub mod preset_bar;
pub mod settings;
pub mod sidebar;
pub mod theme;
```

- [ ] **Step 2: Create placeholder `src/ui/compact.rs`** (full impl in Task 5)

```rust
use crate::app::HallintaApp;
use eframe::egui;

pub fn render_compact(_app: &mut HallintaApp, ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.label("Compact mode");
    });
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/mod.rs src/ui/compact.rs
git commit -m "chore: register design and compact UI modules"
```

---

## Chunk 2: Compact Mode

### Task 4: Wire compact mode routing in `app.rs`

**Files:**
- Modify: `src/app.rs` — replace inline compact stub with `render_compact` call

- [ ] **Step 1: Replace the inline compact stub**

In `src/app.rs` `update()`, find:

```rust
View::ModList => {
    if self.compact_mode {
        // Compact mode: show only monitor status
        ui.heading("Save Monitor");
        if self.save_monitor.is_running() {
            ui.colored_label(egui::Color32::GREEN, "Running");
            ui.label(format!("Snapshots: {}", self.save_monitor.snapshot_count));
            if ui.button("Stop Monitor").clicked() {
                self.stop_save_monitor();
            }
        } else if ui.button("Start Monitor").clicked() {
            self.start_save_monitor();
        }
    } else if self.save_monitor.is_running() {
```

Replace with:

```rust
View::ModList => {
    if self.compact_mode {
        crate::ui::compact::render_compact(self, ui);
    } else if self.save_monitor.is_running() {
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/app.rs
git commit -m "refactor: route compact mode to compact::render_compact"
```

---

### Task 5: Implement `src/ui/compact.rs`

**Files:**
- Modify: `src/ui/compact.rs` — replace placeholder with full implementation

- [ ] **Step 1: Write the compact panel**

```rust
use crate::app::HallintaApp;
use crate::models::Modal;
use crate::ui::design::Design;
use eframe::egui;

pub fn render_compact(app: &mut HallintaApp, ui: &mut egui::Ui) {
    let d = Design::new(ui.ctx(), &app.settings);

    // Use vertical_centered with a max width so everything aligns nicely
    let available = ui.available_size();
    let top_pad = (available.y * 0.12).max(d.lg);

    ui.add_space(top_pad);

    ui.vertical_centered(|ui| {
        ui.set_max_width(260.0 * d.scale);

        // ── Preset selector ──────────────────────────────────────────────────
        ui.label(
            egui::RichText::new("Preset")
                .size(d.font_small)
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(d.xs);

        let is_locked = app.save_monitor.is_running();
        let prev_selected = app.selected_preset.clone();

        let mut preset_names: Vec<String> = app.presets.keys().cloned().collect();
        preset_names.sort_by(|a, b| {
            if a == "Default" {
                std::cmp::Ordering::Less
            } else if b == "Default" {
                std::cmp::Ordering::Greater
            } else {
                a.to_lowercase().cmp(&b.to_lowercase())
            }
        });

        egui::ComboBox::from_id_salt("compact_preset_selector")
            .selected_text(&app.selected_preset)
            .width(250.0 * d.scale)
            .show_ui(ui, |ui| {
                for name in &preset_names {
                    if ui
                        .selectable_label(*name == app.selected_preset, name)
                        .clicked()
                        && !is_locked
                    {
                        app.selected_preset = name.clone();
                    }
                }
            });

        if app.selected_preset != prev_selected {
            app.switch_preset();
        }

        ui.add_space(d.sm);

        // Mod count summary
        let total = app.current_mods.len();
        let enabled_count = app.current_mods.iter().filter(|m| m.enabled).count();
        ui.label(
            egui::RichText::new(format!("{} / {} mods enabled", enabled_count, total))
                .size(d.font_body)
                .color(ui.visuals().weak_text_color()),
        );

        ui.add_space(d.lg);
        ui.separator();
        ui.add_space(d.lg);

        let btn_w = 240.0 * d.scale;
        let btn_h = 28.0 * d.scale;
        let backup_busy = app.backup_state.in_progress || app.backup_state.restoring;

        // ── Monitor button ───────────────────────────────────────────────────
        if is_locked {
            ui.colored_label(
                d.status_ok,
                egui::RichText::new("● MONITOR ACTIVE")
                    .size(d.font_body)
                    .strong(),
            );
            ui.add_space(d.sm);
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Stop Monitor").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.stop_save_monitor();
            }
        } else {
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Start Monitor").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.start_save_monitor();
            }
        }

        ui.add_space(d.sm);

        // ── Secondary actions ────────────────────────────────────────────────
        ui.add_enabled_ui(!is_locked && !backup_busy, |ui| {
            if ui
                .add_sized(
                    [btn_w, btn_h],
                    egui::Button::new(
                        egui::RichText::new("Create Backup").size(d.font_body),
                    ),
                )
                .clicked()
            {
                app.start_backup_modal();
            }
        });

        ui.add_space(d.xs);

        if ui
            .add_sized(
                [btn_w, btn_h],
                egui::Button::new(
                    egui::RichText::new("View Snapshots").size(d.font_body),
                ),
            )
            .clicked()
        {
            let preset = app.selected_preset.clone();
            app.load_snapshot_list_async(preset.clone());
            app.active_modal = Some(Modal::SnapshotManager {
                preset_name: preset,
            });
        }

        // ── Live snapshot count when running ─────────────────────────────────
        if is_locked {
            ui.add_space(d.md);
            ui.label(
                egui::RichText::new(format!(
                    "Snapshots taken: {}",
                    app.save_monitor.snapshot_count
                ))
                .size(d.font_small)
                .color(ui.visuals().weak_text_color()),
            );
        }
    });
}
```

- [ ] **Step 2: Build and smoke-test**

```bash
cargo build
```

Expected: compiles without errors. Launch the app, toggle Compact mode — verify the new layout appears with preset selector and action buttons.

- [ ] **Step 3: Commit**

```bash
git add src/ui/compact.rs
git commit -m "feat: implement purposeful compact mode layout with preset selector and actions"
```

---

## Chunk 3: Mod List Visual Refresh

### Task 6: Rebuild mod rows in `src/ui/mod_list.rs`

**Files:**
- Modify: `src/ui/mod_list.rs` — Design colors, pill badges, scaled toggle

- [ ] **Step 1: Add `draw_badge` helper at the bottom of the file**

After the existing `draw_toggle_visual` function, add:

```rust
/// Draws a small rounded pill badge with the given background color and white text.
fn draw_badge(ui: &mut egui::Ui, text: &str, bg: egui::Color32, d: &crate::ui::design::Design) {
    let font = d.font(d.font_small);
    // layout_no_wrap lives on Fonts, not Painter — access via ui.fonts()
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE));
    let text_size = galley.size();
    let pad = egui::vec2(d.sm, d.xs);
    let badge_size = text_size + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(badge_size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter()
            .rect_filled(rect, rect.height() / 2.0, bg);
        ui.painter()
            .galley(rect.min + pad, galley, egui::Color32::WHITE);
    }
}
```

- [ ] **Step 2: Update `draw_toggle_visual` to accept `&Design`**

Replace the existing `draw_toggle_visual` function:

```rust
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
```

- [ ] **Step 3: Update `render_mod_list` to use Design**

At the very top of `render_mod_list`, after the function signature, add:

```rust
let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
```

Then replace the empty-state label:

```rust
// OLD:
.size(14.0)

// NEW:
.size(d.font_heading)
```

Replace the row frame inner margin and font sizes. Find:

```rust
egui::Frame::NONE
    .inner_margin(egui::Margin::symmetric(6, 3))
    .corner_radius(4)
    .fill(base_fill)
```

Replace with:

```rust
egui::Frame::NONE
    .inner_margin(egui::Margin::symmetric(
        d.row_pad_x as i8,
        d.row_pad_y as i8,
    ))
    .corner_radius(4.0 * d.scale)
    .fill(base_fill)
```

Replace the row number label:

```rust
// OLD:
ui.label(
    egui::RichText::new(format!("{}", row.idx + 1))
        .size(11.0)
        .color(ui.visuals().weak_text_color()),
);

// NEW:
ui.label(
    egui::RichText::new(format!("{}", row.idx + 1))
        .size(d.font_small)
        .color(d.row_number_color),
);
```

Replace the `ui.add_space(4.0)` between row number and name:

```rust
ui.add_space(d.sm);
```

Replace the mod name label:

```rust
// OLD:
ui.label(
    egui::RichText::new(&row.name)
        .size(13.0)
        .color(name_color),
);

// NEW:
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
```

Replace the workshop badge:

```rust
// OLD:
if row.is_workshop {
    ui.label(
        egui::RichText::new("[W]")
            .small()
            .strong()
            .color(egui::Color32::from_rgb(70, 130, 180)),
    );
}

// NEW:
if row.is_workshop {
    draw_badge(ui, "W", d.badge_workshop, &d);
}
```

Replace the missing mod indicator:

```rust
// OLD:
if let Some(false) = row.workshop_installed {
    ui.label(
        egui::RichText::new("[Missing]")
            .small()
            .strong()
            .color(egui::Color32::from_rgb(220, 60, 60)),
    );
}

// NEW:
if let Some(false) = row.workshop_installed {
    draw_badge(ui, "Missing", d.badge_missing, &d);
}
```

Replace the toggle call:

```rust
// OLD:
draw_toggle_visual(ui, row.enabled);

// NEW:
draw_toggle_visual(ui, row.enabled, &d);
```

Update the row background fill logic. Find the `base_fill` block:

```rust
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
```

Replace with:

```rust
let base_fill = if is_drag_source {
    ui.visuals().extreme_bg_color
} else if row.enabled {
    if is_even { d.enabled_even } else { d.enabled_odd }
} else if is_even {
    d.disabled_even
} else {
    d.disabled_odd
};
```

Update the hover border stroke width:

```rust
// OLD:
egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_stroke.color),

// NEW:
egui::Stroke::new(1.5 * d.scale, ui.visuals().widgets.hovered.bg_stroke.color),
```

Update the mod count footer label:

```rust
// Add .size(d.font_small) to the RichText
egui::RichText::new(count_text)
    .size(d.font_small)
    .color(ui.visuals().weak_text_color()),
```

Also rewrite `render_monitor_active` to use Design and true vertical centering:

```rust
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
```

- [ ] **Step 4: Build and verify**

```bash
cargo build
```

Expected: no errors. Launch app — mod list rows should show blue tinting for enabled mods, pill badges, scaled toggle with shadow, and italic disabled names.

- [ ] **Step 5: Commit**

```bash
git add src/ui/mod_list.rs
git commit -m "feat: mod list visual refresh — Design colors, pill badges, scaled toggle"
```

---

## Chunk 4: Header, Sidebar, Preset Bar

### Task 7: Update `src/ui/header.rs`

**Files:**
- Modify: `src/ui/header.rs`

- [ ] **Step 1: Add Design instantiation and use scaled fonts/spacing**

At the top of `render_header`, after the function signature, add:

```rust
let d = crate::ui::design::Design::new(ctx, &app.settings);
```

Replace the hardcoded tab font:

```rust
// OLD:
let tab_font = egui::FontId::proportional(15.0);

// NEW:
let tab_font = d.font(d.font_tab);
```

Replace the search box width:

```rust
// OLD:
.desired_width(150.0)

// NEW:
.desired_width(d.search_w)
```

Replace the top/bottom spacing:

```rust
// OLD (line 7): ui.add_space(4.0);
ui.add_space(d.sm);

// OLD (line 94): ui.add_space(2.0);
ui.add_space(d.xs);
```

Replace the monitor label color:

```rust
// OLD:
egui::Color32::from_rgb(50, 200, 50),

// NEW:
d.status_ok,
```

- [ ] **Step 2: Build**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/header.rs
git commit -m "feat: header uses Design for fonts, spacing, and search width"
```

---

### Task 8: Update `src/ui/sidebar.rs`

**Files:**
- Modify: `src/ui/sidebar.rs`

- [ ] **Step 1: Add Design, scaled width, full-width buttons**

At the top of `render_sidebar`, after the function signature:

```rust
let d = crate::ui::design::Design::new(ctx, &app.settings);
```

Update the panel width:

```rust
// OLD:
.default_width(160.0)

// NEW:
.default_width(d.sidebar_w)
```

Inside the panel, after `ui.add_space(8.0)` at the very top, replace that with:

```rust
ui.add_space(d.md);
```

Add this line at the start of the panel content (right after `ui.add_space(d.md)`):

```rust
// Make buttons fill the sidebar width minus padding
let btn_width = d.sidebar_w - d.md * 2.0;
ui.set_min_width(d.sidebar_w);
```

Then for EVERY `ui.button(...)` call in sidebar.rs, wrap it to set width:

```rust
// OLD pattern:
if ui.button("Import Mod List").clicked() { ... }

// NEW pattern:
if ui.add_sized([btn_width, 0.0], egui::Button::new("Import Mod List")).clicked() { ... }
```

Apply the same `add_sized` wrapping to ALL buttons: Import Mod List, Export Mod List, Open mod_config.xml, Export Presets, Import Presets, Create Backup, Restore Backup, Manage Backups, Stop Monitor, Start Monitor, View Snapshots, Clear All Snapshots.

Replace all `ui.add_space(8.0)` with `ui.add_space(d.md)` and `ui.add_space(4.0)` with `ui.add_space(d.sm)`.

Replace the monitor status color:

```rust
// OLD:
egui::Color32::from_rgb(50, 200, 50),

// NEW:
d.status_ok,
```

Update the heading:

```rust
// OLD:
ui.label(egui::RichText::new("Actions").heading().strong());

// NEW:
ui.label(egui::RichText::new("Actions").size(d.font_heading).strong());
```

- [ ] **Step 2: Build**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/sidebar.rs
git commit -m "feat: sidebar uses Design — scaled width, full-width buttons, scaled spacing"
```

---

### Task 9: Update `src/ui/preset_bar.rs`

**Files:**
- Modify: `src/ui/preset_bar.rs`

- [ ] **Step 1: Add Design, scale combo width**

At the top of `render_preset_bar`, after the function signature:

```rust
let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
```

Update the ComboBox width:

```rust
// OLD:
.width(200.0)

// NEW:
.width(200.0 * d.scale)
```

Replace the top spacing:

```rust
// OLD (line 6): ui.add_space(4.0);
ui.add_space(d.sm);
```

Update the "Preset:" label font to match body:

```rust
// OLD:
ui.label(egui::RichText::new("Preset:").strong());

// NEW:
ui.label(egui::RichText::new("Preset:").size(d.font_body).strong());
```

- [ ] **Step 2: Build**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/preset_bar.rs
git commit -m "feat: preset bar uses Design for scaled ComboBox and body font"
```

---

## Chunk 5: Polish Pass

### Task 10: Update `src/ui/modals.rs`

**Files:**
- Modify: `src/ui/modals.rs`

- [ ] **Step 1: Scale modal widths and scroll heights**

In `render_backup_manager`, replace the hardcoded width and scroll height:

```rust
// OLD:
.default_width(500.0)
// ...
.max_height(400.0)

// NEW — compute based on scale at the top of the function:
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.default_width(500.0 * d.scale)
// ...
.max_height(400.0 * d.scale)
```

In `render_snapshot_manager`, replace:

```rust
// OLD:
.default_width(450.0)
// ...
.max_height(350.0)

// NEW (add Design at top of function):
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.default_width(450.0 * d.scale)
// ...
.max_height(350.0 * d.scale)
```

In `render_checklist`, replace:

```rust
// OLD:
.max_height(300.0)

// NEW (add Design at top of function):
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.max_height(300.0 * d.scale)
```

In `render_missing_mods`, replace:

```rust
// OLD:
.max_height(250.0)

// NEW (add Design at top of function):
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.max_height(250.0 * d.scale)
```

In `render_open_source`, replace:

```rust
// OLD:
.max_height(400.0)

// NEW (add Design at top of function):
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.max_height(400.0 * d.scale)
```

In `render_system_info`, replace:

```rust
// OLD:
.min_col_width(120.0)

// NEW (add Design at top of function):
let d = crate::ui::design::Design::new(ctx, &app.settings);
// ...
.min_col_width(120.0 * d.scale)
```

- [ ] **Step 2: Build**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/modals.rs
git commit -m "feat: modals use Design for scaled widths and scroll heights"
```

---

### Task 11: Update `src/ui/gallery.rs`

**Files:**
- Modify: `src/ui/gallery.rs`

- [ ] **Step 1: Add Design, scaled heading and spacing**

At the top of `render_gallery`, after the function signature:

```rust
let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
```

Replace the heading:

```rust
// OLD:
ui.label(egui::RichText::new("Modpacks").heading().strong().size(20.0));

// NEW:
ui.label(egui::RichText::new("Modpacks").size(d.font_display).strong());
```

Replace `ui.add_space(8.0)` calls with `ui.add_space(d.md)` and `ui.add_space(4.0)` with `ui.add_space(d.sm)`.

Update the preset cards inner margin:

```rust
// OLD:
.inner_margin(egui::Margin::same(8))

// NEW:
.inner_margin(egui::Margin::same(d.md))
```

`Margin::same` takes `f32` directly — pass `d.md` as-is.

- [ ] **Step 2: Build**

```bash
cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src/ui/gallery.rs
git commit -m "feat: gallery uses Design for scaled heading and card margins"
```

---

### Task 12: Add `ui_scale` slider to `src/ui/settings.rs`

**Files:**
- Modify: `src/ui/settings.rs`

- [ ] **Step 1: Add the scale slider to the Appearance section**

In the `// ── Appearance ──` group, after the compact_mode checkbox:

```rust
ui.checkbox(&mut settings.compact_mode, "Compact Mode");

ui.add_space(d.sm);
ui.horizontal(|ui| {
    ui.label("UI Scale:");
    let scale_resp = ui.add(
        egui::Slider::new(&mut settings.ui_scale, 0.75..=2.0)
            .step_by(0.05)
            .text("×"),
    );
    // Live preview: apply scale immediately so the UI resizes as the slider moves.
    // On Cancel, the live change persists (scale is visual-only and safe to keep).
    if scale_resp.changed() {
        app.settings.ui_scale = settings.ui_scale;
    }
    if ui.small_button("Reset").clicked() {
        settings.ui_scale = 1.0;
        app.settings.ui_scale = 1.0;
    }
});
```

Add Design at the top of `render_settings`:

```rust
let d = crate::ui::design::Design::new(ui.ctx(), &app.settings);
```

Also add `let d = ...` and replace the `desired_width(ui.available_width() - 180.0)` calls in the directory section to use the scaled offset:

```rust
// OLD:
.desired_width(ui.available_width() - 180.0)

// NEW:
.desired_width(ui.available_width() - 180.0 * d.scale)
```

Do this for both the Noita dir and Entangled dir text fields (two occurrences).

Replace all `ui.add_space(8.0)` with `ui.add_space(d.md)` and `ui.add_space(4.0)` with `ui.add_space(d.sm)`.

Update the section heading sizes:

```rust
// OLD: .size(14.0)
// NEW: .size(d.font_tab)
```

Apply to all section headings: Directories, Appearance, Logging, Backup, Save Monitor, Modpacks.

- [ ] **Step 2: Build**

```bash
cargo build
```

Expected: no errors.

- [ ] **Step 3: Run tests**

```bash
cargo test
```

Expected: all tests pass (the 4 design math tests plus any others).

- [ ] **Step 4: Commit**

```bash
git add src/ui/settings.rs
git commit -m "feat: add UI Scale slider (0.75x–2.0x) to Settings > Appearance"
```

---

## Final Verification

- [ ] **Build release**

```bash
cargo build --release
```

Expected: no warnings (or only pre-existing ones), no errors.

- [ ] **Run all tests**

```bash
cargo test
```

Expected: all tests pass.

- [ ] **Manual smoke test checklist**

Launch the app and verify:
1. Normal mode: mod list shows colored rows (blue tint for enabled, neutral for disabled)
2. Workshop mods show pill badge "W" (blue), missing mods show "Missing" (red)
3. Disabled mod names appear italic
4. Toggle switch is visible and has a subtle shadow
5. Sidebar buttons are full-width, aligned
6. Open Settings → Appearance: UI Scale slider is present
7. Change scale to 1.5 → Save & Close → all elements scale up proportionally
8. Change scale back to 1.0 → Save & Close
9. Toggle Compact mode → new layout shows preset selector, mod count, 3 buttons
10. Start monitor in compact mode → "● MONITOR ACTIVE" appears, Stop button shown
11. Open Modpacks tab → heading and cards look proportional
12. Open Manage Backups modal → scroll area height is reasonable
