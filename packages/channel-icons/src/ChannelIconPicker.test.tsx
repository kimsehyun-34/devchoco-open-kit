import { fireEvent, render, screen, within } from "@testing-library/react";
import { userEvent } from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  ChannelIconPicker,
  type ChannelIconPickerProps,
  type ChannelIconValue,
} from "./ChannelIconPicker.js";
import { CHANNEL_ICONS } from "./glyphs.js";
import { enLabels, koLabels } from "./labels.js";
import { DEFAULT_COLORS } from "./palette.js";

/** Keeps the value in state like a real form would. */
function Harness({
  initial = { icon: "general", color: null },
  onChange,
  ...rest
}: Partial<ChannelIconPickerProps> & { initial?: ChannelIconValue }) {
  const [value, setValue] = useState<ChannelIconValue>(initial);
  return (
    <ChannelIconPicker
      {...rest}
      value={value}
      onChange={(next) => {
        setValue(next);
        onChange?.(next);
      }}
    />
  );
}

const iconGroup = () => screen.getByRole("radiogroup", { name: "Icon" });
const colorGroup = () => screen.getByRole("radiogroup", { name: "Background color" });

describe("ChannelIconPicker structure and names", () => {
  it("renders two native radiogroups named by their legends", () => {
    render(<Harness />);
    expect(iconGroup().tagName).toBe("FIELDSET");
    expect(colorGroup().tagName).toBe("FIELDSET");
    expect(within(iconGroup()).getByText("Icon").tagName).toBe("LEGEND");
    for (const radio of screen.getAllByRole("radio")) {
      expect(radio).toHaveAttribute("type", "radio");
    }
  });

  it("offers every icon as a radio with its English label as the accessible name", () => {
    render(<Harness />);
    const radios = within(iconGroup()).getAllByRole("radio");
    expect(radios).toHaveLength(20);
    expect(radios.map((radio) => radio.getAttribute("value"))).toEqual(
      CHANNEL_ICONS.map((icon) => icon.key),
    );
    for (const icon of CHANNEL_ICONS) {
      expect(within(iconGroup()).getByRole("radio", { name: icon.label })).toHaveAttribute(
        "value",
        icon.key,
      );
    }
  });

  it("offers auto plus the default palette as named radios", () => {
    render(<Harness autoColor="#2563eb" />);
    const radios = within(colorGroup()).getAllByRole("radio");
    expect(radios).toHaveLength(1 + DEFAULT_COLORS.length);
    expect(radios[0]).toHaveAttribute("value", "");
    expect(radios[0]).toHaveAccessibleName("Auto (Use the project color)");
    expect(radios[0]).toBeChecked();
    for (const [hex, name] of Object.entries(enLabels.colorNames)) {
      expect(within(colorGroup()).getByRole("radio", { name })).toHaveAttribute("value", hex);
    }
  });

  it("uses exactly one radio name per group, and unique names per picker", () => {
    render(
      <>
        <Harness />
        <Harness />
      </>,
    );
    const names = (group: HTMLElement) =>
      new Set(within(group).getAllByRole("radio").map((radio) => radio.getAttribute("name")));
    const [firstIcons, secondIcons] = screen.getAllByRole("radiogroup", { name: "Icon" });
    const [firstColors] = screen.getAllByRole("radiogroup", { name: "Background color" });
    expect(names(firstIcons!).size).toBe(1);
    expect(names(firstColors!).size).toBe(1);
    expect([...names(firstIcons!)][0]).not.toBe([...names(secondIcons!)][0]);
    expect([...names(firstIcons!)][0]).not.toBe([...names(firstColors!)][0]);
  });

  it("hides the glyph and dot decoration from assistive tech", () => {
    render(<Harness />);
    expect(screen.queryByRole("img")).toBeNull();
    const dots = document.querySelectorAll(".channel-picker-dot");
    expect(dots.length).toBe(1 + DEFAULT_COLORS.length);
    for (const dot of dots) expect(dot).toHaveAttribute("aria-hidden", "true");
  });
});

