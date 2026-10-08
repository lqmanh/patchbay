import { afterEach, describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  captureStateBeforeEnable,
  clearProxyState,
  loadProxyState,
  saveProxyState,
} from "./interception";

describe("captureStateBeforeEnable", () => {
  test("does not overwrite a state we already own", () => {
    expect(
      captureStateBeforeEnable({ enabled: true, host: "127.0.0.1", port: 25344 }, 25344),
    ).toBeNull();
  });

  test("captures disabled and foreign proxy states", () => {
    const disabled = { enabled: false, host: "", port: 0 };
    expect(captureStateBeforeEnable(disabled, 25344)).toEqual(disabled);

    const foreign = { enabled: true, host: "10.0.0.1", port: 1080 };
    expect(captureStateBeforeEnable(foreign, 25344)).toEqual(foreign);
  });
});

describe("proxy state persistence", () => {
  const original = process.env.XDG_STATE_HOME;
  let dir = "";

  afterEach(async () => {
    if (original === undefined) delete process.env.XDG_STATE_HOME;
    else process.env.XDG_STATE_HOME = original;
    if (dir) await rm(dir, { recursive: true, force: true });
    dir = "";
  });

  test("round-trips saved state", async () => {
    dir = await mkdtemp(join(tmpdir(), "patchbay-state-"));
    process.env.XDG_STATE_HOME = dir;

    await saveProxyState({
      service: "Wi-Fi",
      enabled: true,
      host: "10.0.0.1",
      port: 1080,
    });
    expect(await loadProxyState()).toEqual({
      service: "Wi-Fi",
      enabled: true,
      host: "10.0.0.1",
      port: 1080,
    });

    await clearProxyState();
    expect(await loadProxyState()).toBeNull();
  });
});
