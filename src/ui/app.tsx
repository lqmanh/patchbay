/** @jsxImportSource @opentui/solid */
import { SelectRenderable, type InputRenderable, type MouseEvent } from "@opentui/core";
import { formatKeySequence } from "@opentui/keymap/extras";
import {
  reactiveMatcherFromSignal,
  useBindings,
  useKeymapSelector,
} from "@opentui/keymap/solid";
import { useRenderer } from "@opentui/solid";
import { createMemo, createSignal, For, Show } from "solid-js";
import { serversDir } from "../core/paths";
import { Dialog } from "./dialog";
import { createTheme, THEME_NAMES, type Palette } from "./theme";
import { createVpn } from "./vpn";

type Pane = "servers" | "rules";
type Screen = "main" | "settings";
type PromptKind = "domain" | "socksPort";
type Prompt = { kind: PromptKind; value: string; ruleId?: string };
type StateKind = "on" | "broken" | "off";

const SETTINGS = [
  "mode",
  "socksPort",
  "launchAtLogin",
  "interceptionAtLogin",
  "theme",
] as const;
type SettingKey = (typeof SETTINGS)[number];

function stateStyle(palette: Palette, kind: StateKind) {
  if (kind === "on") return { label: "● intercepted", color: palette.success };
  if (kind === "broken") {
    return { label: "! proxy set without daemon", color: palette.error };
  }
  return { label: "○ direct", color: palette.textMuted };
}

const SETTING_LABELS: Record<SettingKey, string> = {
  mode: "mode",
  socksPort: "socks port",
  launchAtLogin: "launch at login",
  interceptionAtLogin: "intercept at login",
  theme: "theme",
};

const PROMPT_LABELS: Record<PromptKind, string> = {
  domain: "ADD DOMAIN",
  socksPort: "SOCKS PORT",
};

function logColor(line: string, palette: Palette) {
  if (line.includes("ERROR") || line.includes("WARN")) return palette.error;
  if (line.includes("endpoint/wireguard")) return palette.success;
  if (line.includes("outbound/direct")) return palette.textMuted;
  return palette.text;
}

const HINT_ITEMS: ReadonlyArray<readonly [string, string]> = [
  ["interception.toggle", "intercept"],
  ["pane.toggle", "pane"],
  ["rule.add", "add"],
  ["rule.edit", "edit"],
  ["rule.delete", "delete"],
  ["item.activate", "activate"],
  ["daemon.restart", "restart"],
  ["settings.change", "change"],
  ["screen.settings", "settings"],
  ["screen.back", "back"],
  ["app.quit", "quit"],
];

const PROMPT_HINT_ITEMS: ReadonlyArray<readonly [string, string]> = [
  ["prompt.submit", "confirm"],
  ["prompt.cancel", "cancel"],
];

const PICKER_HINT_ITEMS: ReadonlyArray<readonly [string, string]> = [
  ["theme.confirm", "apply"],
  ["theme.cancel", "cancel"],
];

type SelectViewport = Omit<
  SelectRenderable,
  "scrollOffset" | "maxVisibleItems" | "linesPerItem"
> & {
  scrollOffset: number;
  maxVisibleItems: number;
  linesPerItem: number;
};

// scrollOffset/maxVisibleItems/linesPerItem are private in the SelectRenderable
// types but plain fields at runtime; the viewport cannot be scrolled otherwise.
function selectViewport(target: unknown): SelectViewport | null {
  return target instanceof SelectRenderable
    ? (target as unknown as SelectViewport)
    : null;
}

function wheelSelect(event: MouseEvent) {
  const select = selectViewport(event.target);
  if (!select) return;
  const direction = event.scroll?.direction;
  if (direction !== "up" && direction !== "down") return;
  const step = Math.max(1, Math.round(event.scroll?.delta ?? 1));
  const max = Math.max(0, select.options.length - select.maxVisibleItems);
  const next = clampIndex(
    select.scrollOffset + (direction === "down" ? step : -step),
    max + 1,
  );
  if (next === select.scrollOffset) return;
  select.scrollOffset = next;
  select.requestRender();
}

