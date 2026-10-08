import { run, runChecked } from "./exec";

export type SocksState = {
  enabled: boolean;
  host: string;
  port: number;
};

export function parseActiveInterface(output: string): string | null {
  return /^[ \t]*interface:[ \t]*(\S+)[ \t]*$/m.exec(output)?.[1] ?? null;
}

export function parseServiceForDevice(output: string, device: string): string | null {
  const entries =
    /^\((?:\d+|\*)\)[ \t]+(.+)\n\(Hardware Port: [^,]+,[ \t]*Device:[ \t]*(\S*)\)/gm;
  for (const match of output.matchAll(entries)) {
    if (match[2] === device) return match[1]?.trim() ?? null;
  }
  return null;
}

export function parseSocksState(output: string): SocksState {
  return {
    enabled: /^Enabled:[ \t]*(Yes|No)[ \t]*$/m.exec(output)?.[1] === "Yes",
    host: /^Server:[ \t]*(.*?)[ \t]*$/m.exec(output)?.[1] ?? "",
    port: Number.parseInt(/^Port:[ \t]*(\d+)[ \t]*$/m.exec(output)?.[1] ?? "0", 10),
  };
}

export async function activeNetworkService(): Promise<string | null> {
  const device = parseActiveInterface(run("route", ["-n", "get", "default"]));
  if (!device) return null;
  return parseServiceForDevice(run("networksetup", ["-listnetworkserviceorder"]), device);
}

export async function getSocksProxy(service: string): Promise<SocksState> {
  return parseSocksState(run("networksetup", ["-getsocksfirewallproxy", service]));
}

export async function setSocksProxy(
  service: string,
  host: string,
  port: number,
): Promise<void> {
  runChecked("networksetup", ["-setsocksfirewallproxy", service, host, String(port)]);
  runChecked("networksetup", ["-setsocksfirewallproxystate", service, "on"]);
}

export async function disableSocksProxy(service: string): Promise<void> {
  runChecked("networksetup", ["-setsocksfirewallproxystate", service, "off"]);
}
