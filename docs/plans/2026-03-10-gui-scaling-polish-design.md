# GUI Scaling & Polish — Design Document
Date: 2026-03-10
Branch: egui-overwrite

## Problem Statement

The current UI has four issues:
1. All sizes are hardcoded logical pixels with no user-controllable scale
2. Compact mode just hides controls — it has no purposeful layout
3. Centering is inconsistent; key elements aren't visually anchored
4. Mod list rows are visually flat and lack color-based hierarchy

## Approach: Design System + Targeted Refresh

Introduce a thin `design.rs` module that centralizes all sizes, spacing, fonts,
and colors as a `Design` struct computed once per frame. Apply it across all UI
files in a single pass. Compact mode gets a purpose-built layout. Mod rows get
a visual refresh.

---

## Section 1: Design System (`src/ui/design.rs`)

### Scale Factor

A `ui_scale: f32` field added to `AppSettings` (default `1.0`, range 0.75–2.0).
A slider in Settings → Appearance lets users dial it up/down. This is the
missing "make the UI bigger/smaller" control.

```rust
pub struct Design {
    pub scale: f32,
    // Spacing
    pub xs: f32,   // 2 * scale
    pub sm: f32,   // 4 * scale
    pub md: f32,   // 8 * scale
    pub lg: f32,   // 16 * scale
    pub xl: f32,   // 24 * scale
    // Font sizes
    pub font_small:   f32,   // 11 * scale
    pub font_body:    f32,   // 13 * scale
    pub font_tab:     f32,   // 15 * scale
    pub font_heading: f32,   // 18 * scale
    pub font_display: f32,   // 22 * scale
    // Widget sizes
    pub toggle_w:     f32,   // 30 * scale
    pub toggle_h:     f32,   // 16 * scale
    pub sidebar_w:    f32,   // 160 * scale
    pub search_w:     f32,   // 150 * scale
    pub row_pad_x:    i8,    // (6 * scale) as i8
    pub row_pad_y:    i8,    // (3 * scale) as i8
    // Colors
    pub enabled_even:    Color32,
    pub enabled_odd:     Color32,
    pub disabled_even:   Color32,
    pub disabled_odd:    Color32,
    pub accent:          Color32,
    pub badge_workshop:  Color32,
    pub badge_missing:   Color32,
    pub toggle_on:       Color32,
    pub status_ok:       Color32,
    pub row_number:      Color32,
    pub drag_highlight:  Color32,
}

impl Design {
    pub fn new(ctx: &egui::Context, settings: &AppSettings) -> Self { ... }
    pub fn font(&self, size: f32) -> FontId { FontId::proportional(size) }
}
```

`Design::new()` is called once at the top of `HallintaApp::update()` and passed
as `&d` to every `render_*` function.

### Color Palette

Colors are defined for both dark and light themes. Example dark palette:

| Token            | Dark Mode (RGB)        | Light Mode (RGB)         | Use                        |
|------------------|------------------------|--------------------------|----------------------------|
| `enabled_even`   | accent @ 18% opacity   | accent @ 10% opacity     | Enabled mod, even row      |
| `enabled_odd`    | accent @ 10% opacity   | accent @ 5% opacity      | Enabled mod, odd row       |
| `disabled_even`  | faint_bg_color         | faint_bg_color           | Disabled mod, even row     |
| `disabled_odd`   | transparent            | transparent              | Disabled mod, odd row      |
| `badge_workshop` | (70, 130, 180)         | (50, 100, 160)           | [W] workshop badge         |
| `badge_missing`  | (220, 60, 60)          | (190, 40, 40)            | [Missing] badge            |
| `toggle_on`      | (60, 160, 70)          | (40, 140, 55)            | Toggle switch, enabled     |
| `status_ok`      | (50, 200, 50)          | (30, 160, 30)            | Monitor active indicator   |
| `drag_highlight` | selection.bg_fill      | selection.bg_fill        | Drag source border         |

---

## Section 2: Compact Mode Redesign

Current compact mode hides tabs/search/filter and shows Start/Stop buttons.
It looks unfinished — the layout is a void with a couple of buttons floating.

### New Layout (480×400 window)

```
┌──────────────────────────────────────┐
│  [● MONITOR ACTIVE]        [Normal]  │  header row — status left, toggle right
├──────────────────────────────────────┤
│  Preset: [Default          ▾]        │  full-width preset combo
│  12 / 34 mods enabled                │  summary line
├──────────────────────────────────────┤
│  [    Start Monitor    ]             │  primary action, full width
│  [    Create Backup    ]             │  secondary actions
│  [    View Snapshots   ]             │
└──────────────────────────────────────┘
```

