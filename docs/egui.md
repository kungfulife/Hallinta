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

At max UI scale (3.0), verify that `(value * scale) as i8` doesn't overflow (max i8 = 127).

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