describe("ChannelIconPicker selection", () => {
  it("checks the current icon and shows general for unknown or empty keys", () => {
    const { rerender } = render(
      <ChannelIconPicker value={{ icon: "dev", color: null }} onChange={() => {}} />,
    );
    expect(within(iconGroup()).getByRole("radio", { name: "Development" })).toBeChecked();
    for (const icon of ["", "spaceship"]) {
      rerender(<ChannelIconPicker value={{ icon, color: null }} onChange={() => {}} />);
      expect(within(iconGroup()).getByRole("radio", { name: "General" })).toBeChecked();
      expect(within(iconGroup()).getAllByRole("radio").filter((r) => (r as HTMLInputElement).checked)).toHaveLength(1);
    }
  });

  it("checks the current palette color, and null, invalid or missing colors mean auto", () => {
    const { rerender } = render(
      <ChannelIconPicker value={{ icon: "general", color: "#2563EB" }} onChange={() => {}} />,
    );
    expect(within(colorGroup()).getByRole("radio", { name: "Blue" })).toBeChecked();
    for (const color of [null, "red", ""]) {
      rerender(<ChannelIconPicker value={{ icon: "general", color }} onChange={() => {}} />);
      expect(within(colorGroup()).getByRole("radio", { name: /^Auto/ })).toBeChecked();
    }
  });

  it("keeps a color that is not in the palette selectable instead of dropping it", () => {
    render(
      <ChannelIconPicker value={{ icon: "general", color: "#123456" }} onChange={() => {}} />,
    );
    const current = within(colorGroup()).getByRole("radio", { name: "Current color" });
    expect(current).toBeChecked();
    expect(current).toHaveAttribute("value", "#123456");
    expect(within(colorGroup()).getAllByRole("radio")).toHaveLength(2 + DEFAULT_COLORS.length);
  });

  it("previews icons in the selected color, else the auto color", () => {
    const previews = () =>
      [...iconGroup().querySelectorAll<HTMLElement>(".channel-icon")].map((el) =>
        el.getAttribute("data-color"),
      );
    const { rerender } = render(
      <ChannelIconPicker
        value={{ icon: "general", color: null }}
        autoColor="#16a34a"
        onChange={() => {}}
      />,
    );
    expect(new Set(previews())).toEqual(new Set(["#16a34a"]));
    rerender(
      <ChannelIconPicker
        value={{ icon: "general", color: "#7c3aed" }}
        autoColor="#16a34a"
        onChange={() => {}}
      />,
    );
    expect(new Set(previews())).toEqual(new Set(["#7c3aed"]));
    rerender(<ChannelIconPicker value={{ icon: "general", color: null }} onChange={() => {}} />);
    expect(new Set(previews())).toEqual(new Set([null]));
  });
});

describe("ChannelIconPicker onChange", () => {
  it("reports the complete next value when an icon is picked", () => {
    const onChange = vi.fn();
    render(<Harness initial={{ icon: "general", color: "#dc2626" }} onChange={onChange} />);
    fireEvent.click(within(iconGroup()).getByRole("radio", { name: "Security" }));
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith({ icon: "security", color: "#dc2626" });
    expect(within(iconGroup()).getByRole("radio", { name: "Security" })).toBeChecked();
    expect(within(iconGroup()).getByRole("radio", { name: "General" })).not.toBeChecked();
  });

  it("reports a palette color, then null when auto is picked again", () => {
    const onChange = vi.fn();
    render(<Harness initial={{ icon: "docs", color: null }} onChange={onChange} />);
    fireEvent.click(within(colorGroup()).getByRole("radio", { name: "Teal" }));
    expect(onChange).toHaveBeenLastCalledWith({ icon: "docs", color: "#0d9488" });
    fireEvent.click(within(colorGroup()).getByRole("radio", { name: /^Auto/ }));
    expect(onChange).toHaveBeenLastCalledWith({ icon: "docs", color: null });
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  it("does not report a change for the radio that is already selected", () => {
    const onChange = vi.fn();
    render(<Harness initial={{ icon: "dev", color: null }} onChange={onChange} />);
    fireEvent.click(within(iconGroup()).getByRole("radio", { name: "Development" }));
    fireEvent.click(within(colorGroup()).getByRole("radio", { name: /^Auto/ }));
    expect(onChange).not.toHaveBeenCalled();
  });

  it("does not mutate the value it was given", () => {
    const value = Object.freeze({ icon: "general", color: null });
    const onChange = vi.fn();
    render(<ChannelIconPicker value={value} onChange={onChange} />);
    fireEvent.click(within(iconGroup()).getByRole("radio", { name: "Alerts" }));
    expect(onChange.mock.calls[0]![0]).not.toBe(value);
    expect(value).toEqual({ icon: "general", color: null });
  });
});

describe("ChannelIconPicker keyboard", () => {
  it("has one Tab stop per radio group, on the checked radio", async () => {
    const user = userEvent.setup();
    render(<Harness initial={{ icon: "dev", color: "#2563eb" }} />);
    await user.tab();
    expect(within(iconGroup()).getByRole("radio", { name: "Development" })).toHaveFocus();
    await user.tab();
    expect(within(colorGroup()).getByRole("radio", { name: "Blue" })).toHaveFocus();
    await user.tab();
    expect(document.body).toHaveFocus();
  });

  it("moves through icons with the arrow keys and reports each stop", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Harness initial={{ icon: "general", color: null }} onChange={onChange} />);
    await user.tab();
    await user.keyboard("{ArrowRight}");
    expect(within(iconGroup()).getByRole("radio", { name: "Notice" })).toHaveFocus();
    expect(onChange).toHaveBeenLastCalledWith({ icon: "notice", color: null });
    await user.keyboard("{ArrowDown}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "ops", color: null });
    await user.keyboard("{ArrowLeft}{ArrowUp}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "general", color: null });
    expect(onChange).toHaveBeenCalledTimes(4);
  });

  it("wraps around at both ends of the icon group", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Harness initial={{ icon: "general", color: null }} onChange={onChange} />);
    await user.tab();
    await user.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "alert", color: null });
    await user.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "general", color: null });
  });

  it("moves between auto and swatches as one color group", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Harness initial={{ icon: "general", color: null }} onChange={onChange} />);
    await user.tab();
    await user.tab();
    expect(within(colorGroup()).getByRole("radio", { name: /^Auto/ })).toHaveFocus();
    await user.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "general", color: "#475569" });
    await user.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "general", color: null });
    await user.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenLastCalledWith({ icon: "general", color: "#7c3aed" });
  });
});

