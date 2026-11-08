import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (name: string) =>
  readFileSync(new URL(name, import.meta.url), "utf8");
const css = read("./styles.css");
const readme = read("../README.md");

describe("stylesheet contract", () => {
  it("documents every public custom property in the README", () => {
    const names = new Set(css.match(/--channel-(?:icon|picker)-[a-z]+/g));
    expect(names.size).toBeGreaterThanOrEqual(10);
    for (const name of names) expect(readme, name).toContain("`" + name + "`");
  });

  it("documents every public custom property in the CSS header", () => {
    const header = css.slice(0, css.indexOf("*/"));
    const used = new Set(css.slice(css.indexOf("*/")).match(/--channel-(?:icon|picker)-[a-z]+/g));
    for (const name of used) expect(header, name).toContain(name);
  });

  it("keeps the picker usable at 320px: wrapping grids and a shrinkable fieldset", () => {
    expect(css).toMatch(/\.channel-picker-group\s*{[^}]*min-inline-size:\s*0/);
    expect(css).toMatch(/\.channel-picker-icons\s*{[^}]*repeat\(\s*auto-fill/);
    expect(css).toMatch(/\.channel-picker-colors\s*{[^}]*flex-wrap:\s*wrap/);
    expect(css).not.toMatch(/overflow-x:\s*scroll/);
  });

  it("has visible focus and a forced-colors block", () => {
    expect(css).toMatch(/:focus-visible\s*\+\s*\.channel-picker-face\s*{[^}]*outline:\s*2px/);
    expect(css).toContain("@media (forced-colors: active)");
    // Forced colors: selection by border shape, never a Highlight fill (it hides text in Chromium).
    const forced = css.slice(css.indexOf("@media (forced-colors: active)"));
    expect(forced).toMatch(/border:\s*5px double Highlight/);
    expect(forced).not.toMatch(/background:\s*Highlight/);
    // Dark defaults follow the page's color-scheme (not the OS), with plain light fallbacks.
    expect(css).toContain("@supports (color: light-dark(#000, #fff))");
    expect(css).toMatch(/--_text:\s*var\(--channel-picker-text,\s*light-dark\(/);
    expect(css).not.toContain("prefers-color-scheme");
  });
});
