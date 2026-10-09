import { closeSync, existsSync, openSync } from "node:fs";
import { rm, writeFile } from "node:fs/promises";
import { createConnection } from "node:net";
import type { Mode } from "./config";
import type { DomainRule } from "./domains";
import { daemonConfigFile, ensureDirs, logFile, pidFile } from "./paths";
import { readServerConfig } from "./servers";

export type RunOptions = {
  serverFile: string;
  socksPort: number;
  mode: Mode;
  domains: readonly DomainRule[];
};

export type DaemonStatus =
  | { state: "stopped" }
  | { state: "running"; pid: number }
  | { state: "stale"; pid: number };

const TUNNEL_TAG = "tunnel";
const DIRECT_TAG = "direct";
const FALLBACK_BINARIES = ["/opt/homebrew/bin/sing-box", "/usr/local/bin/sing-box"];

export function resolveSingboxBinary(): string | null {
  return (
    Bun.which("sing-box") ?? FALLBACK_BINARIES.find((path) => existsSync(path)) ?? null
  );
}

export function requireSingboxBinary(): string {
  const binary = resolveSingboxBinary();
  if (!binary)
    throw new Error("sing-box binary not found; install it with `brew install sing-box`");
  return binary;
}

type WireguardPeer = {
  publicKey: string;
  presharedKey: string | null;
  host: string;
  port: number;
  allowedIps: string[];
  keepalive: number | null;
};

type WireguardConfig = {
  addresses: string[];
  privateKey: string;
  mtu: number | null;
  dns: string[];
  peers: WireguardPeer[];
};

function parseWireguardConfig(text: string): WireguardConfig {
  const config: WireguardConfig = {
    addresses: [],
    privateKey: "",
    mtu: null,
    dns: [],
    peers: [],
  };
  let section: "interface" | "peer" | null = null;
  let peer: WireguardPeer | null = null;

  const flushPeer = () => {
    if (peer && peer.publicKey && peer.host && peer.port > 0) config.peers.push(peer);
    peer = null;
  };

  for (const rawLine of text.split("\n")) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) continue;

    const header = /^\[(\w+)\]$/.exec(line);
    if (header) {
      flushPeer();
      section = header[1]?.toLowerCase() === "peer" ? "peer" : "interface";
      if (section === "peer") {
        peer = {
          publicKey: "",
          presharedKey: null,
          host: "",
          port: 0,
          allowedIps: [],
          keepalive: null,
        };
      }
      continue;
    }

    const separator = line.indexOf("=");
    if (separator === -1) continue;
    const key = line.slice(0, separator).trim().toLowerCase();
    const value = line.slice(separator + 1).trim();

    if (section === "interface") {
      switch (key) {
        case "address":
          config.addresses.push(...splitList(value));
          break;
        case "privatekey":
          config.privateKey = value;
          break;
        case "mtu":
          config.mtu = parsePositiveInt(value);
          break;
        case "dns":
          config.dns.push(...splitList(value));
          break;
      }
    } else if (section === "peer" && peer) {
      switch (key) {
        case "publickey":
          peer.publicKey = value;
          break;
        case "presharedkey":
          peer.presharedKey = value;
          break;
        case "allowedips":
          peer.allowedIps = splitList(value);
          break;
        case "persistentkeepalive":
          peer.keepalive = parsePositiveInt(value);
          break;
        case "endpoint": {
          const parsed = parseEndpoint(value);
          if (parsed) {
            peer.host = parsed.host;
            peer.port = parsed.port;
          }
          break;
        }
      }
    }
  }
  flushPeer();

  if (config.addresses.length === 0 || !config.privateKey || config.peers.length === 0) {
    throw new Error("invalid WireGuard config: missing Address, PrivateKey, or Peer");
  }
  return config;
}

