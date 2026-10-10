import type { DomainRule } from "./domains";
import { writeJson } from "./json";
import { configFile } from "./paths";

export type Mode = "split" | "full";

export type AppConfig = {
  socksPort: number;
  mode: Mode;
  activeServer: string | null;
  domains: DomainRule[];
  theme: string;
};

export const defaultConfig: AppConfig = {
  socksPort: 25344,
  mode: "split",
  activeServer: null,
  domains: [],
  theme: "system",
};

export function freshConfig(): AppConfig {
  return { ...defaultConfig, domains: [] };
}

export function isPort(value: unknown): value is number {
  return (
    typeof value === "number" && Number.isInteger(value) && value > 0 && value < 65536
  );
}

function isRule(value: unknown): value is DomainRule {
  if (typeof value !== "object" || value === null) return false;
  const rule = value as Record<string, unknown>;
  return (
    typeof rule.id === "string" &&
    typeof rule.value === "string" &&
    rule.value.length > 0 &&
    (rule.kind === "domain" || rule.kind === "regex") &&
    typeof rule.enabled === "boolean"
  );
}

export function sanitizeConfig(input: unknown): AppConfig {
  if (typeof input !== "object" || input === null) return freshConfig();
  const raw = input as Record<string, unknown>;
  return {
    socksPort: isPort(raw.socksPort) ? raw.socksPort : defaultConfig.socksPort,
    mode: raw.mode === "full" ? "full" : "split",
    activeServer: typeof raw.activeServer === "string" ? raw.activeServer : null,
    domains: Array.isArray(raw.domains) ? raw.domains.filter(isRule) : [],
    theme:
      typeof raw.theme === "string" && raw.theme.length > 0
        ? raw.theme
        : defaultConfig.theme,
  };
}

export async function loadConfig(): Promise<AppConfig> {
  try {
    return sanitizeConfig(await Bun.file(configFile()).json());
  } catch {
    return freshConfig();
  }
}

export async function saveConfig(config: AppConfig): Promise<void> {
  await writeJson(configFile(), config);
}
