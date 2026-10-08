import { defaultConfig } from "../src/core/config";
import { listServers, resolveActiveServer } from "../src/core/servers";
import {
  daemonHealth,
  daemonStatus,
  resolveWireproxyBinary,
  startDaemon,
  stopDaemon,
  waitForReachable,
} from "../src/core/wireproxy";
import { activeNetworkService, getSocksProxy } from "../src/platform/macos";

const binary = resolveWireproxyBinary();
console.log("wireproxy:", binary ?? "NOT FOUND");

const servers = await listServers();
console.log("servers:", servers.map((server) => server.name).join(", ") || "none");

const requested = process.argv[2];
const server = requested
  ? (servers.find((candidate) => candidate.file === requested) ??
    servers.find((candidate) => candidate.name === requested) ??
    null)
  : await resolveActiveServer(null);
if (!binary || !server) {
  console.error(
    `missing wireproxy binary or server config (requested: ${requested ?? "first"})`,
  );
  process.exit(1);
}

const service = await activeNetworkService();
if (service) {
  console.log("active service:", service, JSON.stringify(await getSocksProxy(service)));
}

await stopDaemon();
const proxy = `socks5h://127.0.0.1:${defaultConfig.socksPort}`;
const pid = await startDaemon({
  serverFile: server.file,
  socksPort: defaultConfig.socksPort,
  healthPort: defaultConfig.healthPort,
  mode: "split",
  domains: [
    { id: "smoke1", value: "reddit.com", kind: "domain", enabled: true },
    { id: "smoke2", value: "medium.com", kind: "domain", enabled: true },
  ],
});
console.log("started daemon, pid", pid);

await waitForReachable(defaultConfig.healthPort, 50, 200);
const health = await daemonHealth(defaultConfig.healthPort);
console.log("health:", JSON.stringify(health.metrics?.slice(0, 200)));

const curl = Bun.spawnSync([
  "curl",
  "-s",
  "-o",
  "/dev/null",
  "-w",
  "%{http_code}",
  "--proxy",
  proxy,
  "--max-time",
  "20",
  "https://www.reddit.com",
]);
console.log(
  "reddit via proxy:",
  curl.stdout.toString() || `curl exited ${curl.exitCode}`,
);

const exitProxy = Bun.spawnSync([
  "curl",
  "-s",
  "--max-time",
  "10",
  "--proxy",
  proxy,
  "https://ipinfo.io/ip",
]);
const exitDirect = Bun.spawnSync([
  "curl",
  "-s",
  "--max-time",
  "10",
  "https://ipinfo.io/ip",
]);
console.log(
  "exit via proxy (unmatched, expect direct):",
  exitProxy.stdout.toString().trim(),
);
console.log("exit direct:", exitDirect.stdout.toString().trim());

console.log("status:", JSON.stringify(await daemonStatus()));

if (process.env.SMOKE_KEEP) {
  console.log("SMOKE_KEEP set, leaving daemon running");
} else {
  await stopDaemon();
  console.log("stopped:", JSON.stringify(await daemonStatus()));
}