function splitList(value: string): string[] {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

function parsePositiveInt(value: string): number | null {
  const parsed = Number.parseInt(value, 10);
  return Number.isNaN(parsed) || parsed <= 0 ? null : parsed;
}

function parseEndpoint(value: string): { host: string; port: number } | null {
  const match = /^\[(.+)\]:(\d+)$/.exec(value) ?? /^(.*):(\d+)$/.exec(value);
  if (!match?.[1] || !match[2]) return null;
  const port = Number.parseInt(match[2], 10);
  return port > 0 ? { host: match[1], port } : null;
}

function matchFields(rules: readonly DomainRule[]): Record<string, string[]> {
  const fields: Record<string, string[]> = {};
  for (const rule of rules) {
    if (!rule.enabled) continue;
    if (rule.kind === "domain") {
      (fields.domain ??= []).push(rule.value);
      (fields.domain_suffix ??= []).push(`.${rule.value}`);
    } else {
      (fields.domain_regex ??= []).push(rule.value);
    }
  }
  return fields;
}

export function generateConfig(
  serverConf: string,
  options: Omit<RunOptions, "serverFile">,
): string {
  const wireguard = parseWireguardConfig(serverConf);
  const split = options.mode === "split";
  const fields = matchFields(options.domains);
  const hasRules = Object.keys(fields).length > 0;

  const endpoint: Record<string, unknown> = {
    type: "wireguard",
    tag: TUNNEL_TAG,
    // Resolve the WireGuard server address with the system resolver: using
    // dns-tunnel here would deadlock the tunnel it is needed to establish.
    domain_resolver: "dns-direct",
    address: wireguard.addresses,
    private_key: wireguard.privateKey,
    peers: wireguard.peers.map((peer) => ({
      address: peer.host,
      port: peer.port,
      public_key: peer.publicKey,
      ...(peer.presharedKey ? { pre_shared_key: peer.presharedKey } : {}),
      allowed_ips: peer.allowedIps,
      ...(peer.keepalive ? { persistent_keepalive_interval: peer.keepalive } : {}),
    })),
  };
  if (wireguard.mtu) endpoint.mtu = wireguard.mtu;

  const config = {
    log: { level: "info", timestamp: true },
    dns: {
      servers: [
        { type: "local", tag: "dns-direct" },
        {
          type: "udp",
          tag: "dns-tunnel",
          server: wireguard.dns[0] ?? "1.1.1.1",
          detour: TUNNEL_TAG,
        },
      ],
      rules: split && hasRules ? [{ ...fields, server: "dns-tunnel" }] : [],
      final: split ? "dns-direct" : "dns-tunnel",
    },
    inbounds: [
      {
        type: "socks",
        tag: "socks-in",
        listen: "127.0.0.1",
        listen_port: options.socksPort,
      },
    ],
    endpoints: [endpoint],
    outbounds: [{ type: "direct", tag: DIRECT_TAG }],
    route: {
      rules:
        split && hasRules ? [{ ...fields, action: "route", outbound: TUNNEL_TAG }] : [],
      final: split ? DIRECT_TAG : TUNNEL_TAG,
      default_domain_resolver: "dns-direct",
    },
  };

  return `// auto-generated by patchbay; edits will be overwritten\n${JSON.stringify(config, null, 2)}\n`;
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
  await writeFile(daemonConfigFile(), generateConfig(serverConf, options), {
    mode: 0o600,
  });
}

export async function startDaemon(options: RunOptions): Promise<number> {
  const binary = requireSingboxBinary();

  const status = await daemonStatus();
  if (status.state === "running")
    throw new Error(`daemon already running (pid ${status.pid})`);

  await writeGeneratedConfig(options);

  const logFd = openSync(logFile(), "a");
  const child = Bun.spawn([binary, "run", "-c", daemonConfigFile(), "--disable-color"], {
    stdin: "ignore",
    stdout: logFd,
    stderr: logFd,
    detached: true,
  });
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

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return (error as NodeJS.ErrnoException).code === "EPERM";
  }
}

export async function isReachable(port: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    const finish = (result: boolean) => {
      socket.removeAllListeners();
      socket.destroy();
      resolve(result);
    };
    socket.setTimeout(1500, () => finish(false));
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}

export async function waitForReachable(
  port: number,
  attempts = 30,
  delay = 500,
): Promise<boolean> {
  for (let attempt = 0; attempt < attempts; attempt++) {
    if (await isReachable(port)) return true;
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
