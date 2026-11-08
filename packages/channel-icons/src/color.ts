/** Glyph color on dark tiles. */
export const LIGHT_GLYPH = "#ffffff";
/** Glyph color on light tiles; near-black rather than pure black for a softer look. */
export const DARK_GLYPH = "#0b0b0f";

const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i;

/**
 * Normalizes `#rgb` / `#rrggbb` (any case) to lowercase `#rrggbb`.
 * Returns `null` for anything else, including CSS color names and `var(...)`.
 */
export function normalizeHex(value: unknown): string | null {
  if (typeof value !== "string") return null;
  const text = value.trim();
  if (!HEX.test(text)) return null;
  const digits = text.slice(1).toLowerCase();
  return digits.length === 3
    ? "#" + [...digits].map((digit) => digit + digit).join("")
    : "#" + digits;
}

function channel(hex: string, at: number): number {
  const value = parseInt(hex.slice(at, at + 2), 16) / 255;
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

/** WCAG 2 relative luminance (0 black, 1 white) of a hex color. Throws on invalid input. */
export function relativeLuminance(color: string): number {
  const hex = normalizeHex(color);
  if (!hex) throw new RangeError(`Not a hex color: ${String(color)}`);
  return (
    0.2126 * channel(hex, 1) + 0.7152 * channel(hex, 3) + 0.0722 * channel(hex, 5)
  );
}

/** WCAG 2 contrast ratio between two hex colors, from 1 (identical) to 21 (black on white). */
export function contrastRatio(a: string, b: string): number {
  const first = relativeLuminance(a);
  const second = relativeLuminance(b);
  const lighter = Math.max(first, second);
  const darker = Math.min(first, second);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Picks whichever of {@link LIGHT_GLYPH} and {@link DARK_GLYPH} has the higher contrast on
 * `background`. Ties go to the light glyph. For any background the result is at least 4.4:1.
 */
export function readableGlyphColor(background: string): string {
  return contrastRatio(LIGHT_GLYPH, background) >=
    contrastRatio(DARK_GLYPH, background)
    ? LIGHT_GLYPH
    : DARK_GLYPH;
}
