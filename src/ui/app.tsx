/** @jsxImportSource @opentui/solid */
import type { InputRenderable } from "@opentui/core";
import { formatKeySequence } from "@opentui/keymap/extras";
import {
  reactiveMatcherFromSignal,
  useBindings,
  useKeymapSelector,
} from "@opentui/keymap/solid";
import { useRenderer } from "@opentui/solid";
import { createMemo, createSignal, For, Show } from "solid-js";
import { serversDir } from "../core/paths";
import { theme } from "./theme";
import { createVpn, formatBytes } from "./vpn";

type Pane = "servers" | "rules";
type Screen = "main" | "settings";
type PromptKind = "domain" | "socksPort" | "healthPort";
type Prompt = { kind: PromptKind; value: string; ruleId?: string };

const SETTINGS = [
  "mode",
  "socksPort",
  "healthPort",
  "launchAtLogin",
  "interceptionAtLogin",
] as const;
type SettingKey = (typeof SETTINGS)[number];

const STATE_STYLE = {
  on: { label: "● intercepted", color: theme.ok },
  broken: { label: "! proxy set without daemon", color: theme.error },
  off: { label: "○ direct", color: theme.muted },
} as const;

const SETTING_LABELS: Record<SettingKey, string> = {
  mode: "mode",
  socksPort: "socks port",
  healthPort: "health port",
  launchAtLogin: "launch at login",
  interceptionAtLogin: "intercept at login",
};

const PROMPT_LABELS: Record<PromptKind, string> = {
  domain: "add domain",
  socksPort: SETTING_LABELS.socksPort,
  healthPort: SETTING_LABELS.healthPort,
};

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

const HINT_COMMANDS = HINT_ITEMS.map(([name]) => name);
const PROMPT_HINT_COMMANDS = PROMPT_HINT_ITEMS.map(([name]) => name);

const LIST_SELECT_PROPS = {
  flexGrow: 1,
  showDescription: false,
  backgroundColor: "transparent",
  textColor: theme.text,
  focusedBackgroundColor: "transparent",
  focusedTextColor: theme.text,
  selectedBackgroundColor: "transparent",
  selectedTextColor: theme.accent,
  keyBindings: [
    { name: "j", action: "move-down" as const },
    { name: "k", action: "move-up" as const },
  ],
};

function clampIndex(index: number, length: number): number {
  return Math.min(Math.max(index, 0), Math.max(length - 1, 0));
}

type HintSegment = { text: string; muted: boolean };

