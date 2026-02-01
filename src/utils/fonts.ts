export const DEFAULT_UI_FONT_FAMILY =
  "\"InterVariable\", \"Inter\", -apple-system, \"Helvetica Neue\", sans-serif";

export const DEFAULT_CODE_FONT_FAMILY =
  "\"Geist Mono\", \"SF Mono\", \"SFMono-Regular\", Menlo, Monaco, monospace";

export const UI_FONT_FAMILY_OPTIONS = [
  DEFAULT_UI_FONT_FAMILY,
  "\"SF Pro Text\", \"SF Pro Display\", -apple-system, \"Helvetica Neue\", sans-serif",
];

export const CODE_FONT_FAMILY_OPTIONS = [
  DEFAULT_CODE_FONT_FAMILY,
  "\"SF Mono\", \"SFMono-Regular\", Menlo, Monaco, monospace",
  "\"JetBrains Mono\", \"SF Mono\", \"SFMono-Regular\", Menlo, Monaco, monospace",
];

export type InterFontFeature = {
  tag: string;
  label: string;
  enabledByDefault: boolean;
};

export const INTER_FONT_FEATURES: InterFontFeature[] = [
  { tag: "aalt", label: "Access All Alternates", enabledByDefault: false },
  { tag: "c2sc", label: "Small Capitals From Capitals", enabledByDefault: false },
  { tag: "calt", label: "Contextual Alternates", enabledByDefault: true },
  { tag: "case", label: "Case-Sensitive Forms", enabledByDefault: false },
  { tag: "ccmp", label: "Glyph Composition/Decomposition", enabledByDefault: true },
  { tag: "cpsp", label: "Capital Spacing", enabledByDefault: false },
  { tag: "cv01", label: "Alternate one", enabledByDefault: false },
  { tag: "cv02", label: "Open four", enabledByDefault: false },
  { tag: "cv03", label: "Open six", enabledByDefault: false },
  { tag: "cv04", label: "Open nine", enabledByDefault: false },
  { tag: "cv05", label: "Lower-case L with tail", enabledByDefault: false },
  { tag: "cv06", label: "Simplified u", enabledByDefault: false },
  { tag: "cv07", label: "Alternate German double s", enabledByDefault: false },
  { tag: "cv08", label: "Upper-case i with serif", enabledByDefault: false },
  { tag: "cv09", label: "Flat-top three", enabledByDefault: false },
  { tag: "cv10", label: "Capital G with spur", enabledByDefault: false },
  { tag: "cv11", label: "Single-story a", enabledByDefault: false },
  { tag: "cv12", label: "Compact f", enabledByDefault: false },
  { tag: "cv13", label: "Compact t", enabledByDefault: false },
  { tag: "dlig", label: "Discretionary Ligatures", enabledByDefault: false },
  { tag: "dnom", label: "Denominators", enabledByDefault: false },
  { tag: "frac", label: "Fractions", enabledByDefault: false },
  { tag: "locl", label: "Localized Forms", enabledByDefault: false },
  { tag: "numr", label: "Numerators", enabledByDefault: false },
  { tag: "ordn", label: "Ordinals", enabledByDefault: false },
  { tag: "pnum", label: "Proportional Figures", enabledByDefault: false },
  { tag: "salt", label: "Stylistic Alternates", enabledByDefault: false },
  { tag: "sinf", label: "Scientific Inferiors", enabledByDefault: false },
  { tag: "ss01", label: "Open digits", enabledByDefault: false },
  { tag: "ss02", label: "Disambiguation (with zero)", enabledByDefault: false },
  { tag: "ss03", label: "Round quotes & commas", enabledByDefault: false },
  { tag: "ss04", label: "Disambiguation (no zero)", enabledByDefault: false },
  { tag: "ss05", label: "Circled characters", enabledByDefault: false },
  { tag: "ss06", label: "Squared characters", enabledByDefault: false },
  { tag: "ss07", label: "Square punctuation", enabledByDefault: false },
  { tag: "ss08", label: "Square quotes", enabledByDefault: false },
  { tag: "subs", label: "Subscript", enabledByDefault: false },
  { tag: "sups", label: "Superscript", enabledByDefault: false },
  { tag: "tnum", label: "Tabular Figures", enabledByDefault: false },
  { tag: "zero", label: "Slashed Zero", enabledByDefault: false },
];

export const DEFAULT_INTER_FONT_FEATURES = Object.fromEntries(
  INTER_FONT_FEATURES.map((feature) => [feature.tag, feature.enabledByDefault]),
) as Record<string, boolean>;

export function normalizeInterFontFeatures(
  value: Record<string, boolean> | null | undefined,
) {
  const next: Record<string, boolean> = { ...DEFAULT_INTER_FONT_FEATURES };
  if (!value) {
    return next;
  }
  for (const feature of INTER_FONT_FEATURES) {
    if (Object.prototype.hasOwnProperty.call(value, feature.tag)) {
      next[feature.tag] = Boolean(value[feature.tag]);
    }
  }
  return next;
}

export function buildInterFontFeatureSettings(features: Record<string, boolean>) {
  return INTER_FONT_FEATURES.map(
    (feature) => `"${feature.tag}" ${features[feature.tag] ? 1 : 0}`,
  ).join(", ");
}

export function isInterFontFamily(value: string | null | undefined) {
  return /inter/i.test(value ?? "");
}

export const CODE_FONT_SIZE_DEFAULT = 11;
export const CODE_FONT_SIZE_MIN = 9;
export const CODE_FONT_SIZE_MAX = 16;

export function normalizeFontFamily(
  value: string | null | undefined,
  fallback: string,
) {
  const trimmed = value?.trim();
  return trimmed ? trimmed : fallback;
}

export function clampCodeFontSize(value: number) {
  if (!Number.isFinite(value)) {
    return CODE_FONT_SIZE_DEFAULT;
  }
  return Math.min(CODE_FONT_SIZE_MAX, Math.max(CODE_FONT_SIZE_MIN, value));
}
