# channel-icons

An accessible icon tile and icon/color picker for chat channels, rooms, projects, or anything else
that needs a small colored badge. React 18+, no runtime dependencies, about 20 inline SVG glyphs.

- `<ChannelIcon />` draws a rounded tile in any color you give it and picks a readable glyph color
  on its own (WCAG contrast, always at least 4.4:1).
- `<ChannelIconPicker />` is two native radio groups (icon, color) with a built-in "auto" color
  choice that means "use the project color". Keyboard, focus and screen reader behavior come from
  the browser, not from custom key handlers.
- Every string can be replaced: an English default and a Korean map are included.

## Install and use

The package is ESM only and has one peer dependency, `react >= 18`.

```sh
pnpm add channel-icons
```

Import the stylesheet once, anywhere in your app:

```tsx
import "channel-icons/styles.css";
```

### The tile

```tsx
import { ChannelIcon } from "channel-icons";

// Decorative: the channel name next to it already carries the meaning.
<>
  <ChannelIcon icon="dev" color="#2563eb" />
  <span>engineering</span>
</>

// Standalone: give it a name and it becomes role="img".
<ChannelIcon icon="alert" color="#dc2626" size={40} label="Alerts channel" />
```

| Prop | Type | Default | Notes |
| --- | --- | --- | --- |
| `icon` | `string \| null` | `"general"` | One of the 20 keys below. Empty, missing or unknown keys render `general`. |
| `color` | `string \| null` | stylesheet default | `#rgb` or `#rrggbb`. Written to `--channel-icon-bg`; the glyph color is derived and written to `--channel-icon-fg`. Anything that is not hex (names, `var(...)`) is ignored. |
| `size` | `number` | `24` | Tile edge in px. The glyph is 58% of it, never below 12px. |
| `label` | `string` | none | Accessible name. Omitted or blank means `aria-hidden`. |
| `className`, `style`, other span attributes | | | Passed to the root `<span>`. |

Glyph keys, in display order: `general`, `notice`, `ops`, `dev`, `talk`, `design`, `marketing`,
`support`, `planning`, `schedule`, `docs`, `security`, `people`, `finance`, `issue`, `release`,
`question`, `social`, `celebrate`, `alert`. They are meant to be stored in your own data. The array
`CHANNEL_ICONS` (key, English label, SVG paths) is the single source of truth for all of them.

### The picker

```tsx
import { useState } from "react";
import { ChannelIconPicker, type ChannelIconValue } from "channel-icons";

function Appearance({ projectColor }: { projectColor: string }) {
  const [value, setValue] = useState<ChannelIconValue>({ icon: "general", color: null });
  return (
    <ChannelIconPicker
      value={value}
      onChange={setValue}
      autoColor={projectColor} // what "Auto" previews as
    />
  );
}
```

`color: null` means auto. Store `null` and resolve it to the project color wherever you draw the tile:

```tsx
<ChannelIcon icon={value.icon} color={value.color ?? projectColor} />
```

| Prop | Type | Default | Notes |
| --- | --- | --- | --- |
| `value` | `{ icon: string; color: string \| null }` | required | Controlled. Unknown icon keys show `general` as selected. |
| `onChange` | `(next: ChannelIconValue) => void` | required | Called with the complete next value, only when the selection actually changes. |
| `colors` | `{ value: string; label?: string }[]` | `DEFAULT_COLORS` | Swatches offered next to "Auto". Invalid and duplicate entries are skipped. |
| `autoColor` | `string \| null` | none | Color the "Auto" dot and the icon previews use while auto is selected. |
| `labels` | `ChannelIconLabelOverrides` | English | Any subset of the label set, see below. |
| `disabled` | `boolean` | `false` | Disables both groups. |
| `className` | `string` | none | Added to the root element. |

If `value.color` is a valid hex that is not in `colors`, it is appended as an extra "Current color"
swatch so it stays selectable and is not lost on the next pick.

### Translations

```tsx
import { ChannelIconPicker, koLabels } from "channel-icons";

<ChannelIconPicker value={value} onChange={setValue} labels={koLabels} />

// Or override only what you need; everything else stays English.
<ChannelIconPicker
  value={value}
  onChange={setValue}
  labels={{ iconGroup: "Glyph", icons: { dev: "Engineering" }, colorNames: { "#2563eb": "Ocean" } }}
/>
```

`ChannelIconLabels` has these fields: `icons` (name per key), `colorNames` (name per lowercase hex),
`iconGroup` and `colorGroup` (the legends), `autoColor` (visible text), `autoColorHint` (extra
accessible text such as "Use the project color"), and `currentColor`. `enLabels` and `koLabels` are
complete maps; `resolveLabels(overrides, base)` merges a partial map onto either.

For your own palette, put the names in `colors` (`{ value: "#112233", label: "Midnight" }`) or in
`labels.colorNames`. A color with neither is announced by its hex value.

## What it looks like

No screenshots are shipped; this is the layout in words.

**Tile.** A square with corners rounded to 30% of its edge, filled with the chosen color, with one
white or near-black line glyph centered in it. Glyphs are 2px round-capped strokes with no fills, so
they stay crisp from 16px to 64px. Pale tiles (yellow, orange, white) get the dark glyph; saturated
or dark tiles get the light one.

**Picker.** Two stacked sections, each with a small gray legend ("Icon", "Background color").

