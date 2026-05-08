# Design System

Color palette and theming architecture. Noita-inspired purple accent with dark/light mode support.

## Architecture

Three concerns, two files:

| Concern | Where | When applied |
|---|---|---|
| Global zoom (UI scaling) | `design::apply_zoom()` | Every frame in `update()` + startup |
| egui visuals (widget fills, selection, text) | `theme::apply_theme()` | Startup + dark mode toggle |
| App-specific colors & layout constants | `Design::new()` | Per-frame, wherever `d.` tokens are used |

`Design` values are **base logical pixels** — `ctx.set_zoom_factor()` handles scaling globally. Do not multiply Design values by scale.

## Color Palette

- **Purple is the accent color.** Row stripes, selection, badges, drag indicators.
- **Semantic colors stay fixed:** red for missing/danger, green for toggle-on/status-ok. Mode-independent.
- **Dark mode uses `override_text_color`** in theme.rs to brighten base text. Light mode does not.
- **Light mode needs lower alpha.** Same RGB values that look good in dark mode become stained/muddy in light mode. Light mode row colors use roughly half the alpha of dark mode equivalents.

## Premultiplied Alpha

`Color32::from_rgba_premultiplied(r, g, b, a)` — RGB values are already multiplied by alpha.
- `(80, 50, 150, 65)` is NOT `rgb(80,50,150)` at `65/255` opacity
- In practice: keep RGB values lower than or near the alpha for subtle tints
- When tuning: adjust alpha first (opacity), then RGB (hue/saturation)

## Adding a New Design Token

1. Add the field to the `Design` struct
2. Add the dark/light values in `Design::new()` using the `if dark { ... } else { ... }` pattern
3. Use it in render code via `d.field_name`

No registration, no maps — it's a plain struct.

## Light vs Dark Tuning

| Symptom | Likely fix |
|---|---|
| Looks stained/muddy in light mode | Reduce alpha, shift RGB toward cooler tones |
| Text unreadable on colored background | Use `fg_stroke` override in theme.rs, or a Design token for explicit text color |
| Selection highlight drowns out text | Use semi-transparent `selection.bg_fill` in light mode |
| Row stripes invisible | Increase alpha difference between even/odd rows |
| Widget hover invisible in light mode | Set `widgets.hovered.bg_fill` in theme.rs |