Key decisions:
- Preset dropdown stays — it's meaningful even in compact mode
- Mod count summary shown as text (no scrollable list)
- Only 3 buttons: Start/Stop Monitor, Create Backup, View Snapshots
- All sizes go through Design — scale 1.5× still fits comfortably
- Window `min_inner_size` set to `(280 * scale, 220 * scale)` so scaling works

Implementation: `render_compact_central()` new function in `app.rs` or a new
`src/ui/compact.rs` file. Called instead of normal central panel when
`app.compact_mode` is true.

---

## Section 3: Mod Row Visual Refresh

### Current state
Flat rows with very faint tinting. Workshop badge is `[W]` text. Toggle is
a small painted rect. Row numbers are small gray text. No clear visual difference
between enabled and disabled beyond text color.

### New treatment

**Row backgrounds:**
- Enabled rows: subtle blue tint (2 shades alternating) — visually "on"
- Disabled rows: near-neutral (2 shades) — visually "off"
- Drag source: `extreme_bg_color` + 2px selection border (unchanged)

**Row number:** right-aligned in a fixed-width column (Design: `font_small`,
`row_number` color). Always aligned regardless of mod count digits.

**Mod name:** `font_body` size. Enabled = normal text color, disabled = weak
color + slight italic.

**Workshop badge:** Instead of `[W]` text, a small rounded pill: `W` with
`badge_workshop` background fill and white text. More visually distinct.

**Missing badge:** Same pill shape in `badge_missing` red.

**Toggle switch:** Slightly larger (scaled), pill shape unchanged but knob
gets a subtle shadow/inset effect via a second circle 1px offset in a darker
color. Cleaner on/off distinction.

**Hover:** 1.5px border (up from 1px) in accent color, slightly more visible.

**Enabled count footer:** stays as-is, already well-placed.

---

## Section 4: Centering & General QoL

### Issues fixed

1. **Empty mod list message** — already uses `centered_and_justified`, looks ok.
   Make text `font_heading` size so it reads as a proper empty state.

2. **Monitor Active screen** (`render_monitor_active`) — vertically centered
   with `add_space(40)` hardcoded. Instead use `ui.vertical_centered` with
   available height to truly center it. Font sizes go through Design.

3. **Modal widths** — `BackupManager` is 500px hardcoded, `SnapshotManager`
   450px. Make these `400 * scale` and `360 * scale` respectively.

4. **Scrollable modal heights** — 300/400px hardcoded. Use `300 * scale` etc.

5. **Settings form** — the `desired_width(ui.available_width() - 180.0)`
   pattern breaks at small widths. Change to subtract a scaled constant:
   `available_width() - (180.0 * scale)`.

6. **Sidebar buttons** — currently default width (egui fits to text). Set
   `ui.set_width(d.sidebar_w - d.md * 2.0)` so buttons fill the sidebar
   consistently.

7. **Gallery preset cards** — use `ui.available_width()` for the card frame
   so they expand to fill the panel.

---

## Section 5: Settings Additions

In Settings → Appearance, add:

```
UI Scale:   [0.75 ──●────── 2.00]   (slider, default 1.0)
              Small          Large
```

The value is stored as `ui_scale: f32` in `AppSettings` with `#[serde(default)]`
defaulting to `1.0`. Changes apply immediately (live preview) rather than
requiring Save & Close, since it's purely visual.

---

## Files Changed

| File | Change |
|------|--------|
| `src/ui/design.rs` | **New** — Design struct |
| `src/ui/theme.rs` | Expand palette, integrate Design colors |
| `src/ui/mod_list.rs` | Use Design for all sizes/colors, badge pills, toggle |
| `src/ui/header.rs` | Use Design font/spacing |
| `src/ui/sidebar.rs` | Use Design width/spacing, full-width buttons |
| `src/ui/preset_bar.rs` | Use Design |
| `src/ui/gallery.rs` | Expand card width, use Design |
| `src/ui/modals.rs` | Scale modal/scroll heights |
| `src/ui/settings.rs` | Add ui_scale slider |
| `src/ui/compact.rs` | **New** — compact mode central panel |
| `src/app.rs` | Instantiate Design, pass to render fns, call compact view |
| `src/models.rs` | Add `ui_scale: f32` to AppSettings |

## Non-Goals

- No new dependencies
- No restructuring of async/task system
- No changes to data model or file formats beyond the new `ui_scale` field
- No changes to compact mode trigger logic (existing toggle stays)