function clickSelect(event: MouseEvent) {
  const select = selectViewport(event.target);
  if (!select) return;
  const row =
    Math.floor((event.y - select.y) / select.linesPerItem) + select.scrollOffset;
  if (row < 0 || row >= select.options.length) return;
  select.setSelectedIndex(row);
}

const SELECT_KEY_BINDINGS = [
  { name: "j", action: "move-down" as const },
  { name: "k", action: "move-up" as const },
];

function listSelectProps(palette: Palette) {
  return {
    flexGrow: 1,
    showDescription: false,
    wrapSelection: true,
    onMouseDown: clickSelect,
    onMouseScroll: wheelSelect,
    backgroundColor: "transparent",
    textColor: palette.text,
    focusedBackgroundColor: "transparent",
    focusedTextColor: palette.text,
    selectedBackgroundColor: "transparent",
    selectedTextColor: palette.primary,
    keyBindings: SELECT_KEY_BINDINGS,
  };
}

function clampIndex(index: number, length: number): number {
  return Math.min(Math.max(index, 0), Math.max(length - 1, 0));
}

type HintSegment = { text: string; muted: boolean };

function HintText(props: { segments: readonly HintSegment[]; palette: Palette }) {
  return (
    <text>
      <For each={props.segments}>
        {(segment) => (
          <span
            style={{ fg: segment.muted ? props.palette.textMuted : props.palette.text }}
          >
            {segment.text}
          </span>
        )}
      </For>
    </text>
  );
}

