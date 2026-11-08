import { describe, expect, it } from "vitest";
import {
  CHANNEL_ICONS,
  CHANNEL_ICON_KEYS,
  DEFAULT_ICON_KEY,
  getChannelIcon,
  isChannelIconKey,
} from "./glyphs.js";

describe("glyph set", () => {
  it("has the 20 documented keys in display order", () => {
    expect([...CHANNEL_ICON_KEYS]).toEqual([
      "general",
      "notice",
      "ops",
      "dev",
      "talk",
      "design",
      "marketing",
      "support",
      "planning",
      "schedule",
      "docs",
      "security",
      "people",
      "finance",
      "issue",
      "release",
      "question",
      "social",
      "celebrate",
      "alert",
    ]);
  });

  it("gives every glyph an English label and well-formed path data", () => {
    const labels = new Set<string>();
    const commands = /^[MmLlHhVvCcSsQqTtAaZz0-9eE.,\s+-]+$/;
    for (const icon of CHANNEL_ICONS) {
      expect(icon.label).toMatch(/^[A-Za-z][A-Za-z ]+$/);
      labels.add(icon.label);
      expect(icon.paths.length).toBeGreaterThan(0);
      for (const d of icon.paths) {
        expect(d).toMatch(/^[Mm]/);
        expect(d).toMatch(commands);
      }
    }
    expect(labels.size).toBe(CHANNEL_ICONS.length);
  });

  it("draws every glyph differently", () => {
    const drawings = new Set(CHANNEL_ICONS.map((icon) => icon.paths.join("|")));
    expect(drawings.size).toBe(CHANNEL_ICONS.length);
  });

  it("falls back to general for empty, missing and unknown keys", () => {
    expect(DEFAULT_ICON_KEY).toBe("general");
    for (const key of [undefined, null, "", "spaceship", "GENERAL"]) {
      expect(getChannelIcon(key).key).toBe("general");
    }
    expect(getChannelIcon("dev").key).toBe("dev");
    expect(isChannelIconKey("dev")).toBe(true);
    expect(isChannelIconKey("spaceship")).toBe(false);
    expect(isChannelIconKey(7)).toBe(false);
  });
});
