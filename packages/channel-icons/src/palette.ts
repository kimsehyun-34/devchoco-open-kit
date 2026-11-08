/** One selectable tile color in the picker. */
export interface ChannelColorOption {
  /** `#rgb` or `#rrggbb`. Entries that are not valid hex colors are skipped. */
  value: string;
  /**
   * Accessible name. When omitted, the picker looks the color up in `labels.colorNames`
   * and finally falls back to the hex value.
   */
  label?: string;
}

/** The default palette: eight mid-saturation colors, a mix of dark and light tiles. */
export const DEFAULT_COLORS: readonly ChannelColorOption[] = [
  { value: "#475569" },
  { value: "#dc2626" },
  { value: "#f97316" },
  { value: "#facc15" },
  { value: "#16a34a" },
  { value: "#0d9488" },
  { value: "#2563eb" },
  { value: "#7c3aed" },
];
