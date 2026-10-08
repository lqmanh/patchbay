import { join } from "node:path";
import { serversDir } from "./paths";

export type Server = {
  file: string;
  name: string;
};

export async function listServers(): Promise<Server[]> {
  const found: Server[] = [];
  try {
    const glob = new Bun.Glob("*.conf");
    for await (const file of glob.scan({ cwd: serversDir(), onlyFiles: true })) {
      found.push({ file, name: file.replace(/\.conf$/, "") });
    }
  } catch {
    // servers directory does not exist yet
  }
  return found.toSorted((left, right) => left.name.localeCompare(right.name));
}

export function serverPath(file: string): string {
  return join(serversDir(), file);
}

export function readServerConfig(file: string): Promise<string> {
  return Bun.file(serverPath(file)).text();
}

export function resolveActiveServer(active: string | null): Promise<Server | null> {
  return listServers().then((servers) => pickServer(servers, active));
}

export function pickServer(servers: Server[], active: string | null): Server | null {
  return servers.find((server) => server.file === active) ?? servers[0] ?? null;
}
