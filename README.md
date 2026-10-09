# patchbay

Per-domain VPN routing for macOS. A TUI front-end for [sing-box](https://github.com/SagerNet/sing-box): only the domains you list go through the VPN, everything else stays direct.

## Requirements

- [Bun](https://bun.sh) >= 1.4
- [sing-box](https://github.com/SagerNet/sing-box) — `brew install sing-box`
- A WireGuard credential file from your VPN provider (e.g. MEGA VPN `.conf`) in `~/.config/patchbay/servers/`

## Development

```sh
bun install
bun run dev        # run the TUI
bun run smoke      # start sing-box, verify the tunnel, stop it (optional: server file)
bun run src/index.tsx intercept on|off   # headless control (also used by the login agent)
bun test           # unit tests
bun run check      # typecheck + lint + format check
bun run build      # compile a standalone binary to dist/patchbay
```

## Usage

- `space` — toggle interception (daemon + system SOCKS proxy; previous proxy state is restored on disable)
- `tab` — switch pane; `j`/`k` or arrows move; `enter` selects a server or toggles a rule
- `a` — add a domain rule; `e` — edit the selected rule; `d` — delete the selected rule
- `s` — settings: mode (split/full), socks port, launch at login, intercept at login (auto-enables the system proxy when you log in)
- `r` — restart the daemon; `q` — quit

Only domains matching a rule are routed through the VPN; everything else is dialed directly. `server` selection maps to the `.conf` files in `~/.config/patchbay/servers/`.

Another VPN app (including the MEGA desktop client) must be disconnected while patchbay intercepts: its tunnel becomes the default route, macOS replaces the global proxy state that apps read, and enabling interception fails with "no active network service".

### Firefox-based browsers

Firefox, Zen and other Firefox-based browsers parse the macOS system SOCKS proxy as **SOCKS v4** by default: they resolve DNS locally and send only IP addresses, so no domain rule can match and the traffic goes direct. Chrome and Safari assume v5 and need no setup.

In the browser's `about:config`, set (applies immediately, no restart):

- `network.proxy.default_pac_script_socks_version` → `5`

Keep the browser's proxy setting on "Use system proxy settings" (the default); "No proxy" and "Auto-detect proxy settings" bypass patchbay entirely. This path also resolves DNS at the proxy, which is what allows domain rules to match.