function HintText(props: { segments: readonly HintSegment[] }) {
  return (
    <text>
      <For each={props.segments}>
        {(segment) => (
          <span style={{ fg: segment.muted ? theme.muted : theme.text }}>
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

  const [pane, setPane] = createSignal<Pane>("servers");
  const [screen, setScreen] = createSignal<Screen>("main");
  const [serverIndex, setServerIndex] = createSignal(0);
  const [ruleIndex, setRuleIndex] = createSignal(0);
  const [settingIndex, setSettingIndex] = createSignal(0);
  const [prompt, setPrompt] = createSignal<Prompt | null>(null);
  let promptInput: InputRenderable | undefined;

  const stateKind = () => {
    if (vpn.intercepting() && !vpn.running()) return "broken";
    if (vpn.intercepting()) return "on";
    return "off";
  };

  const stateStyle = () => STATE_STYLE[stateKind()];

  const tunnelLabel = () => {
    if (!vpn.running()) return "daemon stopped";
    const health = vpn.health();
    if (!health) return "checking";
    if (!health.ready) return "connecting";
    const handshake = vpn.stats()?.lastHandshakeSec;
    if (!handshake) return "ready";
    const age = Math.max(0, Math.floor(Date.now() / 1000) - handshake);
    return `ready · handshake ${age}s ago`;
  };

  const transferLabel = () => {
    const stats = vpn.stats();
    if (!stats) return "";
    return `rx ${formatBytes(stats.rxBytes)} · tx ${formatBytes(stats.txBytes)}`;
  };

  const activeFile = () => vpn.activeServer()?.file ?? null;

  const selectedServer = () =>
    vpn.servers()[clampIndex(serverIndex(), vpn.servers().length)] ?? null;

  const selectedRule = () =>
    vpn.config().domains[clampIndex(ruleIndex(), vpn.config().domains.length)] ?? null;

  function moveSetting(delta: number) {
    setSettingIndex((index) => clampIndex(index + delta, SETTINGS.length));
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

  const settingValue = (key: SettingKey): string => {
    const config = vpn.config();
    switch (key) {
      case "mode":
        return config.mode === "split"
          ? "split (listed domains only)"
          : "full (all traffic)";
      case "socksPort":
        return String(config.socksPort);
      case "healthPort":
        return String(config.healthPort);
      case "launchAtLogin": {
        const agent = vpn.launchAgent();
        if (!agent.installed) return "off";
        return agent.running ? `on · running (pid ${agent.pid})` : "on · stopped";
      }
      case "interceptionAtLogin":
        return vpn.interceptAgent() ? "on · enables the system proxy at login" : "off";
    }
  };

  function activateSetting() {
    const key = SETTINGS[settingIndex()];
    switch (key) {
      case "mode":
        void vpn.setMode(vpn.config().mode === "split" ? "full" : "split");
        break;
      case "socksPort":
      case "healthPort":
        if (vpn.intercepting()) return;
        setPrompt({ kind: key, value: String(vpn.config()[key]) });
        break;
      case "launchAtLogin":
        void vpn.toggleLaunchAtLogin();
        break;
      case "interceptionAtLogin":
        void vpn.toggleInterceptAtLogin();
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
        void vpn.setPort("socksPort", trimmed);
        break;
      case "healthPort":
        void vpn.setPort("healthPort", trimmed);
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
  ];

  const when = (predicate: () => boolean) =>
    reactiveMatcherFromSignal(createMemo(predicate));

  const promptClosed = when(() => prompt() === null);
  const mainActive = when(() => prompt() === null && screen() === "main");
  const rulesActive = when(
    () => prompt() === null && screen() === "main" && pane() === "rules",
  );
  const settingsActive = when(() => prompt() === null && screen() === "settings");
  const promptActive = when(() => prompt() !== null);

  useBindings(() => ({ commands }));

  useBindings(() => ({
    enabled: promptClosed,
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
      { name: "prompt.cancel", run: () => setPrompt(null) },
    ],
    bindings: [
      { key: "return, enter", cmd: "prompt.submit" },
      { key: "escape", cmd: "prompt.cancel" },
    ],
  }));

  const hintSegments = useKeymapSelector((keymap) => {
    const build = (
      items: ReadonlyArray<readonly [string, string]>,
      commands: readonly string[],
    ) => {
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
      main: build(HINT_ITEMS, HINT_COMMANDS),
      prompt: build(PROMPT_HINT_ITEMS, PROMPT_HINT_COMMANDS),
    };
  });

  const promptLabel = () => {
    const current = prompt();
    if (!current) return "";
    return current.ruleId ? "edit domain" : PROMPT_LABELS[current.kind];
  };

  return (
    <box flexDirection="column" width="100%" height="100%">
      <box
        borderStyle="rounded"
        borderColor={theme.muted}
        title="patchbay"
        paddingLeft={1}
      >
        <box flexDirection="row" gap={1}>
          <text fg={stateStyle().color}>{stateStyle().label}</text>
          <text fg={theme.muted}>·</text>
          <text fg={theme.text}>{vpn.activeServer()?.name ?? "no server"}</text>
          <text fg={theme.muted}>·</text>
          <text fg={theme.text}>{tunnelLabel()}</text>
          <Show when={transferLabel()}>
            <text fg={theme.muted}>·</text>
            <text fg={theme.muted}>{transferLabel()}</text>
          </Show>
        </box>
      </box>

      <Show
        when={screen() === "main"}
        fallback={
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={theme.accent}
            title="settings"
            paddingLeft={1}
          >
            <For each={SETTINGS}>
              {(key, index) => (
                <text fg={index() === settingIndex() ? theme.accent : theme.text}>
                  {index() === settingIndex() ? "> " : "  "}
                  {SETTING_LABELS[key]}: {settingValue(key)}
                </text>
              )}
            </For>
          </box>
        }
      >
        <box flexGrow={1} flexDirection="row">
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={pane() === "servers" ? theme.accent : theme.muted}
            title="servers"
            paddingLeft={1}
            onMouseDown={() => setPane("servers")}
          >
            <Show
              when={vpn.servers().length > 0}
              fallback={<text fg={theme.muted}>none in {serversDir()}</text>}
            >
              <select
                {...LIST_SELECT_PROPS}
                options={vpn.servers().map((server) => ({
                  name: `${server.name}${server.file === activeFile() ? "  (active)" : ""}`,
                  description: "",
                  value: server.file,
                }))}
                focused={pane() === "servers" && prompt() === null}
                onChange={setServerIndex}
              />
            </Show>
          </box>

          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={pane() === "rules" ? theme.accent : theme.muted}
            title="domain rules"
            paddingLeft={1}
            onMouseDown={() => setPane("rules")}
          >
            <Show
              when={vpn.config().domains.length > 0}
              fallback={<text fg={theme.muted}>none — press a to add</text>}
            >
              <select
                {...LIST_SELECT_PROPS}
                options={vpn.config().domains.map((rule) => ({
                  name: `${rule.enabled ? "[x]" : "[ ]"} ${rule.value}`,
                  description: "",
                  value: rule.id,
                }))}
                focused={pane() === "rules" && prompt() === null}
                onChange={setRuleIndex}
              />
            </Show>
          </box>
        </box>
      </Show>

      <scrollbox
        borderStyle="rounded"
        borderColor={theme.muted}
        focusedBorderColor={theme.muted}
        title="log"
        paddingLeft={1}
        height={12}
        stickyScroll={true}
        stickyStart="bottom"
        verticalScrollbarOptions={{
          trackOptions: { foregroundColor: theme.muted, backgroundColor: "transparent" },
        }}
      >
        <For each={vpn.logLines()}>
          {(line) => (
            <text
              fg={
                line.includes("-> TUNNEL")
                  ? theme.ok
                  : line.includes("-> DIRECT")
                    ? theme.muted
                    : theme.text
              }
              wrapMode="none"
              truncate
            >
              {line}
            </text>
          )}
        </For>
      </scrollbox>

      <Show when={vpn.busy()}>
        <text fg={theme.muted}>working...</text>
      </Show>
      <Show when={!vpn.busy()}>
        <HintText segments={hintSegments().main} />
      </Show>

      <Show when={prompt() !== null}>
        <box
          position="absolute"
          left={0}
          top={0}
          width="100%"
          height="100%"
          alignItems="center"
          justifyContent="center"
        >
          <box
            borderStyle="rounded"
            borderColor={theme.accent}
            title={promptLabel()}
            backgroundColor={theme.surface}
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
              textColor={theme.text}
              focusedTextColor={theme.text}
              cursorColor={theme.text}
              onSubmit={() => submitPrompt()}
            />
            <box flexDirection="row" justifyContent="center">
              <HintText segments={hintSegments().prompt} />
            </box>
          </box>
        </box>
      </Show>
    </box>
  );
}
