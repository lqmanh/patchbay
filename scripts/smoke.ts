import { defaultConfig } from "../src/core/config";
import { listServers, resolveActiveServer } from "../src/core/servers";
import {
  daemonStatus,
  readLog,
  resolveSingboxBinary,
  restartDaemon,
  startDaemon,
  stopDaemon,
  waitForReachable,
} from "../src/core/singbox";
import { activeNetworkService, getSocksProxy } from "../src/platform/macos";

const failures: string[] = [];

function check(label: string, ok: boolean): void {
  console.log(ok ? `ok: ${label}` : `FAIL: ${label}`);
  if (!ok) failures.push(label);
}

const binary = resolveSingboxBinary();
console.log("sing-box:", binary ?? "NOT FOUND");

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
    `missing sing-box binary or server config (requested: ${requested ?? "first"})`,
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
    mode: "split",
    domains: [
      { id: "smoke1", value: "reddit.com", kind: "domain", enabled: true },
      { id: "smoke2", value: "medium.com", kind: "domain", enabled: true },
    ],
  });
  console.log("started daemon, pid", pid);
  check("daemon reports running", (await daemonStatus()).state === "running");

  const reachable = await waitForReachable(defaultConfig.socksPort, 50, 200);
  check("socks port reachable", reachable);

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
  check("reddit reachable via proxy", /^[1-9]\d\d$/.test(redditCode));

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

  await Bun.sleep(500);
  const log = (await readLog(400, 800)).join("\n");
  check("tunneled connections logged", log.includes("endpoint/wireguard"));
  check("direct connections logged", log.includes("outbound/direct"));

  console.log("--- full mode ---");
  const fullPid = await restartDaemon({
    serverFile: server.file,
    socksPort: defaultConfig.socksPort,
    mode: "full",
    domains: [],
  });
  console.log("started full-mode daemon, pid", fullPid);
  check(
    "full-mode socks port reachable",
    await waitForReachable(defaultConfig.socksPort, 50, 200),
  );

  const fullExitProxy = Bun.spawnSync([
    "curl",
    "-s",
    "--max-time",
    "30",
    "--proxy",
    proxy,
    "https://ipinfo.io/ip",
  ]);
  const fullExitIp = fullExitProxy.stdout.toString().trim();
  console.log("exit via proxy (full mode, expect VPN):", fullExitIp);
  check(
    "full mode tunnels unmatched traffic",
    fullExitIp.length > 0 && fullExitIp !== directIp,
  );

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
