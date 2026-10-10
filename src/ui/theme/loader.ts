import { RGBA, type TerminalColors } from "@opentui/core";

export type ThemeMode = "dark" | "light";

export type ColorValue = string | { dark: string; light: string };

export type ThemeJson = {
  $schema?: string;
  defs?: Record<string, string>;
  theme: Record<string, ColorValue>;
};

export type ThemeColor =
  | "accent"
  | "background"
  | "backgroundElement"
  | "backgroundMenu"
  | "backgroundPanel"
  | "border"
  | "borderActive"
  | "borderSubtle"
  | "diffAdded"
  | "diffAddedBg"
  | "diffAddedLineNumberBg"
  | "diffContext"
  | "diffContextBg"
  | "diffHighlightAdded"
  | "diffHighlightRemoved"
  | "diffHunkHeader"
  | "diffLineNumber"
  | "diffRemoved"
  | "diffRemovedBg"
  | "diffRemovedLineNumberBg"
  | "error"
  | "info"
  | "markdownBlockQuote"
  | "markdownCode"
  | "markdownCodeBlock"
  | "markdownEmph"
  | "markdownHeading"
  | "markdownHorizontalRule"
  | "markdownImage"
  | "markdownImageText"
  | "markdownLink"
  | "markdownLinkText"
  | "markdownListEnumeration"
  | "markdownListItem"
  | "markdownStrong"
  | "markdownText"
  | "primary"
  | "secondary"
  | "selectedListItemText"
  | "success"
  | "syntaxComment"
  | "syntaxFunction"
  | "syntaxKeyword"
  | "syntaxNumber"
  | "syntaxOperator"
  | "syntaxPunctuation"
  | "syntaxString"
  | "syntaxType"
  | "syntaxVariable"
  | "text"
  | "textMuted"
  | "warning";

export type Palette = Record<ThemeColor, RGBA>;

function resolveColor(
  value: ColorValue,
  mode: ThemeMode,
  defs: Record<string, string>,
  theme: Record<string, ColorValue>,
): RGBA {
  if (typeof value === "object") return resolveColor(value[mode], mode, defs, theme);
  if (value === "transparent") return RGBA.fromInts(0, 0, 0, 0);
  if (value.startsWith("#")) return RGBA.fromHex(value);
  const next = defs[value] ?? theme[value];
  if (next === undefined) throw new Error(`Unresolved theme color "${value}"`);
  return resolveColor(next, mode, defs, theme);
}

export function resolveTheme(json: ThemeJson, mode: ThemeMode): Palette {
  const defs = json.defs ?? {};
  const palette: Record<string, RGBA> = {};
  for (const [token, value] of Object.entries(json.theme)) {
    palette[token] = resolveColor(value, mode, defs, json.theme);
  }
  palette.selectedListItemText ??= palette.background!;
  palette.backgroundMenu ??= palette.backgroundElement!;
  return palette as Palette;
}

