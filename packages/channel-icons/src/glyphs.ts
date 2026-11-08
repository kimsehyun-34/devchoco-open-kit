/**
 * The glyph set: one array that holds each icon's key, English label and SVG paths.
 *
 * Every glyph is drawn on a 24x24 grid as open strokes (2px, round caps and joins), so the
 * same paths work at any size and take their color from `currentColor`. Keys are stable
 * identifiers meant to be stored in your own data; labels are only the English defaults
 * (see `labels.ts` for translating them).
 */
export const CHANNEL_ICONS = [
  {
    key: "general",
    label: "General",
    paths: ["M5 9h14M5 15h14M10 4 8 20M16 4l-2 16"],
  },
  {
    key: "notice",
    label: "Notice",
    paths: ["M4 9.5v5h3l7 4v-13l-7 4H4z", "M17.5 9a4 4 0 0 1 0 6"],
  },
  {
    key: "ops",
    label: "Operations",
    paths: [
      "M4 7h9M17 7h3M4 17h3M11 17h9",
      "M15 5a2 2 0 1 0 0 4 2 2 0 0 0 0-4z",
      "M9 15a2 2 0 1 0 0 4 2 2 0 0 0 0-4z",
    ],
  },
  {
    key: "dev",
    label: "Development",
    paths: ["M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16"],
  },
  {
    key: "talk",
    label: "Conversation",
    paths: ["M4 5h16v11H10l-4 4v-4H4z"],
  },
  {
    key: "design",
    label: "Design",
    paths: [
      "M12 3a9 9 0 1 0 0 18c1.2 0 2-.8 2-1.8 0-.5-.2-.9-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1 .8-1.6 1.8-1.6H17a4 4 0 0 0 4-4c0-4.4-4-8-9-8z",
      "M7.5 11h.01M10 7.5h.01M15 8h.01",
    ],
  },
  {
    key: "marketing",
    label: "Marketing",
    paths: ["M3 17l6-6 4 4 8-8M15 7h6v6"],
  },
  {
    key: "support",
    label: "Support",
    paths: [
      "M4 13v-1a8 8 0 0 1 16 0v1",
      "M4 13h3v5H5.5A1.5 1.5 0 0 1 4 16.5z",
      "M20 13h-3v5h1.5a1.5 1.5 0 0 0 1.5-1.5z",
      "M18.5 18v1.5a2.5 2.5 0 0 1-2.5 2.5h-3",
    ],
  },
  {
    key: "planning",
    label: "Planning",
    paths: [
      "M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2.1h5c0-.9.4-1.6 1-2.1A6 6 0 0 0 12 3z",
      "M10 19h4M11 22h2",
    ],
  },
  {
    key: "schedule",
    label: "Schedule",
    paths: [
      "M5 5h14a1 1 0 0 1 1 1v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z",
      "M4 10h16M8 3v4M16 3v4",
    ],
  },
  {
    key: "docs",
    label: "Documents",
    paths: [
      "M7 3h7l5 5v12a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z",
      "M14 3v5h5M9 13h6M9 17h6",
    ],
  },
  {
    key: "security",
    label: "Security",
    paths: ["M12 3l8 3v6c0 4.5-3.2 7.8-8 9-4.8-1.2-8-4.5-8-9V6z", "M9 12l2 2 4-4"],
  },
  {
    key: "people",
    label: "People",
    paths: [
      "M9 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z",
      "M2.5 20v-1a5 5 0 0 1 5-5h3a5 5 0 0 1 5 5v1",
      "M16 4.3a3.5 3.5 0 0 1 0 6.4M18.5 14.4A5 5 0 0 1 21.5 19v1",
    ],
  },
  {
    key: "finance",
    label: "Finance",
    paths: [
      "M4 6h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1z",
      "M12 9.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z",
      "M6.5 12h.01M17.5 12h.01",
    ],
  },
  {
    key: "issue",
    label: "Issues",
    paths: [
      "M8 14v-3a4 4 0 0 1 8 0v3a4 4 0 0 1-8 0z",
      "M10 7.2 8.5 4M14 7.2 15.5 4M12 9v10",
      "M3.5 8.5 8.2 10M20.5 8.5 15.8 10M3 14h5M16 14h5M3.5 19.5 8 17.5M20.5 19.5 16 17.5",
    ],
  },
  {
    key: "release",
    label: "Release",
    paths: [
      "M12 3c3 2 5 5.5 5 9.5l-2 2.5H9l-2-2.5C7 8.5 9 5 12 3z",
      "M12 10.5h.01",
      "M7 12.5l-2.5 3.5 3-.5M17 12.5l2.5 3.5-3-.5",
      "M10.5 18c0 1.5.7 2.5 1.5 3 .8-.5 1.5-1.5 1.5-3",
    ],
  },
  {
    key: "question",
    label: "Questions",
    paths: [
      "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z",
      "M9.5 9.5a2.5 2.5 0 1 1 3.6 2.2c-.7.4-1.1.9-1.1 1.8M12 17h.01",
    ],
  },
  {
    key: "social",
    label: "Social",
    paths: [
      "M5 9h11v6a4 4 0 0 1-4 4H9a4 4 0 0 1-4-4z",
      "M16 10h1.5a2.5 2.5 0 0 1 0 5H16",
      "M8 3v3M12 3v3",
    ],
  },
  {
    key: "celebrate",
    label: "Celebration",
    paths: [
      "M4 20 9 8l7 7z",
      "M13.5 5.5v-2M19 10h2M17.5 6l1.5-1.5M12.5 10l2-2",
    ],
  },
  {
    key: "alert",
    label: "Alerts",
    paths: ["M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z", "M12 3v2M10 21h4"],
  },
] as const;

/** Union of the 20 glyph keys. */
export type ChannelIconKey = (typeof CHANNEL_ICONS)[number]["key"];

/** One entry of {@link CHANNEL_ICONS}. */
export interface ChannelIconDefinition {
  readonly key: ChannelIconKey;
  /** English default label; translations live in the label maps. */
  readonly label: string;
  /** SVG path data on a 24x24 grid, stroked with `currentColor`. */
  readonly paths: readonly string[];
}

/** Keys in display order. */
export const CHANNEL_ICON_KEYS: readonly ChannelIconKey[] = CHANNEL_ICONS.map(
  (icon) => icon.key,
);

/** The glyph used when a key is empty or unknown. */
export const DEFAULT_ICON_KEY: ChannelIconKey = "general";

const byKey = new Map<string, ChannelIconDefinition>(
  CHANNEL_ICONS.map((icon) => [icon.key, icon]),
);

/** Type guard for the 20 known keys. */
export function isChannelIconKey(value: unknown): value is ChannelIconKey {
  return typeof value === "string" && byKey.has(value);
}

/** Looks a glyph up by key; empty, missing or unknown keys resolve to `general`. */
export function getChannelIcon(
  key: string | null | undefined,
): ChannelIconDefinition {
  return (key ? byKey.get(key) : undefined) ?? byKey.get(DEFAULT_ICON_KEY)!;
}
