import { describe, expect, test } from "bun:test";
import { defaultConfig } from "./config";
import { buildRunOptions } from "./options";

const servers = [
  { file: "a.conf", name: "a" },
  { file: "b.conf", name: "b" },
];

describe("buildRunOptions", () => {
  test("prefers the active server", () => {
    const options = buildRunOptions(
      { ...defaultConfig, activeServer: "b.conf" },
      servers,
    );
    expect(options?.serverFile).toBe("b.conf");
  });

  test("falls back to the first server", () => {
    expect(buildRunOptions(defaultConfig, servers)?.serverFile).toBe("a.conf");
  });

  test("returns null without any server", () => {
    expect(buildRunOptions(defaultConfig, [])).toBeNull();
  });
});
