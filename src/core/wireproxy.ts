import { closeSync, existsSync, openSync } from "node:fs";
import { rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import type { Mode } from "./config";
import { enabledPatterns, type DomainRule } from "./domains";
import { ensureDirs, logFile, pidFile, wireproxyConfigFile } from "./paths";
import { readServerConfig } from "./servers";

export type RunOptions = {
  serverFile: string;
  socksPort: number;
  healthPort: number;
  mode: Mode;
  domains: readonly DomainRule[];
};

export type DaemonStatus =
  | { state: "stopped" }
  | { state: "running"; pid: number }
  | { state: "stale"; pid: number };

export type Health = {
  reachable: boolean;
  ready: boolean | null;
  metrics: string | null;
};

export type TunnelStats = {
  lastHandshakeSec: number | null;
  rxBytes: number | null;
  txBytes: number | null;
};

const FALLBACK_BINARY = `${homedir()}/.local/bin/wireproxy`;

export function resolveWireproxyBinary(): string | null {
  return Bun.which("wireproxy") ?? (existsSync(FALLBACK_BINARY) ? FALLBACK_BINARY : null);
}

export function requireWireproxyBinary(): string {
  const binary = resolveWireproxyBinary();
  if (!binary) throw new Error("wireproxy binary not found; install it to ~/.local/bin");
  return binary;
}

export function generateConfig(
  serverConf: string,
  options: Omit<RunOptions, "serverFile">,
): string {
  const base = serverConf.trimEnd();
  const peerIndex = base.search(/^\[Peer\]/m);
  const keepalive = "CheckAlive = 1.1.1.1\nCheckAliveInterval = 5";

  let head: string;
  let tail: string;
  if (peerIndex === -1) {
    head = `${base}\n${keepalive}`;
    tail = "";
  } else {
    head = `${base.slice(0, peerIndex).trimEnd()}\n${keepalive}\n`;
    tail = `\n${base.slice(peerIndex).trimEnd()}`;
  }

  const patterns = options.mode === "split" ? enabledPatterns(options.domains) : [];
  // With no TunnelDomains key wireproxy tunnels everything, so an empty rule
  // list in split mode needs a pattern that matches no host.
  const tunnelLines =
    options.mode === "split" && patterns.length === 0
      ? ["TunnelDomains = ^$"]
      : patterns.map((pattern) => `TunnelDomains = ${pattern}`);
  const socks = [
    "[Socks5]",
    `BindAddress = 127.0.0.1:${options.socksPort}`,
    ...tunnelLines,
    "LogDomains = true",
  ].join("\n");

  return `${head}${tail}\n\n${socks}\n`;
}

export async function daemonStatus(): Promise<DaemonStatus> {
  let pid: number;
  try {
    pid = Number.parseInt((await Bun.file(pidFile()).text()).trim(), 10);
  } catch {
    return { state: "stopped" };
  }
  if (!Number.isInteger(pid) || pid <= 0) return { state: "stopped" };
  return isAlive(pid) ? { state: "running", pid } : { state: "stale", pid };
}

export async function writeGeneratedConfig(options: RunOptions): Promise<void> {
  const serverConf = await readServerConfig(options.serverFile);
  await ensureDirs();
  await writeFile(wireproxyConfigFile(), generateConfig(serverConf, options), {
    mode: 0o600,
  });
}

export async function startDaemon(options: RunOptions): Promise<number> {
  const binary = requireWireproxyBinary();

  const status = await daemonStatus();
  if (status.state === "running")
    throw new Error(`daemon already running (pid ${status.pid})`);

  await writeGeneratedConfig(options);

  const logFd = openSync(logFile(), "a");
  const child = Bun.spawn(
    [binary, "-c", wireproxyConfigFile(), "-i", `127.0.0.1:${options.healthPort}`],
    { stdin: "ignore", stdout: logFd, stderr: logFd, detached: true },
  );
  closeSync(logFd);
  child.unref();

  await writeFile(pidFile(), String(child.pid));
  return child.pid;
}

export async function stopDaemon(): Promise<void> {
  const status = await daemonStatus();
  if (status.state === "stopped") return;
  if (status.state === "stale") {
    await rm(pidFile(), { force: true });
    return;
  }

  try {
    process.kill(status.pid, "SIGTERM");
  } catch {
    // already gone
  }

  const deadline = Date.now() + 5000;
  while (Date.now() < deadline && isAlive(status.pid)) {
    await Bun.sleep(100);
  }
  if (isAlive(status.pid)) process.kill(status.pid, "SIGKILL");

  await rm(pidFile(), { force: true });
}

export async function restartDaemon(options: RunOptions): Promise<number> {
  await stopDaemon();
  return startDaemon(options);
}

export async function daemonHealth(port: number): Promise<Health> {
  const base = `http://127.0.0.1:${port}`;
  const [ready, metrics] = await Promise.all([
    fetchText(`${base}/readyz`),
    fetchText(`${base}/metrics`),
  ]);
  if (!ready) return { reachable: false, ready: null, metrics: null };
  return { reachable: true, ready: ready.status === 200, metrics: metrics?.text ?? null };
}

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

async function fetchText(url: string): Promise<{ status: number; text: string } | null> {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(1500) });
    return { status: response.status, text: await response.text() };
  } catch {
    return null;
  }
}

async function fetchStatus(url: string): Promise<number | null> {
  try {
    const response = await fetch(url, { signal: AbortSignal.timeout(1500) });
    return response.status;
  } catch {
    return null;
  }
}

export async function waitForReachable(
  port: number,
  attempts = 30,
  delay = 500,
): Promise<boolean> {
  for (let attempt = 0; attempt < attempts; attempt++) {
    if ((await fetchStatus(`http://127.0.0.1:${port}/readyz`)) !== null) return true;
    await Bun.sleep(delay);
  }
  return false;
}

export async function readLog(windowLines: number, keepLines: number): Promise<string[]> {
  try {
    const file = Bun.file(logFile());
    if (file.size === 0) return [];

    const lines = (await file.text()).split("\n").filter((line) => line.length > 0);
    const kept = lines.slice(-keepLines);
    if (kept.length < lines.length) await Bun.write(logFile(), `${kept.join("\n")}\n`);
    return kept.slice(-windowLines);
  } catch {
    return [];
  }
}

const METRIC_PATTERNS = {
  lastHandshakeSec: /^last_handshake_time_sec=(\d+)$/m,
  rxBytes: /^rx_bytes=(\d+)$/m,
  txBytes: /^tx_bytes=(\d+)$/m,
};

export function parseMetrics(text: string): TunnelStats {
  const read = (pattern: RegExp): number | null => {
    const raw = pattern.exec(text)?.[1];
    if (!raw) return null;
    const parsed = Number.parseInt(raw, 10);
    return Number.isNaN(parsed) ? null : parsed;
  };
  return {
    lastHandshakeSec: read(METRIC_PATTERNS.lastHandshakeSec),
    rxBytes: read(METRIC_PATTERNS.rxBytes),
    txBytes: read(METRIC_PATTERNS.txBytes),
  };
}
