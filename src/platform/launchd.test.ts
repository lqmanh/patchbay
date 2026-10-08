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
      join(homedir(), "Library", "LaunchAgents", "dev.patchbay.wireproxy.plist"),
    );
  });

  test("builds a keep-alive plist with the wireproxy arguments", () => {
    const plist = buildLaunchAgentPlist({
      binary: "/Users/me/.local/bin/wireproxy",
      configPath: "/Users/me/.cache/patchbay/wireproxy.conf",
      healthPort: 9080,
      logPath: "/Users/me/.local/state/patchbay/wireproxy.log",
    });

    expect(plist).toContain("<string>dev.patchbay.wireproxy</string>");
    expect(plist).toContain("<key>KeepAlive</key>");
    expect(plist).toContain("<key>RunAtLoad</key>");
    expect(plist).toContain("<string>/Users/me/.local/bin/wireproxy</string>");
    expect(plist).toContain("<string>127.0.0.1:9080</string>");
    expect(plist).toContain("<key>StandardOutPath</key>");
  });

  test("escapes xml-sensitive characters", () => {
    const plist = buildLaunchAgentPlist({
      binary: "/Users/me/&co/wireproxy",
      configPath: "/tmp/<config>.conf",
      healthPort: 9080,
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