export function App() {
  const renderer = useRenderer();
  const vpn = createVpn();
  const theme = createTheme(vpn, renderer);

  const [pane, setPane] = createSignal<Pane>("servers");
  const [screen, setScreen] = createSignal<Screen>("main");
  const [serverIndex, setServerIndex] = createSignal(0);
  const [ruleIndex, setRuleIndex] = createSignal(0);
  const [settingIndex, setSettingIndex] = createSignal(0);
  const [prompt, setPrompt] = createSignal<Prompt | null>(null);
  const [pickerIndex, setPickerIndex] = createSignal<number | null>(null);
  let promptInput: InputRenderable | undefined;

  const stateKind = (): StateKind => {
    if (vpn.intercepting() && !vpn.running()) return "broken";
    if (vpn.intercepting()) return "on";
    return "off";
  };

  const currentState = createMemo(() => stateStyle(theme.palette(), stateKind()));

  const tunnelLabel = () => {
    if (!vpn.running()) return "daemon stopped";
    const ready = vpn.ready();
    if (ready === null) return "checking";
    if (!ready) return "connecting";
    return "ready";
  };

  const activeFile = () => vpn.activeServer()?.file ?? null;

  const selectedServer = () =>
    vpn.servers()[clampIndex(serverIndex(), vpn.servers().length)] ?? null;

  const selectedRule = () =>
    vpn.config().domains[clampIndex(ruleIndex(), vpn.config().domains.length)] ?? null;

  function moveSetting(delta: number) {
    setSettingIndex((index) => (index + delta + SETTINGS.length) % SETTINGS.length);
  }

  function activate() {
    if (pane() === "servers") {
      const server = selectedServer();
      if (server) void vpn.selectServer(server.file);
      return;
    }
    const rule = selectedRule();
    if (rule) void vpn.toggleRule(rule.id);
  }

  function openPicker() {
    setPickerIndex(THEME_NAMES.indexOf(theme.active()));
  }

  function movePicker(index: number) {
    setPickerIndex(index);
    const name = THEME_NAMES[index];
    if (name) theme.preview(name);
  }

  function applyPicker() {
    const index = pickerIndex();
    const name = index === null ? undefined : THEME_NAMES[index];
    if (name) void theme.setActive(name);
    cancelPicker();
  }

  function cancelPicker() {
    theme.clearPreview();
    setPickerIndex(null);
  }

  const settingValue = (key: SettingKey): string => {
    const config = vpn.config();
    switch (key) {
      case "mode":
        return config.mode === "split"
          ? "split (listed domains only)"
          : "full (all traffic)";
      case "socksPort":
        return String(config.socksPort);
      case "launchAtLogin": {
        const agent = vpn.launchAgent();
        if (!agent.installed) return "off";
        return agent.running ? `on · running (pid ${agent.pid})` : "on · stopped";
      }
      case "interceptionAtLogin":
        return vpn.interceptAgent() ? "on · enables the system proxy at login" : "off";
      case "theme":
        return theme.active();
    }
  };

  function activateSetting() {
    const key = SETTINGS[settingIndex()];
    switch (key) {
      case "mode":
        void vpn.setMode(vpn.config().mode === "split" ? "full" : "split");
        break;
      case "socksPort":
        if (vpn.intercepting()) return;
        setPrompt({ kind: key, value: String(vpn.config()[key]) });
        break;
      case "launchAtLogin":
        void vpn.toggleLaunchAtLogin();
        break;
      case "interceptionAtLogin":
        void vpn.toggleInterceptAtLogin();
        break;
      case "theme":
        openPicker();
        break;
      default:
        break;
    }
  }

  function submitPrompt() {
    const current = prompt();
    if (!current) return;
    const trimmed = (promptInput?.value ?? "").trim();
    setPrompt(null);
    if (!trimmed) return;
    switch (current.kind) {
      case "domain":
        if (current.ruleId) void vpn.updateRule(current.ruleId, trimmed);
        else void vpn.addDomain(trimmed);
        break;
      case "socksPort":
        void vpn.setSocksPort(trimmed);
        break;
    }
  }

  const commands = [
    { name: "app.quit", run: () => renderer.destroy() },
    { name: "screen.settings", run: () => setScreen("settings") },
    { name: "screen.back", run: () => setScreen("main") },
    { name: "settings.change", run: activateSetting },
    {
      name: "pane.toggle",
      run: () => setPane((current) => (current === "servers" ? "rules" : "servers")),
    },
    { name: "settings.down", run: () => moveSetting(1) },
    { name: "settings.up", run: () => moveSetting(-1) },
    { name: "interception.toggle", run: () => void vpn.toggleInterception() },
    { name: "rule.add", run: () => setPrompt({ kind: "domain", value: "" }) },
    {
      name: "rule.edit",
      run: () => {
        const rule = selectedRule();
        if (rule) setPrompt({ kind: "domain", value: rule.value, ruleId: rule.id });
      },
    },
    {
      name: "rule.delete",
      run: () => {
        const rule = selectedRule();
        if (rule) void vpn.removeRule(rule.id);
      },
    },
    { name: "item.activate", run: activate },
    { name: "daemon.restart", run: () => void vpn.restart() },
    { name: "theme.confirm", run: applyPicker },
    {
      name: "theme.cancel",
      run: cancelPicker,
    },
  ];

  const when = (predicate: () => boolean) =>
    reactiveMatcherFromSignal(createMemo(predicate));

  const overlayOpen = createMemo(() => prompt() !== null || pickerIndex() !== null);

  const overlayClosed = when(() => !overlayOpen());
  const mainActive = when(() => !overlayOpen() && screen() === "main");
  const rulesActive = when(
    () => !overlayOpen() && screen() === "main" && pane() === "rules",
  );
  const settingsActive = when(() => !overlayOpen() && screen() === "settings");
  const promptActive = when(() => prompt() !== null);
  const pickerActive = when(() => pickerIndex() !== null);

  useBindings(() => ({ commands }));

  useBindings(() => ({
    enabled: overlayClosed,
    bindings: [{ key: "q", cmd: "app.quit" }],
  }));

  useBindings(() => ({
    enabled: mainActive,
    bindings: [
      { key: "s", cmd: "screen.settings" },
      { key: "tab", cmd: "pane.toggle" },
      { key: "space, t", cmd: "interception.toggle" },
      { key: "a", cmd: "rule.add" },
      { key: "return, enter", cmd: "item.activate" },
      { key: "r", cmd: "daemon.restart" },
    ],
  }));

  useBindings(() => ({
    enabled: rulesActive,
    bindings: [
      { key: "e", cmd: "rule.edit" },
      { key: "d", cmd: "rule.delete" },
    ],
  }));

  useBindings(() => ({
    enabled: settingsActive,
    bindings: [
      { key: "s", cmd: "screen.back" },
      { key: "escape", cmd: "screen.back" },
      { key: "j, down", cmd: "settings.down" },
      { key: "k, up", cmd: "settings.up" },
      { key: "return, enter", cmd: "settings.change" },
    ],
  }));

  useBindings(() => ({
    enabled: promptActive,
    commands: [
      { name: "prompt.submit", run: () => submitPrompt() },
      {
        name: "prompt.cancel",
        run: () => {
          setPrompt(null);
        },
      },
    ],
    bindings: [
      { key: "return, enter", cmd: "prompt.submit" },
      { key: "escape", cmd: "prompt.cancel" },
    ],
  }));

  useBindings(() => ({
    enabled: pickerActive,
    bindings: [
      { key: "return, enter", cmd: "theme.confirm" },
      { key: "escape", cmd: "theme.cancel" },
    ],
  }));

  const hintSegments = useKeymapSelector((keymap) => {
    const build = (items: ReadonlyArray<readonly [string, string]>) => {
      const commands = items.map(([name]) => name);
      const bindings = keymap.getCommandBindings({ visibility: "active", commands });
      const segments: HintSegment[] = [];
      for (const [name, label] of items) {
        const binding = bindings.get(name)?.[0];
        if (!binding) continue;
        if (segments.length > 0) segments.push({ text: " · ", muted: true });
        segments.push({
          text: formatKeySequence(binding.sequence, {
            keyNameAliases: { return: "enter" },
          }),
          muted: false,
        });
        segments.push({ text: ` ${label}`, muted: true });
      }
      return segments;
    };
    return {
      main: build(HINT_ITEMS),
      prompt: build(PROMPT_HINT_ITEMS),
      picker: build(PICKER_HINT_ITEMS),
    };
  });

  const promptLabel = () => {
    const current = prompt();
    if (!current) return "";
    return current.ruleId ? "EDIT DOMAIN" : PROMPT_LABELS[current.kind];
  };

  return (
    <box flexDirection="column" width="100%" height="100%">
      <box
        borderStyle="rounded"
        borderColor={theme.palette().border}
        title="PATCHBAY"
        paddingLeft={1}
      >
        <box flexDirection="row" gap={1}>
          <text fg={currentState().color}>{currentState().label}</text>
          <text fg={theme.palette().textMuted}>·</text>
          <text fg={theme.palette().text}>{vpn.activeServer()?.name ?? "no server"}</text>
          <text fg={theme.palette().textMuted}>·</text>
          <text fg={theme.palette().text}>{tunnelLabel()}</text>
        </box>
      </box>

      <Show
        when={screen() === "main"}
        fallback={
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={theme.palette().borderActive}
            title="SETTINGS"
            paddingLeft={1}
          >
            <select
              {...listSelectProps(theme.palette())}
              options={SETTINGS.map((key) => ({
                name: `${SETTING_LABELS[key]}: ${settingValue(key)}`,
                description: "",
              }))}
              selectedIndex={settingIndex()}
              onChange={setSettingIndex}
            />
          </box>
        }
      >
        <box flexGrow={1} flexDirection="row">
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={
              pane() === "servers" ? theme.palette().borderActive : theme.palette().border
            }
            title="SERVERS"
            paddingLeft={1}
            onMouseDown={() => setPane("servers")}
          >
            <Show
              when={vpn.servers().length > 0}
              fallback={
                <text fg={theme.palette().textMuted}>none in {serversDir()}</text>
              }
            >
              <select
                {...listSelectProps(theme.palette())}
                options={vpn.servers().map((server) => ({
                  name: `${server.name}${server.file === activeFile() ? "  (active)" : ""}`,
                  description: "",
                }))}
                focused={pane() === "servers" && !overlayOpen()}
                onChange={setServerIndex}
              />
            </Show>
          </box>

          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={
              pane() === "rules" ? theme.palette().borderActive : theme.palette().border
            }
            title="DOMAIN RULES"
            paddingLeft={1}
            onMouseDown={() => setPane("rules")}
          >
            <Show
              when={vpn.config().domains.length > 0}
              fallback={<text fg={theme.palette().textMuted}>none — press a to add</text>}
            >
              <select
                {...listSelectProps(theme.palette())}
                options={vpn.config().domains.map((rule) => ({
                  name: `${rule.enabled ? "[x]" : "[ ]"} ${rule.value}`,
                  description: "",
                }))}
                focused={pane() === "rules" && !overlayOpen()}
                onChange={setRuleIndex}
              />
            </Show>
          </box>
        </box>
      </Show>

      <scrollbox
        ref={(el) => (el.content.maxWidth = undefined)}
        borderStyle="rounded"
        borderColor={theme.palette().border}
        focusedBorderColor={theme.palette().border}
        title="LOG"
        paddingLeft={1}
        height={12}
        stickyScroll={true}
        stickyStart="bottom"
        verticalScrollbarOptions={{
          visible: false,
        }}
        horizontalScrollbarOptions={{
          visible: false,
        }}
      >
        <For each={vpn.logLines()}>
          {(line) => (
            <text
              fg={logColor(line, theme.palette())}
              wrapMode="none"
              width={line.length}
            >
              {line}
            </text>
          )}
        </For>
      </scrollbox>

      <Show when={vpn.busy()}>
        <text fg={theme.palette().textMuted}>working...</text>
      </Show>
      <Show when={!vpn.busy()}>
        <HintText segments={hintSegments().main} palette={theme.palette()} />
      </Show>

      <Show when={prompt() !== null}>
        <Dialog onClose={() => setPrompt(null)}>
          <box
            borderStyle="rounded"
            borderColor={theme.palette().borderActive}
            title={promptLabel()}
            backgroundColor={theme.palette().backgroundPanel}
            width="70%"
            paddingLeft={1}
            paddingRight={1}
          >
            <input
              ref={(el) => {
                promptInput = el;
                el.value = prompt()?.value ?? "";
                el.focus();
              }}
              flexGrow={1}
              textColor={theme.palette().text}
              focusedTextColor={theme.palette().text}
              cursorColor={theme.palette().primary}
              onSubmit={() => submitPrompt()}
            />
            <box flexDirection="row" justifyContent="center">
              <HintText segments={hintSegments().prompt} palette={theme.palette()} />
            </box>
          </box>
        </Dialog>
      </Show>

      <Show when={pickerIndex() !== null}>
        <Dialog onClose={cancelPicker}>
          <box
            borderStyle="rounded"
            borderColor={theme.palette().borderActive}
            title="THEME"
            backgroundColor={theme.palette().backgroundPanel}
            width="70%"
            height={16}
            paddingLeft={1}
            paddingRight={1}
          >
            <select
              {...listSelectProps(theme.palette())}
              options={THEME_NAMES.map((name) => ({
                name,
                description: "",
              }))}
              selectedIndex={pickerIndex() ?? 0}
              focused={true}
              onChange={movePicker}
            />
            <box flexDirection="row" justifyContent="center">
              <HintText segments={hintSegments().picker} palette={theme.palette()} />
            </box>
          </box>
        </Dialog>
      </Show>
    </box>
  );
}
