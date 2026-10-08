import { createSignal, onCleanup, onMount } from "solid-js";
import {
  freshConfig,
  isPort,
  loadConfig,
  saveConfig,
  type AppConfig,
  type Mode,
} from "../core/config";
import { createRule } from "../core/domains";
import {
  disableInterception,
  enableInterception,
  installInterceptAtLogin,
  installLaunchAtLogin,
  interceptionStatus,
  restartService,
  uninstallInterceptAtLogin,
  uninstallLaunchAtLogin,
  type InterceptionStatus,
} from "../core/interception";
import { buildRunOptions, requireRunOptions } from "../core/options";
import { listServers, pickServer, type Server } from "../core/servers";
import {
  daemonHealth,
  parseMetrics,
  readLogTail,
  startDaemon,
  type Health,
  type RunOptions,
  type TunnelStats,
} from "../core/wireproxy";
import {
  isInterceptAgentInstalled,
  launchdStatus,
  type LaunchdStatus,
} from "../platform/launchd";

export type Message = { text: string; kind: "info" | "error" };

export function formatBytes(value: number | null): string {
  if (value === null) return "—";
  const units = ["B", "KiB", "MiB", "GiB"];
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit++;
  }
  return `${unit > 0 && size < 10 ? size.toFixed(1) : Math.round(size)} ${units[unit]}`;
}

