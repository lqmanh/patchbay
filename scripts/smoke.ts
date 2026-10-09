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

const failures: string[] = [];

function check(label: string, ok: boolean): void {
  console.log(ok ? `ok: ${label}` : `FAIL: ${label}`);
  if (!ok) failures.push(label);
}

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
try {
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
  check("daemon reports running", (await daemonStatus()).state === "running");

  const reachable = await waitForReachable(defaultConfig.healthPort, 50, 200);
  const health = await daemonHealth(defaultConfig.healthPort);
  console.log("health:", JSON.stringify(health.metrics?.slice(0, 200)));
  check("health endpoint reachable", reachable && health.reachable);

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
  const redditCode = curl.stdout.toString();
  console.log("reddit via proxy:", redditCode || `curl exited ${curl.exitCode}`);
  check("reddit 2xx via proxy", /^2\d\d$/.test(redditCode));

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
  const exitIp = exitProxy.stdout.toString().trim();
  const directIp = exitDirect.stdout.toString().trim();
  console.log("exit via proxy (unmatched, expect direct):", exitIp);
  console.log("exit direct:", directIp);
  check("unmatched traffic exits direct", exitIp.length > 0 && exitIp === directIp);

  console.log("status:", JSON.stringify(await daemonStatus()));
} finally {
  if (process.env.SMOKE_KEEP) {
    console.log("SMOKE_KEEP set, leaving daemon running");
  } else {
    await stopDaemon();
    console.log("stopped:", JSON.stringify(await daemonStatus()));
  }
}

if (failures.length > 0) {
  console.error(`smoke failed: ${failures.join(", ")}`);
  process.exitCode = 1;
} else {
  console.log("smoke passed");
}