describe("ChannelIconPicker props", () => {
  it("accepts a custom palette with its own names, skipping invalid and duplicate entries", () => {
    render(
      <Harness
        colors={[
          { value: "#112233", label: "Midnight" },
          { value: "#ABCDEF" },
          { value: "#abcdef", label: "Duplicate" },
          { value: "not-a-color", label: "Broken" },
        ]}
      />,
    );
    const radios = within(colorGroup()).getAllByRole("radio");
    expect(radios.map((radio) => radio.getAttribute("value"))).toEqual(["", "#112233", "#abcdef"]);
    expect(within(colorGroup()).getByRole("radio", { name: "Midnight" })).toBeInTheDocument();
    // No label and no known name: the hex value is the name.
    expect(within(colorGroup()).getByRole("radio", { name: "#abcdef" })).toBeInTheDocument();
  });

  it("translates through a full label map", () => {
    render(<Harness labels={koLabels} />);
    const icons = screen.getByRole("radiogroup", { name: "아이콘" });
    const colors = screen.getByRole("radiogroup", { name: "배경색" });
    expect(within(icons).getByRole("radio", { name: "고객 지원" })).toHaveAttribute("value", "support");
    expect(within(colors).getByRole("radio", { name: "자동 (프로젝트 색 사용)" })).toBeChecked();
    expect(within(colors).getByRole("radio", { name: "파랑" })).toHaveAttribute("value", "#2563eb");
  });

  it("overrides single strings and keeps the English defaults for the rest", () => {
    render(
      <Harness
        labels={{
          iconGroup: "Glyph",
          icons: { dev: "Engineering" },
          colorNames: { "#2563eb": "Ocean" },
          autoColorHint: "Follow the project",
        }}
      />,
    );
    const icons = screen.getByRole("radiogroup", { name: "Glyph" });
    expect(within(icons).getByRole("radio", { name: "Engineering" })).toHaveAttribute("value", "dev");
    expect(within(icons).getByRole("radio", { name: "Security" })).toBeInTheDocument();
    const colors = colorGroup();
    expect(within(colors).getByRole("radio", { name: "Ocean" })).toBeInTheDocument();
    expect(within(colors).getByRole("radio", { name: "Red" })).toBeInTheDocument();
    expect(within(colors).getByRole("radio", { name: "Auto (Follow the project)" })).toBeInTheDocument();
  });

  it("ignores undefined override values", () => {
    render(<Harness labels={{ iconGroup: undefined, icons: { dev: undefined } }} />);
    expect(iconGroup()).toBeInTheDocument();
    expect(within(iconGroup()).getByRole("radio", { name: "Development" })).toBeInTheDocument();
  });

  it("disables every radio through the fieldsets", async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Harness disabled onChange={onChange} />);
    expect(iconGroup()).toBeDisabled();
    expect(colorGroup()).toBeDisabled();
    for (const radio of screen.getAllByRole("radio")) expect(radio).toBeDisabled();
    // user-event honors `disabled` like a browser; a raw dispatched click would not.
    await user.click(within(iconGroup()).getByRole("radio", { name: "Alerts" }));
    await user.tab();
    await user.keyboard("{ArrowRight}");
    expect(onChange).not.toHaveBeenCalled();
    expect(document.body).toHaveFocus();
  });

  it("adds a custom class to the root", () => {
    const { container } = render(<Harness className="mine" />);
    expect(container.firstElementChild).toHaveClass("channel-picker", "mine");
  });
});
