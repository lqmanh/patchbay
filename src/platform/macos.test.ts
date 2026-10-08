import { describe, expect, test } from "bun:test";
import { parseActiveInterface, parseServiceForDevice, parseSocksState } from "./macos";

const routeOutput = `   route to: default
destination: default
       mask: default
    gateway: 192.168.1.1
  interface: en0
      flags: <UP,GATEWAY,DONE,STATIC,PRCLONING,GLOBAL>
`;

const servicesOutput = `An asterisk (*) denotes that a network service is disabled.
(1) USB 10/100/1000 LAN
(Hardware Port: USB 10/100/1000 LAN, Device: en5)

(2) Wi-Fi
(Hardware Port: Wi-Fi, Device: en0)

(3) Thunderbolt Bridge
(Hardware Port: Thunderbolt Bridge, Device: bridge0)

(4) MEGA VPN
(Hardware Port: nz.mega.MEGAVPN, Device: )
`;

const socksOffOutput = `Enabled: No
Server: 
Port: 0
Authenticated Proxy Enabled: 0
`;

describe("macOS parsers", () => {
  test("finds the default interface", () => {
    expect(parseActiveInterface(routeOutput)).toBe("en0");
    expect(parseActiveInterface("nothing here")).toBeNull();
  });

  test("maps a device to its network service, tolerating empty devices", () => {
    expect(parseServiceForDevice(servicesOutput, "en0")).toBe("Wi-Fi");
    expect(parseServiceForDevice(servicesOutput, "en5")).toBe("USB 10/100/1000 LAN");
    expect(parseServiceForDevice(servicesOutput, "bridge0")).toBe("Thunderbolt Bridge");
    expect(parseServiceForDevice(servicesOutput, "en99")).toBeNull();
  });

  test("parses socks proxy state", () => {
    expect(parseSocksState(socksOffOutput)).toEqual({
      enabled: false,
      host: "",
      port: 0,
    });
    expect(parseSocksState("Enabled: Yes\nServer: 127.0.0.1\nPort: 25344\n")).toEqual({
      enabled: true,
      host: "127.0.0.1",
      port: 25344,
    });
  });
});