function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function createVpn() {
  const [config, setConfig] = createSignal<AppConfig>(freshConfig());
  const [servers, setServers] = createSignal<Server[]>([]);
  const [health, setHealth] = createSignal<Health | null>(null);
  const [stats, setStats] = createSignal<TunnelStats | null>(null);
  const [interception, setInterception] = createSignal<InterceptionStatus | null>(null);
  const [launchAgent, setLaunchAgent] = createSignal<LaunchdStatus>({
    installed: false,
    loaded: false,
    running: false,
    pid: null,
  });
  const [interceptAgent, setInterceptAgent] = createSignal(false);
  const [logLines, setLogLines] = createSignal<string[]>([]);
  const [message, setMessageRaw] = createSignal<Message | null>(null);
  const [busy, setBusy] = createSignal(false);

  let timer: ReturnType<typeof setInterval> | undefined;
  let messageTimer: ReturnType<typeof setTimeout> | undefined;

  function setMessage(next: Message | null): void {
    if (messageTimer) {
      clearTimeout(messageTimer);
      messageTimer = undefined;
    }
    setMessageRaw(next);
    if (next) {
      messageTimer = setTimeout(() => {
        setMessageRaw(null);
        messageTimer = undefined;
      }, 4000);
    }
  }

  const activeServer = () => pickServer(servers(), config().activeServer);
  const running = () => interception()?.daemon.state === "running";
  const intercepting = () => interception()?.proxy?.enabled === true;

  function runOptions(): RunOptions | null {
    return buildRunOptions(config(), servers());
  }

  async function refresh(): Promise<void> {
    const agent = await launchdStatus();
    const [serverList, inter, lines] = await Promise.all([
      listServers(),
      interceptionStatus(agent),
      readLogTail(8),
    ]);
    setServers(serverList);
    setInterception(inter);
    setLogLines(lines);
    setLaunchAgent(agent);
    setInterceptAgent(isInterceptAgentInstalled());

    if (inter.daemon.state === "running") {
      const result = await daemonHealth(config().healthPort);
      setHealth(result);
      setStats(result.metrics ? parseMetrics(result.metrics) : null);
    } else {
      setHealth(null);
      setStats(null);
    }
  }

  onMount(() => {
    void (async () => {
      setConfig(await loadConfig());
      await refresh();
      timer = setInterval(() => void refresh(), 2000);
    })();
  });

  onCleanup(() => {
    if (timer) clearInterval(timer);
    if (messageTimer) clearTimeout(messageTimer);
  });

  async function update(mutate: (draft: AppConfig) => void): Promise<void> {
    const next = structuredClone(config());
    mutate(next);
    setConfig(next);
    await saveConfig(next);
  }

  function notify(text: string, kind: Message["kind"] = "info"): void {
    setMessage({ text, kind });
  }

  async function applyRestart(): Promise<boolean> {
    const options = runOptions();
    if (!options) return false;
    try {
      await restartService(options);
      return true;
    } catch (error) {
      notify(`restart failed: ${errorText(error)}`, "error");
      return false;
    }
  }

  async function mutateAndApply(
    mutate: (draft: AppConfig) => void,
    message?: string,
  ): Promise<void> {
    await update(mutate);
    if (running() && !(await applyRestart())) {
      await refresh();
      return;
    }
    if (message) notify(message);
    await refresh();
  }

  async function toggleInterception(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    setMessage(null);
    try {
      if (intercepting()) {
        await disableInterception();
        notify("interception off");
      } else {
        const options = requireRunOptions(config(), servers());
        await update((draft) => {
          draft.activeServer = options.serverFile;
        });
        await enableInterception(options);
        notify("interception on");
      }
      await refresh();
    } catch (error) {
      notify(errorText(error), "error");
    } finally {
      setBusy(false);
    }
  }

  async function addDomain(input: string): Promise<void> {
    const rule = createRule(input, "domain");
    if (!rule) {
      notify(`invalid domain: ${input}`, "error");
      return;
    }
    await mutateAndApply((draft) => {
      draft.domains.push(rule);
    }, `added ${rule.value}`);
  }

  async function toggleRule(id: string): Promise<void> {
    await mutateAndApply((draft) => {
      const rule = draft.domains.find((candidate) => candidate.id === id);
      if (rule) rule.enabled = !rule.enabled;
    });
  }

  async function removeRule(id: string): Promise<void> {
    await mutateAndApply((draft) => {
      draft.domains = draft.domains.filter((candidate) => candidate.id !== id);
    });
  }

  async function selectServer(file: string): Promise<void> {
    await mutateAndApply((draft) => {
      draft.activeServer = file;
    }, `server: ${file}`);
  }

  async function restart(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    if (await applyRestart()) notify("daemon restarted");
    setBusy(false);
    await refresh();
  }

  async function setMode(mode: Mode): Promise<void> {
    await mutateAndApply((draft) => {
      draft.mode = mode;
    }, `mode: ${mode}`);
  }

  async function setPort(kind: "socksPort" | "healthPort", raw: string): Promise<void> {
    if (intercepting()) {
      notify("turn off interception first", "error");
      return;
    }
    const port = Number(raw.trim());
    if (!isPort(port)) {
      notify(`invalid port: ${raw}`, "error");
      return;
    }
    await mutateAndApply(
      (draft) => {
        draft[kind] = port;
      },
      `${kind === "socksPort" ? "socks" : "health"} port: ${port}`,
    );
  }

  async function toggleLaunchAtLogin(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    setMessage(null);
    try {
      if (launchAgent().installed) {
        await uninstallLaunchAtLogin();
        if (interceptAgent()) await uninstallInterceptAtLogin();
        if (intercepting()) {
          const options = runOptions();
          if (options) await startDaemon(options);
        }
        notify("launch at login off");
      } else {
        await installLaunchAtLogin(requireRunOptions(config(), servers()));
        notify("launch at login on");
      }
      await refresh();
    } catch (error) {
      notify(errorText(error), "error");
    } finally {
      setBusy(false);
    }
  }

  async function toggleInterceptAtLogin(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    setMessage(null);
    try {
      if (interceptAgent()) {
        await uninstallInterceptAtLogin();
        notify("intercept at login off");
      } else {
        if (!launchAgent().installed) {
          await installLaunchAtLogin(requireRunOptions(config(), servers()));
        }
        await installInterceptAtLogin();
        notify("intercept at login on");
      }
      await refresh();
    } catch (error) {
      notify(errorText(error), "error");
    } finally {
      setBusy(false);
    }
  }

  return {
    config,
    servers,
    health,
    stats,
    interception,
    launchAgent,
    interceptAgent,
    logLines,
    message,
    busy,
    activeServer,
    running,
    intercepting,
    toggleInterception,
    addDomain,
    toggleRule,
    removeRule,
    selectServer,
    restart,
    setMode,
    setPort,
    toggleLaunchAtLogin,
    toggleInterceptAtLogin,
    notify,
  };
}
