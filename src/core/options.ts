import type { AppConfig } from "./config";
import { serversDir } from "./paths";
import { pickServer, type Server } from "./servers";
import type { RunOptions } from "./wireproxy";

export function buildRunOptions(config: AppConfig, servers: Server[]): RunOptions | null {
  const server = pickServer(servers, config.activeServer);
  if (!server) return null;
  return {
    serverFile: server.file,
    socksPort: config.socksPort,
    healthPort: config.healthPort,
    mode: config.mode,
    domains: config.domains,
  };
}

export function requireRunOptions(config: AppConfig, servers: Server[]): RunOptions {
  const options = buildRunOptions(config, servers);
  if (!options) throw new Error(`no server config in ${serversDir()}`);
  return options;
}
