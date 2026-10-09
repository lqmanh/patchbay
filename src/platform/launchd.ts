import { existsSync } from "node:fs";
import { mkdir, rm, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { runChecked } from "./exec";

const DAEMON_LABEL = "dev.patchbay.daemon";
const INTERCEPT_LABEL = "dev.patchbay.intercept";

export type LaunchdStatus = {
  installed: boolean;
  loaded: boolean;
  running: boolean;
  pid: number | null;
};

export function launchAgentPath(): string {
  return agentPath(DAEMON_LABEL);
}

export function interceptAgentPath(): string {
  return agentPath(INTERCEPT_LABEL);
}

export function isLaunchAgentInstalled(): boolean {
  return existsSync(launchAgentPath());
}

export function isInterceptAgentInstalled(): boolean {
  return existsSync(interceptAgentPath());
}

export function buildPlist(options: {
  label: string;
  args: string[];
  logPath: string;
  keepAlive: boolean;
}): string {
  const argsXml = options.args
    .map((arg) => `    <string>${escapeXml(arg)}</string>`)
    .join("\n");
  const keepAliveXml = options.keepAlive ? "  <key>KeepAlive</key>\n  <true/>\n" : "";

  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${options.label}</string>
  <key>ProgramArguments</key>
  <array>
${argsXml}
  </array>
  <key>RunAtLoad</key>
  <true/>
${keepAliveXml}  <key>StandardOutPath</key>
  <string>${escapeXml(options.logPath)}</string>
  <key>StandardErrorPath</key>
  <string>${escapeXml(options.logPath)}</string>
</dict>
</plist>
`;
}

export function buildLaunchAgentPlist(options: {
  binary: string;
  configPath: string;
  logPath: string;
}): string {
  return buildPlist({
    label: DAEMON_LABEL,
    args: [options.binary, "run", "-c", options.configPath, "--disable-color"],
    logPath: options.logPath,
    keepAlive: true,
  });
}

export function buildInterceptAgentPlist(options: {
  command: string[];
  logPath: string;
}): string {
  return buildPlist({
    label: INTERCEPT_LABEL,
    args: options.command,
    logPath: options.logPath,
    keepAlive: false,
  });
}

export async function launchdStatus(): Promise<LaunchdStatus> {
  const missing = { installed: false, loaded: false, running: false, pid: null };
  if (!isLaunchAgentInstalled()) return missing;

  const result = Bun.spawnSync(["launchctl", "list", DAEMON_LABEL]);
  if (result.exitCode !== 0) return { ...missing, installed: true };

  const match = /"PID"\s*=\s*(\d+)/.exec(result.stdout.toString());
  const pid = match?.[1] ? Number.parseInt(match[1], 10) : null;
  return { installed: true, loaded: true, running: pid !== null, pid };
}

export async function installLaunchAgent(
  binary: string,
  configPath: string,
  logPath: string,
): Promise<void> {
  await installAgent(
    DAEMON_LABEL,
    buildLaunchAgentPlist({ binary, configPath, logPath }),
  );
}

export async function installInterceptAgent(
  command: string[],
  logPath: string,
): Promise<void> {
  await installAgent(INTERCEPT_LABEL, buildInterceptAgentPlist({ command, logPath }));
}

export async function uninstallLaunchAgent(): Promise<void> {
  await uninstallAgent(DAEMON_LABEL);
}

export async function uninstallInterceptAgent(): Promise<void> {
  await uninstallAgent(INTERCEPT_LABEL);
}

export async function restartLaunchAgent(): Promise<void> {
  runChecked("launchctl", ["kickstart", "-k", `${domain()}/${DAEMON_LABEL}`]);
}

export async function ensureLaunchAgentRunning(): Promise<void> {
  const status = await launchdStatus();
  if (status.running) return;
  if (status.loaded) await restartLaunchAgent();
  else await bootstrap(DAEMON_LABEL);
}

async function installAgent(label: string, plist: string): Promise<void> {
  await mkdir(join(homedir(), "Library", "LaunchAgents"), { recursive: true });
  await writeFile(agentPath(label), plist);
  await bootstrap(label);
}

async function uninstallAgent(label: string): Promise<void> {
  Bun.spawnSync(["launchctl", "bootout", domain(), agentPath(label)]);
  await rm(agentPath(label), { force: true });
}

async function bootstrap(label: string): Promise<void> {
  Bun.spawnSync(["launchctl", "bootout", domain(), agentPath(label)]);
  runChecked("launchctl", ["bootstrap", domain(), agentPath(label)]);
}

function agentPath(label: string): string {
  return join(homedir(), "Library", "LaunchAgents", `${label}.plist`);
}

function domain(): string {
  const uid = process.getuid?.();
  if (uid === undefined) throw new Error("launchctl requires a logged-in user session");
  return `gui/${uid}`;
}

function escapeXml(value: string): string {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}
