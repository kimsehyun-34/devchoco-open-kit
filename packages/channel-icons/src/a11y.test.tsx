import { render } from "@testing-library/react";
import axe from "axe-core";
import { describe, expect, it } from "vitest";
import { ChannelIcon } from "./ChannelIcon.js";
import { ChannelIconPicker } from "./ChannelIconPicker.js";
import { koLabels } from "./labels.js";
import { DEFAULT_COLORS } from "./palette.js";

/**
 * Runs axe-core over a rendered component and returns one readable line per violation.
 *
 * jsdom has no layout engine, so axe cannot measure colour contrast or target size here
 * (`color-contrast` is switched off instead of silently skipped). Contrast is covered by the
 * maths tests in `color.test.ts`; the rest is checked with `axe` in a real browser.
 * `region` is off because a component is not a whole page.
 */
async function violations(container: Element): Promise<string[]> {
  const result = await axe.run(container, {
    runOnly: {
      type: "tag",
      values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa", "best-practice"],
    },
    rules: { "color-contrast": { enabled: false }, region: { enabled: false } },
  });
  return result.violations.map(
    (violation) => `${violation.id}: ${violation.nodes.map((node) => node.target.join(" ")).join(", ")}`,
  );
}

const noop = () => {};

describe("ChannelIcon accessibility", () => {
  it("is hidden from assistive technology when it is decoration", async () => {
    const { container } = render(
      <p>
        <ChannelIcon icon="dev" color="#2563eb" /> engineering
      </p>,
    );
    expect(container.querySelector("[aria-hidden='true']")).not.toBeNull();
    expect(await violations(container)).toEqual([]);
  });

  it("is an image with a name when it stands alone", async () => {
    const { container, getByRole } = render(
      <ChannelIcon icon="alert" color="#dc2626" size={40} label="Alerts channel" />,
    );
    expect(getByRole("img", { name: "Alerts channel" })).toBeTruthy();
    expect(await violations(container)).toEqual([]);
  });

  it("stays valid for unknown icons, missing colours and tiny sizes", async () => {
    const { container } = render(
      <div>
        <ChannelIcon icon="spaceship" label="Unknown icon" />
        <ChannelIcon icon={null} color={null} label="Nothing set" size={12} />
        <ChannelIcon icon="docs" color="not-a-colour" label="Bad colour" />
      </div>,
    );
    expect(await violations(container)).toEqual([]);
  });
});

describe("ChannelIconPicker accessibility", () => {
  const base = { value: { icon: "general", color: null }, onChange: noop };

  it("has no violations with the default labels", async () => {
    const { container } = render(<ChannelIconPicker {...base} />);
    expect(await violations(container)).toEqual([]);
  });

  it("has no violations with the Korean labels and an auto colour", async () => {
    const { container } = render(
      <ChannelIconPicker {...base} labels={koLabels} autoColor="#2563eb" />,
    );
    expect(await violations(container)).toEqual([]);
  });

  it("has no violations while disabled", async () => {
    const { container } = render(<ChannelIconPicker {...base} disabled />);
    expect(await violations(container)).toEqual([]);
  });

  it("has no violations when the selection is outside the palette or unknown", async () => {
    const { container } = render(
      <ChannelIconPicker value={{ icon: "spaceship", color: "#db2777" }} onChange={noop} />,
    );
    expect(await violations(container)).toEqual([]);
  });

  it("has no violations with a custom palette that has names for some colours only", async () => {
    const colors = [
      { value: "#112233", label: "Midnight" },
      { value: "#aabbcc" },
      ...DEFAULT_COLORS.slice(0, 2),
    ];
    const { container } = render(<ChannelIconPicker {...base} colors={colors} />);
    expect(await violations(container)).toEqual([]);
  });

  it("gives every control in a group its own accessible name", () => {
    const { container } = render(<ChannelIconPicker {...base} />);
    for (const group of container.querySelectorAll("fieldset")) {
      const names = [...group.querySelectorAll("input[type='radio']")].map(
        (input) => input.closest("label")?.textContent?.trim() || input.getAttribute("aria-label"),
      );
      expect(names.length).toBeGreaterThan(1);
      expect(names.every(Boolean), "every radio has a name").toBe(true);
      expect(new Set(names).size, "names are unique within a group").toBe(names.length);
    }
  });
});
