# Design System

Color palette and theming architecture. Noita-inspired purple accent with dark/light mode support.

## Architecture

Two files control all visual styling:

- **`src/ui/theme.rs`** — Global egui visuals (widget fills, selection highlight, text overrides). Applied once on startup and when toggling dark mode.
- **`src/ui/design.rs`** — `Design` struct with all app-specific colors, spacing, and font sizes. Instantiated per-frame via `Design::new(ctx, settings)`.

Theme sets the egui baseline; Design adds the app layer on top.

## Color Palette Philosophy

- **Purple is the accent color.** Row stripes, selection, badges, drag indicators — all purple-tinted.
- **Semantic colors stay fixed:** red for missing/danger, green for toggle-on/status-ok. These don't change with dark/light mode.
- **Dark mode uses `override_text_color`** in theme.rs to brighten base text from egui's default. Light mode does not — egui's default dark text is fine.
- **Light mode needs lower alpha.** The same RGB values that look good in dark mode become stained/muddy in light mode. Light mode row colors use roughly half the alpha of dark mode equivalents.

## Premultiplied Alpha Gotcha

`Color32::from_rgba_premultiplied(r, g, b, a)` — the RGB values are already multiplied by alpha. This means:
- `(80, 50, 150, 65)` is NOT 80/255 red at 65/255 opacity
- The actual unmultiplied color is approximately `(314, 196, 589, 65)` which gets clamped
- In practice: keep RGB values lower than or near the alpha value for subtle tints, or use higher alpha for more vivid colors
- When tuning: adjust alpha first (opacity), then RGB (hue/saturation)

## Adding a New Design Token

1. Add the field to the `Design` struct
2. Add the dark/light values in `Design::new()` using the `if dark { ... } else { ... }` pattern
3. Use it in render code via `d.field_name`

No registration, no maps — it's a plain struct.

## Light vs Dark Tuning Guide

When a color looks off in one mode but not the other:

| Symptom | Likely fix |
|---|---|
| Looks stained/muddy in light mode | Reduce alpha, shift RGB toward cooler tones |
| Text unreadable on colored background | Use `fg_stroke` override in theme.rs, or use a Design token for explicit text color |
| Selection highlight drowns out text | Use semi-transparent `selection.bg_fill` in light mode |
| Row stripes invisible | Increase alpha difference between even/odd rows |

## mod_list_bg

The mod list scroll area is wrapped in a `Frame` with `d.mod_list_bg` fill — a very subtle tinted background that visually separates the list from the rest of the UI. Dark mode uses a dark purple wash; light mode uses a barely-visible purple tint. See `mod_list.rs` for the Frame wrapping pattern.
