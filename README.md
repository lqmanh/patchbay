# patchbay

Per-domain VPN routing for macOS. A TUI front-end for [wireproxy](https://github.com/windtf/wireproxy): only the domains you list go through the VPN, everything else stays direct.

## Requirements

- [Bun](https://bun.sh) >= 1.4
- `wireproxy` built from current master — the latest release (v1.1.3) lacks `TunnelDomains`/`LogDomains`: `GOBIN="$HOME/.local/bin" go install github.com/windtf/wireproxy/cmd/wireproxy@a4c526901560`
- A WireGuard credential file from your VPN provider (e.g. MEGA VPN `.conf`) in `~/.config/patchbay/servers/`

## Development

```sh
bun install
bun run dev        # run the TUI
bun run smoke      # start wireproxy, verify the tunnel, stop it (optional: server file)
bun run src/index.tsx intercept on|off   # headless control (also used by the login agent)
bun test           # unit tests
bun run check      # typecheck + lint + format check
bun run build      # compile a standalone binary to dist/patchbay
```

## Usage

- `space` — toggle interception (daemon + system SOCKS proxy; previous proxy state is restored on disable)
- `tab` — switch pane; `j`/`k` or arrows move; `enter` selects a server or toggles a rule
- `a` — add a domain rule; `d` — delete the selected rule
- `s` — settings: mode (split/full), ports, launch at login, intercept at login (auto-enables the system proxy when you log in)
- `r` — restart the daemon; `q` — quit

Only domains matching a rule are routed through the VPN; everything else is dialed directly by wireproxy. `server` selection maps to the `.conf` files in `~/.config/patchbay/servers/`.

Another VPN app (including the MEGA desktop client) must be disconnected while patchbay intercepts: its tunnel becomes the default route, macOS replaces the global proxy state that apps read, and enabling interception fails with "no active network service".

## Layout

- `~/.config/patchbay/` — settings and server credentials (`servers/*.conf`)
- `~/.cache/patchbay/` — generated wireproxy config
- `~/.local/state/patchbay/` — PID, logs, previous proxy state