export function luminance(color: RGBA): number {
  const [r, g, b] = color.toInts();
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

function generateGrayScale(bg: RGBA, isDark: boolean): Record<number, RGBA> {
  const [bgR, bgG, bgB] = bg.toInts();
  const lum = luminance(bg);
  const grays: Record<number, RGBA> = {};
  for (let i = 1; i <= 12; i++) {
    const factor = i / 12;
    let r: number;
    let g: number;
    let b: number;
    if (isDark) {
      if (lum < 10) {
        r = g = b = Math.floor(factor * 0.4 * 255);
      } else {
        const newLum = lum + (255 - lum) * factor * 0.4;
        const ratio = newLum / lum;
        r = Math.min(bgR * ratio, 255);
        g = Math.min(bgG * ratio, 255);
        b = Math.min(bgB * ratio, 255);
      }
    } else if (lum > 245) {
      r = g = b = Math.floor(255 - factor * 0.4 * 255);
    } else {
      const newLum = lum * (1 - factor * 0.4);
      const ratio = newLum / lum;
      r = Math.max(bgR * ratio, 0);
      g = Math.max(bgG * ratio, 0);
      b = Math.max(bgB * ratio, 0);
    }
    grays[i] = RGBA.fromInts(Math.floor(r), Math.floor(g), Math.floor(b));
  }
  return grays;
}

function generateMutedTextColor(bg: RGBA, isDark: boolean): RGBA {
  const bgLum = luminance(bg);
  let gray: number;
  if (isDark) {
    gray = bgLum < 10 ? 180 : Math.min(Math.floor(160 + bgLum * 0.3), 200);
  } else {
    gray = bgLum > 245 ? 75 : Math.max(Math.floor(100 - (255 - bgLum) * 0.2), 60);
  }
  return RGBA.fromInts(gray, gray, gray);
}

function tint(base: RGBA, overlay: RGBA, alpha: number): RGBA {
  const r = base.r + (overlay.r - base.r) * alpha;
  const g = base.g + (overlay.g - base.g) * alpha;
  const b = base.b + (overlay.b - base.b) * alpha;
  return RGBA.fromInts(Math.round(r * 255), Math.round(g * 255), Math.round(b * 255));
}

export function generateSystemPalette(
  colors: TerminalColors,
  mode: ThemeMode,
): Palette | null {
  const background = colors.palette[0];
  if (!background) return null;
  const bg = RGBA.fromHex(colors.defaultBackground ?? background);
  const fg = RGBA.fromHex(colors.defaultForeground ?? colors.palette[7] ?? "#ffffff");
  const transparent = RGBA.fromValues(bg.r, bg.g, bg.b, 0);
  const isDark = mode === "dark";

  const col = (index: number) => {
    const value = colors.palette[index];
    return value ? RGBA.fromHex(value) : RGBA.fromIndex(index);
  };

  const grays = generateGrayScale(bg, isDark);
  const textMuted = generateMutedTextColor(bg, isDark);
  const ansi = {
    red: col(1),
    green: col(2),
    yellow: col(3),
    blue: col(4),
    magenta: col(5),
    cyan: col(6),
    redBright: col(9),
    greenBright: col(10),
  };

  const diffAlpha = isDark ? 0.22 : 0.14;
  const diffContextBg = grays[2]!;

  return {
    primary: ansi.cyan,
    secondary: ansi.magenta,
    accent: ansi.cyan,
    error: ansi.red,
    warning: ansi.yellow,
    success: ansi.green,
    info: ansi.cyan,
    text: fg,
    textMuted,
    selectedListItemText: bg,
    background: transparent,
    backgroundPanel: grays[2]!,
    backgroundElement: grays[3]!,
    backgroundMenu: grays[3]!,
    borderSubtle: grays[6]!,
    border: grays[7]!,
    borderActive: grays[8]!,
    diffAdded: ansi.green,
    diffRemoved: ansi.red,
    diffContext: grays[7]!,
    diffHunkHeader: grays[7]!,
    diffHighlightAdded: ansi.greenBright,
    diffHighlightRemoved: ansi.redBright,
    diffAddedBg: tint(bg, ansi.green, diffAlpha),
    diffRemovedBg: tint(bg, ansi.red, diffAlpha),
    diffContextBg,
    diffLineNumber: textMuted,
    diffAddedLineNumberBg: tint(diffContextBg, ansi.green, diffAlpha),
    diffRemovedLineNumberBg: tint(diffContextBg, ansi.red, diffAlpha),
    markdownText: fg,
    markdownHeading: fg,
    markdownLink: ansi.blue,
    markdownLinkText: ansi.cyan,
    markdownCode: ansi.green,
    markdownBlockQuote: ansi.yellow,
    markdownEmph: ansi.yellow,
    markdownStrong: fg,
    markdownHorizontalRule: grays[7]!,
    markdownListItem: ansi.blue,
    markdownListEnumeration: ansi.cyan,
    markdownImage: ansi.blue,
    markdownImageText: ansi.cyan,
    markdownCodeBlock: fg,
    syntaxComment: textMuted,
    syntaxKeyword: ansi.magenta,
    syntaxFunction: ansi.blue,
    syntaxVariable: fg,
    syntaxString: ansi.green,
    syntaxNumber: ansi.yellow,
    syntaxType: ansi.cyan,
    syntaxOperator: ansi.cyan,
    syntaxPunctuation: fg,
  };
}
