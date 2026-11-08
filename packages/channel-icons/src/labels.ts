import { CHANNEL_ICONS, type ChannelIconKey } from "./glyphs.js";

/** Every user-facing string the picker renders, so the UI can be translated. */
export interface ChannelIconLabels {
  /** Accessible name per glyph key. */
  icons: Record<ChannelIconKey, string>;
  /** Accessible names for palette colors, keyed by lowercase `#rrggbb`. */
  colorNames: Record<string, string>;
  /** Legend of the icon radio group. */
  iconGroup: string;
  /** Legend of the color radio group. */
  colorGroup: string;
  /** Visible text of the "auto" color choice. */
  autoColor: string;
  /** Extra accessible text for the "auto" choice; explains what it does. */
  autoColorHint: string;
  /** Name of a selected color that is not part of the palette. */
  currentColor: string;
}

/** What a caller may pass: any subset, with nested maps partial too. */
export type ChannelIconLabelOverrides = Partial<
  Omit<ChannelIconLabels, "icons" | "colorNames">
> & {
  icons?: Partial<Record<ChannelIconKey, string>>;
  colorNames?: Record<string, string>;
};

/** English defaults. Icon labels come from the glyph array, the single source of truth. */
export const enLabels: ChannelIconLabels = {
  icons: Object.fromEntries(
    CHANNEL_ICONS.map((icon) => [icon.key, icon.label]),
  ) as Record<ChannelIconKey, string>,
  colorNames: {
    "#475569": "Slate",
    "#dc2626": "Red",
    "#f97316": "Orange",
    "#facc15": "Yellow",
    "#16a34a": "Green",
    "#0d9488": "Teal",
    "#2563eb": "Blue",
    "#7c3aed": "Violet",
  },
  iconGroup: "Icon",
  colorGroup: "Background color",
  autoColor: "Auto",
  autoColorHint: "Use the project color",
  currentColor: "Current color",
};

/** Korean map: an example of a full translation that can be passed as `labels`. */
export const koLabels: ChannelIconLabels = {
  icons: {
    general: "일반",
    notice: "공지",
    ops: "운영",
    dev: "개발",
    talk: "대화",
    design: "디자인",
    marketing: "마케팅",
    support: "고객 지원",
    planning: "기획",
    schedule: "일정",
    docs: "문서",
    security: "보안",
    people: "인사",
    finance: "재무",
    issue: "이슈",
    release: "배포",
    question: "질문",
    social: "잡담",
    celebrate: "축하",
    alert: "알림",
  },
  colorNames: {
    "#475569": "슬레이트",
    "#dc2626": "빨강",
    "#f97316": "주황",
    "#facc15": "노랑",
    "#16a34a": "초록",
    "#0d9488": "청록",
    "#2563eb": "파랑",
    "#7c3aed": "보라",
  },
  iconGroup: "아이콘",
  colorGroup: "배경색",
  autoColor: "자동",
  autoColorHint: "프로젝트 색 사용",
  currentColor: "현재 색",
};

/** Drops `undefined` values so a partial override never blanks a default. */
function defined(
  map: Readonly<Record<string, string | undefined>> | undefined,
): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(map ?? {})) {
    if (value !== undefined) result[key] = value;
  }
  return result;
}

/** Merges overrides onto a base (English by default) and returns a complete label set. */
export function resolveLabels(
  overrides: ChannelIconLabelOverrides = {},
  base: ChannelIconLabels = enLabels,
): ChannelIconLabels {
  return {
    iconGroup: overrides.iconGroup ?? base.iconGroup,
    colorGroup: overrides.colorGroup ?? base.colorGroup,
    autoColor: overrides.autoColor ?? base.autoColor,
    autoColorHint: overrides.autoColorHint ?? base.autoColorHint,
    currentColor: overrides.currentColor ?? base.currentColor,
    icons: { ...base.icons, ...defined(overrides.icons) },
    colorNames: { ...base.colorNames, ...defined(overrides.colorNames) },
  };
}
