export type RuleKind = "domain" | "regex";

export type DomainRule = {
  id: string;
  value: string;
  kind: RuleKind;
  enabled: boolean;
};

const HOSTNAME = /^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/;
const RE2_INCOMPATIBLE = [/\(\?[=!<]/, /\\[1-9]/];

export function normalizeDomain(input: string): string | null {
  let value = input.trim().toLowerCase();
  if (!value) return null;
  value = value.replace(/^[a-z][a-z0-9+.-]*:\/\//, "");
  value = value.split(/[/?#]/)[0] ?? "";
  value = value.replace(/:\d+$/, "");
  value = value.replace(/^www\./, "");
  value = value.replace(/\.$/, "");
  return HOSTNAME.test(value) ? value : null;
}

export function domainPattern(domain: string): string {
  const escaped = domain.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return `^(.*\\.)?${escaped}$`;
}

export function regexError(pattern: string): string | null {
  if (!pattern.trim()) return "pattern is empty";
  try {
    new RegExp(pattern);
  } catch {
    return "invalid regular expression";
  }
  if (RE2_INCOMPATIBLE.some((unsupported) => unsupported.test(pattern))) {
    return "lookarounds and backreferences are not supported by the daemon";
  }
  return null;
}

export function rulePattern(rule: DomainRule): string {
  return rule.kind === "domain" ? domainPattern(rule.value) : rule.value;
}

export function enabledPatterns(rules: readonly DomainRule[]): string[] {
  return rules.filter((rule) => rule.enabled).map(rulePattern);
}

export function createRule(input: string, kind: RuleKind = "domain"): DomainRule | null {
  const value = kind === "domain" ? normalizeDomain(input) : input.trim();
  if (value === null || value === "") return null;
  if (kind === "regex" && regexError(value) !== null) return null;
  return { id: crypto.randomUUID().slice(0, 8), value, kind, enabled: true };
}
