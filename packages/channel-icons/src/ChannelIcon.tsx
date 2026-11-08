import type { CSSProperties, HTMLAttributes, ReactElement } from "react";
import { normalizeHex, readableGlyphColor } from "./color.js";
import { getChannelIcon } from "./glyphs.js";

export interface ChannelIconProps
  extends Omit<HTMLAttributes<HTMLSpanElement>, "color"> {
  /** Glyph key. Empty, missing or unknown keys render the `general` glyph. */
  icon?: string | null;
  /**
   * Tile background as `#rgb` or `#rrggbb`. The glyph color is chosen from it automatically.
   * Anything else (names, `var(...)`, `null`) leaves the colors to the stylesheet defaults.
   */
  color?: string | null;
  /** Tile edge in px. Defaults to 24; the glyph is about 58% of it, never below 12px. */
  size?: number;
  /**
   * Accessible name. Without it the tile is decorative (`aria-hidden`), which is right when
   * the channel name is rendered next to it. With it the tile becomes `role="img"`.
   */
  label?: string;
}

/** A rounded, colored tile with one inline-SVG glyph. Needs `channel-icons/styles.css`. */
export function ChannelIcon({
  icon,
  color,
  size = 24,
  label,
  className,
  style,
  ...rest
}: ChannelIconProps): ReactElement {
  const glyph = getChannelIcon(icon);
  const background = normalizeHex(color);
  const edge = Number.isFinite(size) && size > 0 ? size : 24;
  const glyphSize = Math.max(12, Math.round(edge * 0.58));
  const name = label?.trim();

  const vars: Record<string, string> = { "--channel-icon-size": `${edge}px` };
  if (background) {
    vars["--channel-icon-bg"] = background;
    vars["--channel-icon-fg"] = readableGlyphColor(background);
  }

  return (
    <span
      {...rest}
      {...(name
        ? { role: "img", "aria-label": name }
        : { "aria-hidden": true as const })}
      className={className ? `channel-icon ${className}` : "channel-icon"}
      style={{ ...(vars as CSSProperties), ...style }}
      data-icon={glyph.key}
      data-color={background ?? undefined}
    >
      <svg
        viewBox="0 0 24 24"
        width={glyphSize}
        height={glyphSize}
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
        focusable="false"
        aria-hidden="true"
      >
        {glyph.paths.map((d, index) => (
          <path key={index} d={d} />
        ))}
      </svg>
    </span>
  );
}
