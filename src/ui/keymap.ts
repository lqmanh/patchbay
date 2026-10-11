import type { CliRenderer } from "@opentui/core";
import { registerCommaBindings } from "@opentui/keymap/addons";
import { createDefaultOpenTuiKeymap } from "@opentui/keymap/opentui";

export function createAppKeymap(renderer: CliRenderer) {
  const keymap = createDefaultOpenTuiKeymap(renderer);
  registerCommaBindings(keymap);
  return keymap;
}

export const COMMAND_LABELS = {
  "app.quit": "quit",
  "screen.settings": "settings",
  "screen.back": "back",
  "settings.change": "change",
  "pane.toggle": "pane",
  "interception.toggle": "intercept",
  "rule.add": "add",
  "rule.edit": "edit",
  "rule.delete": "delete",
  "item.activate": "activate",
  "daemon.restart": "restart",
  "picker.confirm": "apply",
  "picker.cancel": "cancel",
  "prompt.submit": "confirm",
  "prompt.cancel": "cancel",
  "list.up": "up",
  "list.down": "down",
  "list.page-up": "page up",
  "list.page-down": "page down",
  "list.first": "first",
  "list.last": "last",
} as const;

export type CommandName = keyof typeof COMMAND_LABELS;

type KeyContext = "global" | "base" | "main" | "rules" | "settings" | "prompt" | "picker";

type KeyBinding = { key: string; cmd: CommandName };

const ENTER_KEY = "return, enter, linefeed, kpenter";

const LIST_BINDINGS: readonly KeyBinding[] = [
  { key: "up, k", cmd: "list.up" },
  { key: "down, j", cmd: "list.down" },
  { key: "pageup", cmd: "list.page-up" },
  { key: "pagedown", cmd: "list.page-down" },
  { key: "home", cmd: "list.first" },
  { key: "end", cmd: "list.last" },
];

// Invariant: printable keys must never be always-on; text entry is gated by matchers.
export const KEY_BINDINGS: Record<KeyContext, readonly KeyBinding[]> = {
  global: [{ key: "ctrl+c", cmd: "app.quit" }],
  base: [{ key: "q", cmd: "app.quit" }],
  main: [
    { key: "s", cmd: "screen.settings" },
    { key: "tab", cmd: "pane.toggle" },
    { key: "space, t", cmd: "interception.toggle" },
    { key: "a", cmd: "rule.add" },
    { key: ENTER_KEY, cmd: "item.activate" },
    { key: "r", cmd: "daemon.restart" },
    ...LIST_BINDINGS,
  ],
  rules: [
    { key: "e", cmd: "rule.edit" },
    { key: "d", cmd: "rule.delete" },
  ],
  settings: [
    { key: "s, escape", cmd: "screen.back" },
    { key: ENTER_KEY, cmd: "settings.change" },
    ...LIST_BINDINGS,
  ],
  prompt: [
    { key: ENTER_KEY, cmd: "prompt.submit" },
    { key: "escape", cmd: "prompt.cancel" },
  ],
  picker: [
    { key: ENTER_KEY, cmd: "picker.confirm" },
    { key: "escape", cmd: "picker.cancel" },
    ...LIST_BINDINGS,
  ],
};

type HintContext = "main" | "prompt" | "picker";

export const HINTS: Record<HintContext, readonly CommandName[]> = {
  main: [
    "interception.toggle",
    "pane.toggle",
    "rule.add",
    "rule.edit",
    "rule.delete",
    "item.activate",
    "daemon.restart",
    "settings.change",
    "screen.settings",
    "screen.back",
    "app.quit",
  ],
  prompt: ["prompt.submit", "prompt.cancel"],
  picker: ["picker.confirm", "picker.cancel"],
};
