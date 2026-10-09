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
  ensureDaemon,
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
  readLog,
  startDaemon,
  waitForReachable,
  type Health,
  type RunOptions,
  type TunnelStats,
} from "../core/wireproxy";
import {
  isInterceptAgentInstalled,
  launchdStatus,
  type LaunchdStatus,
} from "../platform/launchd";

const HEALTH_POLL_MS = 60_000;
const LOG_POLL_MS = 5000;
const LOG_WINDOW_LINES = 1000;
const LOG_KEEP_LINES = 2000;

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
  const [busy, setBusy] = createSignal(false);

  let timer: ReturnType<typeof setInterval> | undefined;
  let logTimer: ReturnType<typeof setInterval> | undefined;

  const activeServer = () => pickServer(servers(), config().activeServer);
  const running = () => interception()?.daemon.state === "running";
  const intercepting = () => interception()?.proxy?.enabled === true;

  function runOptions(): RunOptions | null {
    return buildRunOptions(config(), servers());
  }

  async function refresh(): Promise<void> {
    const agent = await launchdStatus();
    const [serverList, inter] = await Promise.all([
      listServers(),
      interceptionStatus(agent),
    ]);
    setServers(serverList);
    setInterception(inter);
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

  async function refreshLog(): Promise<void> {
    setLogLines(await readLog(LOG_WINDOW_LINES, LOG_KEEP_LINES));
  }

  async function healBrokenInterception(): Promise<boolean> {
    const inter = interception();
    if (!inter?.proxy?.enabled || inter.daemon.state === "running") return false;
    try {
      await ensureDaemon(requireRunOptions(config(), servers()));
      await waitForReachable(config().healthPort, 8, 250);
    } catch {
      await disableInterception().catch(() => undefined);
    }
    return true;
  }

  onMount(() => {
    void (async () => {
      setConfig(await loadConfig());
      void refreshLog();
      await refresh();
      if (await healBrokenInterception()) await refresh();
      timer = setInterval(() => void refresh(), HEALTH_POLL_MS);
      logTimer = setInterval(() => void refreshLog(), LOG_POLL_MS);
    })();
  });

  onCleanup(() => {
    if (timer) clearInterval(timer);
    if (logTimer) clearInterval(logTimer);
  });

  async function update(mutate: (draft: AppConfig) => void): Promise<void> {
    const next = structuredClone(config());
    mutate(next);
    setConfig(next);
    await saveConfig(next);
  }

  async function applyRestart(): Promise<void> {
    const options = runOptions();
    if (!options) return;
    await restartService(options).catch(() => undefined);
  }

  async function mutateAndApply(mutate: (draft: AppConfig) => void): Promise<void> {
    await update(mutate);
    if (running()) await applyRestart();
    await refresh();
  }

  async function runBusy(task: () => Promise<void>): Promise<void> {
    if (busy()) return;
    setBusy(true);
    try {
      await task();
    } catch {
      // errors are intentionally silent
    } finally {
      setBusy(false);
    }
  }

  async function toggleInterception(): Promise<void> {
    await runBusy(async () => {
      if (intercepting()) {
        await disableInterception();
      } else {
        const options = requireRunOptions(config(), servers());
        await update((draft) => {
          draft.activeServer = options.serverFile;
        });
        await enableInterception(options);
      }
      await refresh();
    });
  }

  async function addDomain(input: string): Promise<void> {
    const rule = createRule(input, "domain");
    if (!rule) return;
    await mutateAndApply((draft) => {
      draft.domains.push(rule);
    });
  }

  async function updateRule(id: string, input: string): Promise<void> {
    const existing = config().domains.find((rule) => rule.id === id);
    if (!existing) return;
    const rule = createRule(input, existing.kind);
    if (!rule) return;
    await mutateAndApply((draft) => {
      const target = draft.domains.find((candidate) => candidate.id === id);
      if (target) target.value = rule.value;
    });
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
    });
  }

  async function restart(): Promise<void> {
    await runBusy(async () => {
      await applyRestart();
      await refresh();
    });
  }

  async function setMode(mode: Mode): Promise<void> {
    await mutateAndApply((draft) => {
      draft.mode = mode;
    });
  }

  async function setPort(kind: "socksPort" | "healthPort", raw: string): Promise<void> {
    if (intercepting()) return;
    const port = Number(raw.trim());
    if (!isPort(port)) return;
    await mutateAndApply((draft) => {
      draft[kind] = port;
    });
  }

  async function toggleLaunchAtLogin(): Promise<void> {
    await runBusy(async () => {
      if (launchAgent().installed) {
        await uninstallLaunchAtLogin();
        if (interceptAgent()) await uninstallInterceptAtLogin();
        if (intercepting()) {
          const options = runOptions();
          if (options) await startDaemon(options);
        }
      } else {
        await installLaunchAtLogin(requireRunOptions(config(), servers()));
      }
      await refresh();
    });
  }

  async function toggleInterceptAtLogin(): Promise<void> {
    await runBusy(async () => {
      if (interceptAgent()) {
        await uninstallInterceptAtLogin();
      } else {
        if (!launchAgent().installed) {
          await installLaunchAtLogin(requireRunOptions(config(), servers()));
        }
        await installInterceptAtLogin();
      }
      await refresh();
    });
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
    busy,
    activeServer,
    running,
    intercepting,
    toggleInterception,
    addDomain,
    updateRule,
    toggleRule,
    removeRule,
    selectServer,
    restart,
    setMode,
    setPort,
    toggleLaunchAtLogin,
    toggleInterceptAtLogin,
  };
}
