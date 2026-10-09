import type { CliRenderer } from "@opentui/core";
import { registerCommaBindings } from "@opentui/keymap/addons";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";

export function createAppKeymap(renderer: CliRenderer) {
  const keymap = createDefaultOpenTuiKeymap(renderer);
  registerCommaBindings(keymap);
  return keymap;
}
