import { rm } from "node:fs/promises";
import { join } from "node:path";
import {
  activeNetworkService,
  disableSocksProxy,
  getSocksProxy,
  setSocksProxy,
  type SocksState,
} from "../platform/macos";
import {
  installInterceptAgent,
  installLaunchAgent,
  isLaunchAgentInstalled,
  launchdStatus,
  ensureLaunchAgentRunning,
  uninstallInterceptAgent,
  uninstallLaunchAgent,
  type LaunchdStatus,
} from "../platform/launchd";
import { writeJson } from "./json";
import { daemonConfigFile, logFile, proxyStateFile, stateDir } from "./paths";
import {
  daemonStatus,
  requireSingboxBinary,
  restartDaemon,
  startDaemon,
  stopDaemon,
  writeGeneratedConfig,
  type DaemonStatus,
  type RunOptions,
} from "./singbox";

export type SavedProxyState = SocksState & { service: string };

export type InterceptionStatus = {
  daemon: DaemonStatus;
  service: string | null;
  proxy: SocksState | null;
};

export function captureStateBeforeEnable(
  current: SocksState,
  socksPort: number,
): SocksState | null {
  const alreadyOurs =
    current.enabled && current.host === "127.0.0.1" && current.port === socksPort;
  return alreadyOurs ? null : current;
}

export async function saveProxyState(state: SavedProxyState): Promise<void> {
  await writeJson(proxyStateFile(), state);
}

export async function loadProxyState(): Promise<SavedProxyState | null> {
  try {
    const raw = (await Bun.file(proxyStateFile()).json()) as Record<string, unknown>;
    return {
      service: typeof raw.service === "string" ? raw.service : "",
      enabled: raw.enabled === true,
      host: typeof raw.host === "string" ? raw.host : "",
      port: typeof raw.port === "number" ? raw.port : 0,
    };
  } catch {
    return null;
  }
}

export async function clearProxyState(): Promise<void> {
  await rm(proxyStateFile(), { force: true });
}

async function daemonView(agent?: LaunchdStatus): Promise<DaemonStatus> {
  if (isLaunchAgentInstalled()) {
    const status = agent ?? (await launchdStatus());
    return status.running && status.pid
      ? { state: "running", pid: status.pid }
      : { state: "stopped" };
  }
  return daemonStatus();
}

export async function interceptionStatus(
  agent?: LaunchdStatus,
): Promise<InterceptionStatus> {
  const [daemon, service] = await Promise.all([
    daemonView(agent),
    activeNetworkService(),
  ]);
  const proxy = service ? await getSocksProxy(service) : null;
  return { daemon, service, proxy };
}

export async function ensureDaemon(options: RunOptions): Promise<boolean> {
  if (isLaunchAgentInstalled()) {
    await writeGeneratedConfig(options);
    await ensureLaunchAgentRunning();
    return false;
  }
  if ((await daemonStatus()).state === "running") return false;
  await startDaemon(options);
  return true;
}

export async function enableInterception(options: RunOptions): Promise<void> {
  const startedManually = await ensureDaemon(options);

  const service = await activeNetworkService();
  if (!service) {
    if (startedManually) await stopDaemon();
    throw new Error(
      "no active network service; disconnect any other VPN app (its tunnel becomes the default route) and retry",
    );
  }

  const current = await getSocksProxy(service);
  const toSave = captureStateBeforeEnable(current, options.socksPort);
  if (toSave) await saveProxyState({ service, ...toSave });

  await setSocksProxy(service, "127.0.0.1", options.socksPort);
}

export async function disableInterception(): Promise<void> {
  const saved = await loadProxyState();
  const service = saved?.service || (await activeNetworkService());
  if (service) {
    if (saved?.enabled) await setSocksProxy(service, saved.host, saved.port);
    else await disableSocksProxy(service);
  }
  await clearProxyState();
  if (!isLaunchAgentInstalled()) await stopDaemon();
}

export async function restartService(options: RunOptions): Promise<void> {
  if (isLaunchAgentInstalled()) {
    await writeGeneratedConfig(options);
    await installLaunchAgent(requireSingboxBinary(), daemonConfigFile(), logFile());
  } else {
    await restartDaemon(options);
  }
}

export async function installLaunchAtLogin(options: RunOptions): Promise<void> {
  const binary = requireSingboxBinary();

  await writeGeneratedConfig(options);
  await stopDaemon();
  await installLaunchAgent(binary, daemonConfigFile(), logFile());
}

export async function uninstallLaunchAtLogin(): Promise<void> {
  await uninstallLaunchAgent();
}

export const INTERCEPT_ARGS = ["intercept", "on"] as const;

export function interceptCommand(): string[] {
  const entry = join(import.meta.dir, "..", "index.tsx");
  const bun = Bun.which("bun");
  return bun ? [bun, entry, ...INTERCEPT_ARGS] : [process.execPath, ...INTERCEPT_ARGS];
}

export async function installInterceptAtLogin(): Promise<void> {
  await installInterceptAgent(interceptCommand(), join(stateDir(), "intercept.log"));
}

export async function uninstallInterceptAtLogin(): Promise<void> {
  await uninstallInterceptAgent();
}
