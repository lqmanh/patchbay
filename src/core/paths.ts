import { mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";

function xdgDir(env: string, fallback: string): string {
  const value = process.env[env];
  return value && value.length > 0 ? value : join(homedir(), fallback);
}

export function configDir(): string {
  return join(xdgDir("XDG_CONFIG_HOME", ".config"), "patchbay");
}

export function serversDir(): string {
  return join(configDir(), "servers");
}

export function cacheDir(): string {
  return join(xdgDir("XDG_CACHE_HOME", ".cache"), "patchbay");
}

export function stateDir(): string {
  return join(xdgDir("XDG_STATE_HOME", ".local/state"), "patchbay");
}

export function runtimeDir(): string {
  const value = process.env.XDG_RUNTIME_DIR;
  return value && value.length > 0 ? join(value, "patchbay") : stateDir();
}

export function configFile(): string {
  return join(configDir(), "config.json");
}

export function daemonConfigFile(): string {
  return join(cacheDir(), "daemon.json");
}

export function pidFile(): string {
  return join(runtimeDir(), "daemon.pid");
}

export function logFile(): string {
  return join(stateDir(), "daemon.log");
}

export function proxyStateFile(): string {
  return join(stateDir(), "proxy-state.json");
}

export async function ensureDirs(): Promise<void> {
  const dirs = [configDir(), serversDir(), cacheDir(), stateDir(), runtimeDir()];
  await Promise.all(dirs.map((dir) => mkdir(dir, { recursive: true })));
}
