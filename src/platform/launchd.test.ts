import { describe, expect, test } from "bun:test";
import { homedir } from "node:os";
import { join } from "node:path";
import {
  buildInterceptAgentPlist,
  buildLaunchAgentPlist,
  interceptAgentPath,
  launchAgentPath,
} from "./launchd";

describe("launch agent", () => {
  test("lives in the user's LaunchAgents directory", () => {
    expect(launchAgentPath()).toBe(
      join(homedir(), "Library", "LaunchAgents", "dev.patchbay.daemon.plist"),
    );
  });

  test("builds a keep-alive plist with the sing-box arguments", () => {
    const plist = buildLaunchAgentPlist({
      binary: "/opt/homebrew/bin/sing-box",
      configPath: "/Users/me/.cache/patchbay/daemon.json",
      logPath: "/Users/me/.local/state/patchbay/daemon.log",
    });

    expect(plist).toContain("<string>dev.patchbay.daemon</string>");
    expect(plist).toContain("<key>KeepAlive</key>");
    expect(plist).toContain("<key>RunAtLoad</key>");
    expect(plist).toContain("<string>/opt/homebrew/bin/sing-box</string>");
    expect(plist).toContain("<string>run</string>");
    expect(plist).toContain("<string>-c</string>");
    expect(plist).toContain("<string>/Users/me/.cache/patchbay/daemon.json</string>");
    expect(plist).toContain("<string>--disable-color</string>");
    expect(plist).toContain("<key>StandardOutPath</key>");
  });

  test("escapes xml-sensitive characters", () => {
    const plist = buildLaunchAgentPlist({
      binary: "/Users/me/&co/sing-box",
      configPath: "/tmp/<config>.json",
      logPath: "/tmp/log",
    });

    expect(plist).toContain("&amp;co");
    expect(plist).toContain("&lt;config&gt;");
  });

  test("builds a one-shot intercept plist", () => {
    const plist = buildInterceptAgentPlist({
      command: ["/opt/homebrew/bin/bun", "/repo/src/index.tsx", "intercept", "on"],
      logPath: "/tmp/intercept.log",
    });

    expect(plist).toContain("<string>dev.patchbay.intercept</string>");
    expect(plist).toContain("<string>intercept</string>");
    expect(plist).toContain("<string>on</string>");
    expect(plist).toContain("<key>RunAtLoad</key>");
    expect(plist).not.toContain("KeepAlive");
    expect(interceptAgentPath()).toContain("dev.patchbay.intercept.plist");
  });
});
