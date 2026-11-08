"use client";

import { useId, type CSSProperties, type ReactElement } from "react";
import { normalizeHex } from "./color.js";
import { ChannelIcon } from "./ChannelIcon.js";
import { CHANNEL_ICONS, getChannelIcon } from "./glyphs.js";
import {
  resolveLabels,
  type ChannelIconLabelOverrides,
} from "./labels.js";
import { DEFAULT_COLORS, type ChannelColorOption } from "./palette.js";

/** The picked appearance. */
export interface ChannelIconValue {
  /** Glyph key; empty or unknown keys are shown (and announced) as `general`. */
  icon: string;
  /** Tile color as hex, or `null` for "auto": use the project color. */
  color: string | null;
}

export interface ChannelIconPickerProps {
  value: ChannelIconValue;
  /** Called with the complete next value whenever the user picks an icon or a color. */
  onChange: (next: ChannelIconValue) => void;
  /** Palette to offer next to "auto". Defaults to {@link DEFAULT_COLORS}. */
  colors?: readonly ChannelColorOption[];
  /**
   * The color "auto" resolves to (typically your project color). It is only used for
   * previews; the picker itself reports `color: null` for auto.
   */
  autoColor?: string | null;
  /** Translations; any subset of {@link ChannelIconLabels} (see `koLabels` for a full map). */
  labels?: ChannelIconLabelOverrides;
  disabled?: boolean;
  className?: string;
}

interface Swatch {
  hex: string;
  label: string;
}

const swatchStyle = (hex: string) =>
  ({ "--channel-picker-swatch": hex }) as CSSProperties;

/**
 * Icon and color chooser built from two native radio groups. Each group is a `fieldset`
 * with `role="radiogroup"` named by its `legend`; the browser supplies Tab (one stop per
 * group), arrow-key movement with wrap-around, and selection on focus.
 */
export function ChannelIconPicker({
  value,
  onChange,
  colors = DEFAULT_COLORS,
  autoColor,
  labels,
  disabled = false,
  className,
}: ChannelIconPickerProps): ReactElement {
  const id = useId();
  const text = resolveLabels(labels);
  const selectedIcon = getChannelIcon(value.icon).key;
  const selectedColor = normalizeHex(value.color);
  const auto = normalizeHex(autoColor);

  const palette = new Map<string, Swatch>();
  for (const option of colors) {
    const hex = normalizeHex(option.value);
    if (hex && !palette.has(hex)) {
      palette.set(hex, {
        hex,
        label: option.label?.trim() || text.colorNames[hex] || hex,
      });
    }
  }
  // A color chosen elsewhere stays selectable instead of silently changing on the next pick.
  if (selectedColor && !palette.has(selectedColor)) {
    palette.set(selectedColor, { hex: selectedColor, label: text.currentColor });
  }

  // Icon choices preview the color the user would get right now.
  const previewColor = selectedColor ?? auto;

  return (
    <div className={className ? `channel-picker ${className}` : "channel-picker"}>
      <fieldset
        className="channel-picker-group"
        role="radiogroup"
        disabled={disabled}
      >
        <legend className="channel-picker-legend">{text.iconGroup}</legend>
        <div className="channel-picker-icons">
          {CHANNEL_ICONS.map((item) => {
            const label = text.icons[item.key];
            return (
              <label className="channel-picker-choice" key={item.key} title={label}>
                <input
                  className="channel-picker-input"
                  type="radio"
                  name={`${id}-icon`}
                  value={item.key}
                  checked={selectedIcon === item.key}
                  onChange={() =>
                    onChange({ icon: item.key, color: value.color ?? null })
                  }
                />
                <span className="channel-picker-face">
                  <ChannelIcon icon={item.key} color={previewColor} size={32} />
                </span>
                <span className="channel-picker-sr">{label}</span>
              </label>
            );
          })}
        </div>
      </fieldset>

      <fieldset
        className="channel-picker-group"
        role="radiogroup"
        disabled={disabled}
      >
        <legend className="channel-picker-legend">{text.colorGroup}</legend>
        <div className="channel-picker-colors">
          <label className="channel-picker-choice" title={text.autoColorHint}>
            <input
              className="channel-picker-input"
              type="radio"
              name={`${id}-color`}
              value=""
              checked={selectedColor === null}
              onChange={() => onChange({ icon: value.icon, color: null })}
            />
            <span className="channel-picker-face channel-picker-face-auto">
              <span
                className="channel-picker-dot"
                style={auto ? swatchStyle(auto) : undefined}
                aria-hidden="true"
              />
              <span className="channel-picker-auto-text">{text.autoColor}</span>
            </span>
            <span className="channel-picker-sr"> ({text.autoColorHint})</span>
          </label>
          {[...palette.values()].map((swatch) => (
            <label
              className="channel-picker-choice"
              key={swatch.hex}
              title={swatch.label}
            >
              <input
                className="channel-picker-input"
                type="radio"
                name={`${id}-color`}
                value={swatch.hex}
                checked={selectedColor === swatch.hex}
                onChange={() => onChange({ icon: value.icon, color: swatch.hex })}
              />
              <span className="channel-picker-face">
                <span
                  className="channel-picker-dot"
                  style={swatchStyle(swatch.hex)}
                  aria-hidden="true"
                />
              </span>
              <span className="channel-picker-sr">{swatch.label}</span>
            </label>
          ))}
        </div>
      </fieldset>
    </div>
  );
}
