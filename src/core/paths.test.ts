import { afterAll, beforeEach, describe, expect, test } from "bun:test";
import { homedir } from "node:os";
import { join } from "node:path";
import { cacheDir, configDir, runtimeDir, stateDir } from "./paths";

const KEYS = [
  "XDG_CONFIG_HOME",
  "XDG_CACHE_HOME",
  "XDG_STATE_HOME",
  "XDG_RUNTIME_DIR",
] as const;

const original = new Map(KEYS.map((key) => [key, process.env[key]]));

beforeEach(() => {
  for (const key of KEYS) delete process.env[key];
});

afterAll(() => {
  for (const key of KEYS) {
    const value = original.get(key);
    if (value === undefined) delete process.env[key];
    else process.env[key] = value;
  }
});

describe("xdg paths", () => {
  test("uses XDG environment overrides", () => {
    process.env.XDG_CONFIG_HOME = "/tmp/xdg-config";
    process.env.XDG_CACHE_HOME = "/tmp/xdg-cache";
    process.env.XDG_STATE_HOME = "/tmp/xdg-state";
    process.env.XDG_RUNTIME_DIR = "/tmp/xdg-runtime";

    expect(configDir()).toBe("/tmp/xdg-config/patchbay");
    expect(cacheDir()).toBe("/tmp/xdg-cache/patchbay");
    expect(stateDir()).toBe("/tmp/xdg-state/patchbay");
    expect(runtimeDir()).toBe("/tmp/xdg-runtime/patchbay");
  });

  test("falls back to conventional locations", () => {
    expect(configDir()).toBe(join(homedir(), ".config", "patchbay"));
    expect(cacheDir()).toBe(join(homedir(), ".cache", "patchbay"));
    expect(stateDir()).toBe(join(homedir(), ".local", "state", "patchbay"));
    expect(runtimeDir()).toBe(join(homedir(), ".local", "state", "patchbay"));
  });
});
