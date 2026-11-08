import { describe, expect, it } from "vitest";
import {
  DARK_GLYPH,
  LIGHT_GLYPH,
  contrastRatio,
  normalizeHex,
  readableGlyphColor,
  relativeLuminance,
} from "./color.js";
import { DEFAULT_COLORS } from "./palette.js";

describe("hex parsing", () => {
  it("normalizes short, long and mixed-case hex", () => {
    expect(normalizeHex("#ABC")).toBe("#aabbcc");
    expect(normalizeHex("#2F5FB3")).toBe("#2f5fb3");
    expect(normalizeHex("  #0d9488 ")).toBe("#0d9488");
  });

  it("rejects everything that is not a 3 or 6 digit hex color", () => {
    for (const bad of ["red", "#12", "#12345", "#12345g", "2f5fb3", "#aabbccdd", "var(--x)", "", null, undefined, 7]) {
      expect(normalizeHex(bad)).toBeNull();
    }
  });
});

describe("contrast", () => {
  it("matches the WCAG reference values", () => {
    expect(relativeLuminance("#ffffff")).toBeCloseTo(1, 6);
    expect(relativeLuminance("#000000")).toBe(0);
    expect(contrastRatio("#ffffff", "#000000")).toBeCloseTo(21, 5);
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777", "#777")).toBe(1);
    // #767676 on white is the well-known smallest gray that still reaches 4.5:1.
    expect(contrastRatio("#767676", "#ffffff")).toBeGreaterThan(4.5);
    expect(contrastRatio("#777777", "#ffffff")).toBeLessThan(4.5);
  });

  it("throws on input that is not hex instead of returning NaN", () => {
    expect(() => relativeLuminance("blue")).toThrow(RangeError);
    expect(() => contrastRatio("#fff", "nope")).toThrow(RangeError);
  });
});

describe("readable glyph color", () => {
  it("picks light on dark tiles and dark on light tiles", () => {
    expect(readableGlyphColor("#000000")).toBe(LIGHT_GLYPH);
    expect(readableGlyphColor("#2563eb")).toBe(LIGHT_GLYPH);
    expect(readableGlyphColor("#475569")).toBe(LIGHT_GLYPH);
    expect(readableGlyphColor("#ffffff")).toBe(DARK_GLYPH);
    expect(readableGlyphColor("#facc15")).toBe(DARK_GLYPH);
    expect(readableGlyphColor("#f97316")).toBe(DARK_GLYPH);
  });

  it("always picks the higher-contrast option", () => {
    for (const { value } of DEFAULT_COLORS) {
      const chosen = readableGlyphColor(value);
      const other = chosen === LIGHT_GLYPH ? DARK_GLYPH : LIGHT_GLYPH;
      expect(contrastRatio(chosen, value)).toBeGreaterThanOrEqual(
        contrastRatio(other, value),
      );
    }
  });

  it("keeps the glyph above 4.4:1 on every color of a coarse RGB sweep", () => {
    let worst = Infinity;
    for (let r = 0; r <= 255; r += 15) {
      for (let g = 0; g <= 255; g += 15) {
        for (let b = 0; b <= 255; b += 15) {
          const hex =
            "#" + [r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("");
          worst = Math.min(worst, contrastRatio(readableGlyphColor(hex), hex));
        }
      }
    }
    expect(worst).toBeGreaterThanOrEqual(4.4);
  });

  it("keeps the default palette at 3:1 or better for graphics", () => {
    for (const { value } of DEFAULT_COLORS) {
      expect(contrastRatio(readableGlyphColor(value), value)).toBeGreaterThanOrEqual(3);
    }
  });
});
