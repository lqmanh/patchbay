/** @jsxImportSource @opentui/solid */
import { useKeyboard, useRenderer } from "@opentui/solid";
import { createSignal, For, Show } from "solid-js";
import { serversDir } from "../core/paths";
import { createVpn, formatBytes } from "./vpn";

type Pane = "servers" | "rules";
type Screen = "main" | "settings";
type PromptKind = "domain" | "socksPort" | "healthPort";
type Prompt = { kind: PromptKind; value: string };

const SETTINGS = [
  "mode",
  "socksPort",
  "healthPort",
  "launchAtLogin",
  "interceptionAtLogin",
] as const;
type SettingKey = (typeof SETTINGS)[number];

const STATE_STYLE = {
  on: { label: "● intercepted", color: "green" },
  broken: { label: "! proxy set without daemon", color: "red" },
  off: { label: "○ direct", color: "#888888" },
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
  socksPort: "socks port",
  healthPort: "health port",
};

function clampIndex(index: number, length: number): number {
  return Math.min(Math.max(index, 0), Math.max(length - 1, 0));
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

  function move(delta: number) {
    if (pane() === "servers") {
      setServerIndex((index) => clampIndex(index + delta, vpn.servers().length));
    } else {
      setRuleIndex((index) => clampIndex(index + delta, vpn.config().domains.length));
    }
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
        if (vpn.intercepting()) {
          vpn.notify("turn off interception first", "error");
          return;
        }
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
    setPrompt(null);
    if (!current) return;
    const value = current.value.trim();
    if (!value) return;
    switch (current.kind) {
      case "domain":
        void vpn.addDomain(value);
        break;
      case "socksPort":
        void vpn.setPort("socksPort", value);
        break;
      case "healthPort":
        void vpn.setPort("healthPort", value);
        break;
    }
  }

  useKeyboard((key) => {
    const current = prompt();
    if (current) {
      if (key.name === "escape") {
        setPrompt(null);
        return;
      }
      if (key.name === "return" || key.name === "enter") {
        submitPrompt();
        return;
      }
      if (key.name === "backspace") {
        setPrompt({ ...current, value: current.value.slice(0, -1) });
        return;
      }
      if (!key.ctrl && !key.meta && key.sequence && key.sequence.length === 1) {
        setPrompt({ ...current, value: current.value + key.sequence });
      }
      return;
    }

    if (screen() === "settings") {
      switch (key.name) {
        case "q":
          renderer.destroy();
          break;
        case "s":
        case "escape":
          setScreen("main");
          break;
        case "j":
        case "down":
          setSettingIndex((index) => Math.min(index + 1, SETTINGS.length - 1));
          break;
        case "k":
        case "up":
          setSettingIndex((index) => Math.max(index - 1, 0));
          break;
        case "return":
        case "enter":
          activateSetting();
          break;
      }
      return;
    }

    switch (key.name) {
      case "q":
        renderer.destroy();
        break;
      case "s":
        setScreen("settings");
        break;
      case "tab":
        setPane((current) => (current === "servers" ? "rules" : "servers"));
        break;
      case "j":
      case "down":
        move(1);
        break;
      case "k":
      case "up":
        move(-1);
        break;
      case "space":
      case "t":
        void vpn.toggleInterception();
        break;
      case "a":
        setPrompt({ kind: "domain", value: "" });
        break;
      case "d": {
        const rule = selectedRule();
        if (pane() === "rules" && rule) void vpn.removeRule(rule.id);
        break;
      }
      case "return":
      case "enter":
        activate();
        break;
      case "r":
        void vpn.restart();
        break;
    }
  });

  const hints = () => {
    if (screen() === "settings") return "enter change · s or esc back · q quit";
    return "space intercept · tab pane · a add · d delete · enter activate · r restart · s settings · q quit";
  };

  const messageColor = () => {
    const message = vpn.message();
    return message?.kind === "error" ? "red" : "green";
  };

  const promptLabel = () => {
    const current = prompt();
    return current ? PROMPT_LABELS[current.kind] : "";
  };

  return (
    <box flexDirection="column" width="100%" height="100%">
      <box borderStyle="rounded" borderColor="#666666" title="patchbay" paddingLeft={1}>
        <box flexDirection="row" gap={1}>
          <text fg={stateStyle().color}>{stateStyle().label}</text>
          <text fg="#666666">·</text>
          <text>{vpn.activeServer()?.name ?? "no server"}</text>
          <text fg="#666666">·</text>
          <text>{tunnelLabel()}</text>
          <Show when={transferLabel()}>
            <text fg="#666666">·</text>
            <text fg="#888888">{transferLabel()}</text>
          </Show>
        </box>
      </box>

      <Show
        when={screen() === "main"}
        fallback={
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor="cyan"
            title="settings"
            paddingLeft={1}
            marginTop={1}
          >
            <For each={SETTINGS}>
              {(key, index) => (
                <text fg={index() === settingIndex() ? "cyan" : undefined}>
                  {index() === settingIndex() ? "> " : "  "}
                  {SETTING_LABELS[key]}: {settingValue(key)}
                </text>
              )}
            </For>
          </box>
        }
      >
        <box flexGrow={1} flexDirection="row" gap={1} marginTop={1}>
          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={pane() === "servers" ? "cyan" : "#666666"}
            title="servers"
            paddingLeft={1}
          >
            <Show
              when={vpn.servers().length > 0}
              fallback={<text fg="#666666">none in {serversDir()}</text>}
            >
              <For each={vpn.servers()}>
                {(server, index) => (
                  <text
                    fg={
                      index() === serverIndex() && pane() === "servers"
                        ? "cyan"
                        : undefined
                    }
                  >
                    {index() === serverIndex() ? "> " : "  "}
                    {server.name}
                    {server.file === activeFile() ? "  (active)" : ""}
                  </text>
                )}
              </For>
            </Show>
          </box>

          <box
            flexGrow={1}
            borderStyle="rounded"
            borderColor={pane() === "rules" ? "cyan" : "#666666"}
            title="domain rules"
            paddingLeft={1}
          >
            <Show
              when={vpn.config().domains.length > 0}
              fallback={<text fg="#666666">none — press a to add</text>}
            >
              <For each={vpn.config().domains}>
                {(rule, index) => (
                  <text
                    fg={
                      index() === ruleIndex() && pane() === "rules" ? "cyan" : undefined
                    }
                  >
                    {index() === ruleIndex() ? "> " : "  "}
                    {rule.enabled ? "[x]" : "[ ]"} {rule.value}
                  </text>
                )}
              </For>
            </Show>
          </box>
        </box>
      </Show>

      <box
        borderStyle="rounded"
        borderColor="#666666"
        title="log"
        paddingLeft={1}
        marginTop={1}
        height={9}
      >
        <For each={vpn.logLines()}>
          {(line) => (
            <text
              fg={
                line.includes("-> TUNNEL")
                  ? "green"
                  : line.includes("-> DIRECT")
                    ? "#888888"
                    : undefined
              }
              wrapMode="none"
              truncate
            >
              {line}
            </text>
          )}
        </For>
      </box>

      <Show when={prompt() !== null}>
        <text fg="cyan">
          {promptLabel()}: {prompt()?.value}█
        </text>
      </Show>
      <Show when={vpn.message()}>
        <text fg={messageColor()}>{vpn.message()?.text}</text>
      </Show>
      <Show when={vpn.busy()}>
        <text fg="#888888">working...</text>
      </Show>
      <Show when={!vpn.busy()}>
        <text fg="#666666">{hints()}</text>
      </Show>
    </box>
  );
}