- The icon section is a grid of 44px cells showing all 20 glyphs, each drawn on a tile in the color
  that is currently selected, so the grid doubles as a preview. The selected cell has a 2px accent
  border. The grid fits as many cells per row as the width allows and shares leftover space. The
  count follows the width of the grid, not of the screen: measured in the demo page, a 320px
  screen (a grid about 270px wide) gets five per row in four rows, and a 360px screen (about 310px)
  gets six per row.
- The color section is one wrapping row. It starts with a pill reading "Auto" next to a dot in the
  `autoColor`, then eight 28px dots in the default palette. The selected dot gets a gap-and-ring
  halo; the selected pill gets the accent border.
- Keyboard focus draws a 2px outline, offset by 2px, around the focused choice, on top of the
  selection border.
- In forced-colors mode (Windows High Contrast) choices get system-color borders, the selected one
  gets a thicker double `Highlight` border, focus is a separate thick `CanvasText` outline, and
  tiles and dots keep their own colors because those colors are the thing being chosen.

## Accessibility

- Each group is a `<fieldset role="radiogroup">` named by its `<legend>`; every choice is a real
  `<input type="radio">` inside a `<label>`. The radios are visually hidden but stretched over the
  whole choice, so the click and touch target is the full 44px cell.
- Native behavior, with no JavaScript key handling: one Tab stop per group (the selected radio),
  arrow keys move and select with wrap-around, Space selects.
- Names come from visually hidden text, not from `title`; `title` is only an extra tooltip. Glyphs
  and color dots are `aria-hidden`. "Auto" is announced as "Auto (Use the project color)".
- Selection is shown by border and ring shape, not by color alone. The focus ring uses a separate
  token and has at least 3:1 contrast against the default surfaces.
- The `fieldset` is given `min-inline-size: 0`. Without it, fieldsets size to their content and
  cause horizontal scrolling on narrow screens.
- `ChannelIcon` with a `label` is `role="img"`; without one it is hidden. Glyph contrast is
  computed from the actual color, so a custom palette cannot produce an unreadable glyph. The
  exported `contrastRatio`, `relativeLuminance`, `readableGlyphColor` and `normalizeHex` use the
  same math if you need it elsewhere.

## Styling

Everything is plain CSS in `channel-icons/styles.css`. These custom properties are public:

| Property | Where | Meaning |
| --- | --- | --- |
| `--channel-icon-size` | tile | Tile edge. Set by the `size` prop. Default `24px`. |
| `--channel-icon-bg` | tile | Tile background. Set by the `color` prop. Default `#475569`. |
| `--channel-icon-fg` | tile | Glyph color. Set automatically from the background. Default `#fff`. If you set `--channel-icon-bg` in your own CSS, set this one too. |
| `--channel-icon-radius` | tile | Corner radius as a fraction of the tile edge. Default `0.3`. |
| `--channel-picker-text` | picker | Text color. |
| `--channel-picker-muted` | picker | Legend color. |
| `--channel-picker-border` | picker | Ring around color dots. Should have 3:1 against the surface. |
| `--channel-picker-accent` | picker | Border of the selected choice. |
| `--channel-picker-focus` | picker | Keyboard focus ring. |
| `--channel-picker-surface` | picker | Page color behind the picker; fills the gap in the selection halo. |
| `--channel-picker-choice` | picker | Minimum size of a choice. Default `44px`. |
| `--channel-picker-gap` | picker | Space between choices. Default `6px`. |
| `--channel-picker-swatch` | color dot | Set by the component on each dot. Do not set it yourself. |

Set the picker tokens on the picker, a wrapper, or `:root`. The defaults use `light-dark()`, so
they follow the page's `color-scheme`, not the OS setting: add `color-scheme: light dark` to `:root`
if your app follows the OS, or `color-scheme: dark` to your dark theme. A page that never sets
`color-scheme` gets the light values, which is also what browsers without `light-dark()` get. If
your theme is not expressed through `color-scheme`, set the six color tokens yourself.

## Development

```sh
pnpm install
pnpm --filter channel-icons run typecheck
pnpm --filter channel-icons run test
pnpm --filter channel-icons run build   # dist/index.js, dist/*.d.ts, dist/styles.css
```

The tests run in jsdom with Testing Library. jsdom does not implement the browser's arrow-key
movement between radios, so the keyboard tests use `user-event`, which emulates it. A real browser
pass over the same behavior, and over layout at 320px, is worth repeating after any CSS change.

`src/a11y.test.tsx` runs [axe-core](https://github.com/dequelabs/axe-core) (WCAG 2.0 to 2.2 A/AA
and best practices) over the tile and over the picker in its main states: default, Korean labels,
disabled, a colour outside the palette, an unknown icon key and a partly named custom palette.
jsdom has no layout, so axe cannot measure colour contrast or target size there and the
`color-contrast` rule is switched off in those tests; contrast is covered by the maths tests
instead. To check both in a real browser, build the demo in `examples/channel-icons-demo`, open it
with axe-core injected, and repeat in light, dark, 320px and forced-colors modes. The last run of
that kind found no violations in any of the four.

## Limits

- Colors must be hex to get an automatic glyph color. For CSS-only theming, set `--channel-icon-bg`
  and `--channel-icon-fg` yourself.
- There are exactly 20 glyphs and the picker shows all of them. To offer fewer, filter on your side
  and render your own radios, or wrap the tile.
- `ChannelIconPicker` is a client component (`"use client"`). `ChannelIcon` has no hooks and
  renders on the server.

## License

MIT
