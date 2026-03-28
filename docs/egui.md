# egui / eframe Reference

Version-specific notes for the egui stack used in Hallinta. These are things that caused bugs or required non-obvious API usage.

**Versions:** eframe 0.33.3, egui 0.33.3, wgpu renderer (DirectX 12 on Windows).

---

## Theme / Dark Mode

egui 0.33 manages themes through `ThemePreference` (Dark | Light | System). The default preference is `System`, which reads the OS theme via winit each frame in `Options::begin_pass()`.

**Pitfall:** Calling `ctx.set_visuals(dark_visuals())` alone does NOT persist across frames — the next `begin_pass()` resolves the theme preference back to the OS theme and picks the corresponding style, overriding your visuals. This means dark mode appears to work in-session (because `set_visuals` applies immediately) but fails on startup if the OS is in light mode.

**Correct approach:**

```rust
// 1. Set the preference so egui stops following the OS
ctx.set_theme(egui::ThemePreference::Dark);

// 2. Customize what "dark" looks like (optional — skip if defaults are fine)
ctx.set_visuals_of(egui::Theme::Dark, custom_dark_visuals());
```

Both calls are needed: `set_theme` controls which theme is active, `set_visuals_of` controls what that theme looks like. See `src/ui/theme.rs`.

---

## UI Scaling via `set_zoom_factor`

**Do NOT scale individual widgets manually.** Use `ctx.set_zoom_factor(scale)` which scales the entire UI uniformly — text, widgets, spacing, margins, layout, everything. Called once per frame in `update()` and on startup via `design::apply_zoom()`.

`Design` struct values are fixed base logical pixels (not multiplied by scale). egui's zoom handles the rest.

**Slider feedback loop pitfall:** Changing zoom while a slider is being dragged causes a feedback loop — the slider track shifts in logical coordinates, egui thinks the mouse moved, the value oscillates. Fix: only apply the zoom on `drag_stopped()` or `changed() && !dragged()`, never during an active drag. DragValue (number drag) doesn't have this problem because it has no spatial track.

```rust
// WRONG — spasms during drag
if scale_resp.changed() {
    app.settings.ui_scale = settings.ui_scale;
}

// CORRECT — applies on release or non-drag changes
if scale_resp.drag_stopped() || (scale_resp.changed() && !scale_resp.dragged()) {
    app.settings.ui_scale = settings.ui_scale;
}
```

This feedback loop applies to ANY slider that controls zoom, font size, or layout-affecting values. The general rule: if a slider's value changes the coordinate system the slider lives in, defer the effect to `drag_stopped()`.

---

## Stroke API (0.33 breaking change)

`rect_stroke` requires a 4th parameter in 0.33:

```rust
painter.rect_stroke(rect, rounding, stroke, egui::StrokeKind::Outside);
```

Earlier versions used 3 parameters. Forgetting `StrokeKind` is a compile error but the fix isn't obvious from the message.

---

## Margin::symmetric

Takes `i8` values, not `f32`:

```rust
egui::Margin::symmetric(8i8, 5i8)
```

---

## Drag-and-Drop Patterns

### Whole-row drag detection

Render the frame first, then overlay an interaction rect:

```rust
let frame_resp = egui::Frame::NONE.fill(bg).show(ui, |ui| { /* row content */ }).response;
let row_resp = ui.interact(frame_resp.rect, id, egui::Sense::click_and_drag());
```

This gives drag detection AND right-click (`secondary_clicked()`) for context menus without needing a separate drag handle widget.

### Live-preview reorder

Rather than showing a drop-indicator line, the list can be reordered in real-time as the pointer moves:

1. Store `current_index` (where the item is now) and `pre_drag_snapshot` (original order for Escape cancel).
2. Each frame, detect which row the pointer is over. If it differs from `current_index`, remove and reinsert the item.
3. Row numbers update automatically since they're derived from the visual position.
4. On release: save. On Escape: restore snapshot.

See `src/ui/mod_list.rs` and `DragState` in `src/models.rs`.

---

## NativeOptions (eframe 0.33)

The `follow_system_theme` field was **removed** in eframe 0.33 (it existed in 0.30). Theme following is now always on at the egui level via `ThemePreference::System`. Override it with `ctx.set_theme()` as described above.

The wgpu renderer is selected via:

```rust
eframe::NativeOptions {
    renderer: eframe::Renderer::Wgpu,
    ..Default::default()
}
```

Using `Wgpu` avoids the flickering issues seen with the `Glow` renderer on Windows with DirectX 12.

---

## Settings Live Preview with Cancel Revert

When a setting should preview live (e.g., UI scale slider) but also support Cancel:

1. Store the original value in a dedicated field **when entering settings** (e.g., `app.pre_settings_ui_scale`).
2. Write to `app.settings.<field>` on value change for immediate visual feedback.
3. On Save: the value is already applied — just persist.
4. On Cancel: restore from the saved original.

**Why a separate field?** `pending_settings` gets overwritten every frame (it's the working copy), so it can't preserve the "before editing" value. See `pre_settings_ui_scale` in `app.rs`, set in `header.rs`, restored in `settings.rs`.

---

## Button Text Centering: `add_sized` vs `add_enabled` + `min_size`

`ui.add_sized([width, 0.0], Button::new("text"))` centers the text within the allocated width.

`ui.add_enabled(cond, Button::new("text").min_size(vec2(width, 0.0)))` sets a minimum size but does **not** center the text — it left-aligns within the button rect.

**Fix for disabled buttons that need centering:** Wrap `add_sized` in `add_enabled_ui`:

```rust
ui.add_enabled_ui(!is_locked, |ui| {
    if ui.add_sized([btn_width, 0.0], egui::Button::new("Clear All")).clicked() {
        // ...
    }
});
```

---

## Light Mode Visuals: Selection vs Active

In light mode, egui's default `selection.bg_fill` is a solid color painted behind selected `selectable_label` text. A fully opaque accent color makes tab labels unreadable.

**Fix:** Use a semi-transparent selection fill, set `widgets.active.fg_stroke` for text color, and add `widgets.hovered.bg_fill` for visible hover:

```rust
visuals.selection.bg_fill = Color32::from_rgba_premultiplied(100, 65, 160, 90);
visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_rgb(70, 35, 130));
visuals.widgets.hovered.bg_fill = Color32::from_rgba_premultiplied(100, 65, 160, 30);
```

`fg_stroke` controls active/selected widget text color. Without it, egui picks a contrasting color against the fill, which can be white-on-purple (unreadable on light backgrounds).

---

## Wrapping a ScrollArea in a Background Frame

```rust
egui::Frame::NONE
    .fill(bg_color)
    .corner_radius(6.0)
    .inner_margin(Margin::symmetric(pad, pad))
    .show(ui, |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| { /* content */ });
    });
```

The Frame must be the **outer** wrapper. Putting it inside the ScrollArea would scroll the background away.
