import { CliRenderEvents, type CliRenderer } from "@opentui/core";
import { createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { BUILTIN_THEMES } from "./builtin";
import {
  generateSystemPalette,
  resolveTheme,
  type Palette,
  type ThemeMode,
} from "./loader";

type ThemeSource = {
  config: () => { theme: string };
  setTheme: (name: string) => Promise<void>;
};

function resolveName(name: string): string {
  return name === "system" || Object.hasOwn(BUILTIN_THEMES, name) ? name : "system";
}

export function createTheme(vpn: ThemeSource, renderer: CliRenderer) {
  const [mode, setMode] = createSignal<ThemeMode>(renderer.themeMode ?? "dark");
  const [system, setSystem] = createSignal<Palette | null>(null);
  const [preview, setPreview] = createSignal<string | null>(null);

  async function loadSystemPalette(): Promise<void> {
    try {
      const colors = await renderer.getPalette({ size: 16 });
      setSystem(generateSystemPalette(colors, mode()));
    } catch {
      setSystem(null);
    }
  }

  onMount(() => void loadSystemPalette());

  const handleMode = (next: ThemeMode) => {
    setMode(next);
    renderer.clearPaletteCache();
    void loadSystemPalette();
  };
  renderer.on(CliRenderEvents.THEME_MODE, handleMode);
  onCleanup(() => renderer.off(CliRenderEvents.THEME_MODE, handleMode));

  const palette = createMemo<Palette>(() => {
    const name = resolveName(preview() ?? vpn.config().theme);
    if (name === "system") {
      const generated = system();
      if (generated) return generated;
    }
    return resolveTheme(BUILTIN_THEMES[name] ?? BUILTIN_THEMES["opencode"]!, mode());
  });

  createEffect(() => renderer.setBackgroundColor(palette().background));

  return {
    palette,
    active: () => resolveName(vpn.config().theme),
    setActive: (name: string) => vpn.setTheme(name),
    preview: (name: string) => setPreview(name),
    clearPreview: () => setPreview(null),
  };
}

export { THEME_NAMES } from "./builtin";
export type { Palette } from "./loader";
