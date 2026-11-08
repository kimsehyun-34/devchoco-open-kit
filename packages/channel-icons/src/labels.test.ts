import { describe, expect, it } from "vitest";
import { CHANNEL_ICONS } from "./glyphs.js";
import { enLabels, koLabels, resolveLabels } from "./labels.js";
import { DEFAULT_COLORS } from "./palette.js";

describe("label maps", () => {
  it("builds the English icon labels from the glyph array", () => {
    for (const icon of CHANNEL_ICONS) expect(enLabels.icons[icon.key]).toBe(icon.label);
    expect(Object.keys(enLabels.icons)).toHaveLength(20);
  });

  it("ships a Korean map that covers every key and the same fields as English", () => {
    expect(Object.keys(koLabels.icons)).toEqual(Object.keys(enLabels.icons));
    expect(Object.keys(koLabels.colorNames)).toEqual(Object.keys(enLabels.colorNames));
    expect(Object.keys(koLabels).sort()).toEqual(Object.keys(enLabels).sort());
    for (const text of Object.values(koLabels.icons)) expect(text.trim()).not.toBe("");
    expect(koLabels.icons.support).not.toBe(enLabels.icons.support);
  });

  it("names every default palette color in both languages", () => {
    for (const { value } of DEFAULT_COLORS) {
      expect(enLabels.colorNames[value]).toBeTruthy();
      expect(koLabels.colorNames[value]).toBeTruthy();
    }
  });

  it("merges overrides onto the base without touching it", () => {
    const snapshot = JSON.stringify(enLabels);
    const merged = resolveLabels({ icons: { dev: "Engineering" }, autoColor: "Project" });
    expect(merged.icons.dev).toBe("Engineering");
    expect(merged.icons.docs).toBe("Documents");
    expect(merged.autoColor).toBe("Project");
    expect(merged.colorGroup).toBe("Background color");
    expect(JSON.stringify(enLabels)).toBe(snapshot);
  });

  it("can layer overrides on another language", () => {
    const merged = resolveLabels({ icons: { dev: "엔지니어링" } }, koLabels);
    expect(merged.icons.dev).toBe("엔지니어링");
    expect(merged.icons.docs).toBe(koLabels.icons.docs);
  });
});
