import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { ChannelIcon } from "./ChannelIcon.js";
import { DARK_GLYPH, LIGHT_GLYPH } from "./color.js";
import { CHANNEL_ICONS, getChannelIcon } from "./glyphs.js";

const tile = (ui: ReactElement) =>
  render(ui).container.querySelector<HTMLElement>(".channel-icon")!;
const drawn = (el: Element) =>
  [...el.querySelectorAll("path")].map((path) => path.getAttribute("d"));

describe("ChannelIcon", () => {
  it("renders a tile with one inline SVG built from the glyph paths", () => {
    const el = tile(<ChannelIcon icon="dev" />);
    expect(el).toHaveAttribute("data-icon", "dev");
    const svg = el.querySelector("svg")!;
    expect(svg).toHaveAttribute("viewBox", "0 0 24 24");
    expect(svg).toHaveAttribute("stroke", "currentColor");
    expect(svg).toHaveAttribute("aria-hidden", "true");
    expect(drawn(el)).toEqual([...getChannelIcon("dev").paths]);
  });

  it("renders all 20 glyphs", () => {
    for (const { key, paths } of CHANNEL_ICONS) {
      const { container, unmount } = render(<ChannelIcon icon={key} />);
      expect(container.querySelector(".channel-icon")).toHaveAttribute("data-icon", key);
      expect(drawn(container)).toEqual([...paths]);
      unmount();
    }
  });

  it("falls back to the general glyph for empty, missing and unknown keys", () => {
    const general = drawn(tile(<ChannelIcon icon="general" />));
    for (const icon of [undefined, null, "", "spaceship"]) {
      const el = tile(<ChannelIcon icon={icon} />);
      expect(el).toHaveAttribute("data-icon", "general");
      expect(drawn(el)).toEqual(general);
    }
  });

  it("is decorative by default and hidden from assistive tech", () => {
    const el = tile(<ChannelIcon icon="ops" />);
    expect(el).toHaveAttribute("aria-hidden", "true");
    expect(el).not.toHaveAttribute("role");
    expect(el).not.toHaveAttribute("aria-label");
    expect(screen.queryByRole("img")).toBeNull();
  });

  it("becomes a named image when a label is given", () => {
    render(<ChannelIcon icon="ops" label="Operations channel" />);
    const img = screen.getByRole("img", { name: "Operations channel" });
    expect(img).not.toHaveAttribute("aria-hidden");
    expect(img.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
  });

  it("treats a blank label as decorative", () => {
    const el = tile(<ChannelIcon label="   " />);
    expect(el).toHaveAttribute("aria-hidden", "true");
    expect(screen.queryByRole("img")).toBeNull();
  });

  it("sets the background as a CSS variable and picks a readable glyph color", () => {
    const dark = tile(<ChannelIcon color="#2563EB" />);
    expect(dark.style.getPropertyValue("--channel-icon-bg")).toBe("#2563eb");
    expect(dark.style.getPropertyValue("--channel-icon-fg")).toBe(LIGHT_GLYPH);
    expect(dark).toHaveAttribute("data-color", "#2563eb");

    const light = tile(<ChannelIcon color="#facc15" />);
    expect(light.style.getPropertyValue("--channel-icon-fg")).toBe(DARK_GLYPH);

    const short = tile(<ChannelIcon color="#fff" />);
    expect(short.style.getPropertyValue("--channel-icon-bg")).toBe("#ffffff");
    expect(short.style.getPropertyValue("--channel-icon-fg")).toBe(DARK_GLYPH);
  });

  it("leaves colors to the stylesheet when the color is missing or not hex", () => {
    for (const color of [undefined, null, "", "red", "var(--brand)", "#12345g"]) {
      const el = tile(<ChannelIcon color={color} />);
      expect(el.style.getPropertyValue("--channel-icon-bg")).toBe("");
      expect(el.style.getPropertyValue("--channel-icon-fg")).toBe("");
      expect(el).not.toHaveAttribute("data-color");
    }
  });

  it("scales the glyph with the tile but never below 12px", () => {
    const small = tile(<ChannelIcon size={16} />);
    expect(small.style.getPropertyValue("--channel-icon-size")).toBe("16px");
    expect(small.querySelector("svg")).toHaveAttribute("width", "12");

    const big = tile(<ChannelIcon size={40} />);
    expect(big.style.getPropertyValue("--channel-icon-size")).toBe("40px");
    expect(big.querySelector("svg")).toHaveAttribute("width", "23");
    expect(big.querySelector("svg")).toHaveAttribute("height", "23");
  });

  it("ignores a nonsensical size", () => {
    for (const size of [0, -4, Number.NaN]) {
      expect(
        tile(<ChannelIcon size={size} />).style.getPropertyValue("--channel-icon-size"),
      ).toBe("24px");
    }
  });

  it("merges className, style and extra attributes", () => {
    const el = tile(
      <ChannelIcon
        className="mine"
        style={{ marginInlineEnd: 4 }}
        data-testid="x"
        color="#16a34a"
      />,
    );
    expect(el).toHaveClass("channel-icon", "mine");
    expect(el.style.marginInlineEnd).toBe("4px");
    expect(el.style.getPropertyValue("--channel-icon-bg")).toBe("#16a34a");
    expect(el).toHaveAttribute("data-testid", "x");
  });
});
