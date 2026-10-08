import { describe, expect, test } from "bun:test";
import { generateConfig, parseMetrics } from "./wireproxy";

const serverConf = `[Interface]
PrivateKey = FAKE_PRIVATE_KEY
Address = 10.0.0.2/32
DNS = 10.0.0.1

[Peer]
PublicKey = FAKE_PUBLIC_KEY
AllowedIPs = 0.0.0.0/0
Endpoint = 203.0.113.1:51820
`;

const base = {
  socksPort: 25344,
  healthPort: 9080,
  mode: "split" as const,
  domains: [
    { id: "a", value: "reddit.com", kind: "domain" as const, enabled: true },
    { id: "b", value: "medium.com", kind: "domain" as const, enabled: false },
  ],
};

describe("generateConfig", () => {
  test("keeps the server config and adds check-alive before peers", () => {
    const generated = generateConfig(serverConf, base);
    expect(generated).toContain("PrivateKey = FAKE_PRIVATE_KEY");
    expect(generated.indexOf("CheckAlive = 1.1.1.1")).toBeLessThan(
      generated.indexOf("[Peer]"),
    );
  });

  test("adds socks and tunnel domains for enabled rules", () => {
    const generated = generateConfig(serverConf, base);
    expect(generated).toContain("[Socks5]");
    expect(generated).toContain("BindAddress = 127.0.0.1:25344");
    expect(generated).toContain("TunnelDomains = ^(.*\\.)?reddit\\.com$");
    expect(generated).not.toContain("medium");
    expect(generated).toContain("LogDomains = true");
  });

  test("omits tunnel domains in full mode", () => {
    const generated = generateConfig(serverConf, { ...base, mode: "full" });
    expect(generated).not.toContain("TunnelDomains");
  });

  test("empty split rules tunnel nothing", () => {
    const generated = generateConfig(serverConf, { ...base, domains: [] });
    expect(generated).toContain("TunnelDomains = ^$");
  });
});

describe("parseMetrics", () => {
  test("reads handshake and transfer counters from the UAPI text", () => {
    const sample = [
      "private_key=REDACTED",
      "listen_port=56205",
      "public_key=abc",
      "endpoint=149.102.231.231:51820",
      "last_handshake_time_sec=1759600000",
      "last_handshake_time_nsec=123456",
      "rx_bytes=123456",
      "tx_bytes=7890",
      "persistent_keepalive_interval=25",
    ].join("\n");

    expect(parseMetrics(sample)).toEqual({
      lastHandshakeSec: 1759600000,
      rxBytes: 123456,
      txBytes: 7890,
    });
  });

  test("returns nulls for missing fields", () => {
    expect(parseMetrics("listen_port=1")).toEqual({
      lastHandshakeSec: null,
      rxBytes: null,
      txBytes: null,
    });
  });
});
