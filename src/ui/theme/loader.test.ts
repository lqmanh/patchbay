import { describe, expect, test } from "bun:test";
import { RGBA, type TerminalColors } from "@opentui/core";
import { BUILTIN_THEMES } from "./builtin";
import { generateSystemPalette, luminance, resolveTheme } from "./loader";

const MODES = ["dark", "light"] as const;

function terminalColors(palette: Array<string | null>): TerminalColors {
  return {
    palette,
    defaultForeground: "#c0caf5",
    defaultBackground: "#1a1b26",
    cursorColor: null,
    mouseForeground: null,
    mouseBackground: null,
    tekForeground: null,
    tekBackground: null,
    highlightBackground: null,
    highlightForeground: null,
  };
}

describe("resolveTheme", () => {
  test("resolves every built-in theme in both modes", () => {
    for (const [name, json] of Object.entries(BUILTIN_THEMES)) {
      for (const mode of MODES) {
        const palette = resolveTheme(json, mode);
        const values = Object.values(palette);
        expect(values, `${name} ${mode}`).toHaveLength(52);
        for (const value of values) {
          expect(value).toBeInstanceOf(RGBA);
        }
        if (!("selectedListItemText" in json.theme)) {
          expect(palette.selectedListItemText.equals(palette.background)).toBe(true);
        }
        if (!("backgroundMenu" in json.theme)) {
          expect(palette.backgroundMenu.equals(palette.backgroundElement)).toBe(true);
        }
      }
    }
  });

  test("resolves variants, refs and defaults", () => {
    const tokyonight = resolveTheme(BUILTIN_THEMES["tokyonight"]!, "dark");
    expect(tokyonight.text.equals(RGBA.fromHex("#c8d3f5"))).toBe(true);
    expect(tokyonight.primary.equals(RGBA.fromHex("#82aaff"))).toBe(true);
    expect(tokyonight.backgroundPanel.equals(RGBA.fromHex("#1e2030"))).toBe(true);
    expect(tokyonight.selectedListItemText.equals(tokyonight.background)).toBe(true);
    expect(tokyonight.backgroundMenu.equals(tokyonight.backgroundElement)).toBe(true);

    const carbonfox = resolveTheme(BUILTIN_THEMES["carbonfox"]!, "light");
    expect(carbonfox.diffLineNumber.equals(carbonfox.textMuted)).toBe(true);
  });
});

describe("generateSystemPalette", () => {
  test("generates the full palette from terminal colors", () => {
    const colors = terminalColors([
      "#1a1b26",
      "#f7768e",
      "#9ece6a",
      "#e0af68",
      "#7aa2f7",
      "#bb9af7",
      "#7dcfff",
      "#c0caf5",
      "#414868",
      "#ff7a93",
      "#b9f27c",
      "#ff9e64",
      "#7da6ff",
      "#bb9af7",
      "#0db9d7",
      "#acb0d0",
    ]);
    const palette = generateSystemPalette(colors, "dark")!;
    expect(Object.values(palette)).toHaveLength(52);
    expect(palette.text.equals(RGBA.fromHex("#c0caf5"))).toBe(true);
    expect(palette.success.equals(RGBA.fromHex("#9ece6a"))).toBe(true);
    expect(palette.primary.equals(RGBA.fromHex("#7dcfff"))).toBe(true);
    expect(luminance(palette.borderActive)).toBeGreaterThan(luminance(palette.border));
    expect(luminance(palette.border)).toBeGreaterThan(luminance(palette.backgroundPanel));
    expect(palette.diffContextBg.equals(palette.backgroundPanel)).toBe(true);
    expect(palette.diffLineNumber.equals(palette.textMuted)).toBe(true);
    expect(palette.diffAddedBg.equals(palette.diffRemovedBg)).toBe(false);
  });

  test("uses indexed slots for colors the terminal did not report", () => {
    const palette = generateSystemPalette(terminalColors(["#1a1b26"]), "dark")!;
    expect(palette.primary.intent).toBe("indexed");
    expect(palette.primary.slot).toBe(6);
  });

  test("falls back when the terminal reports no palette", () => {
    expect(generateSystemPalette(terminalColors([]), "dark")).toBeNull();
  });
});
