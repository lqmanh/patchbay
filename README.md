# patchbay

Per-domain VPN routing for macOS. A desktop app and headless CLI for [sing-box](https://github.com/SagerNet/sing-box): only the domains you list go through the VPN, everything else stays direct.

## Requirements

- [sing-box](https://github.com/SagerNet/sing-box) — `brew install sing-box`
- A WireGuard credential file from your VPN provider (e.g. MEGA VPN `.conf`) in `~/.config/patchbay/servers/`
- Rust 1.98.1, pinned in `rust-toolchain.toml`

## Build and run

```sh
cargo run -p patchbay-gui                       # desktop app
cargo run -p patchbay-cli -- intercept on|off   # headless control (also used by the login agent)
cargo run -p patchbay-cli -- smoke [server]     # start sing-box, verify the tunnel, stop it
cargo test --workspace                          # unit tests
```

## Usage

The app window has a sidebar with:

- **Status** — the interception switch (daemon plus system SOCKS proxy; the previous proxy state is restored on disable), active server, split/full mode, SOCKS port, and tunnel restart.
- **Servers** — the `.conf` files in `~/.config/patchbay/servers/`; the active one is what the tunnel runs with.
- **Rules** — domains and regexes routed through the VPN; everything else is dialed directly.
- **Log** — the sing-box daemon log, with level colors and a follow switch.
- **Settings** — launch at login, intercept at login, and theme (System/Light/Dark).

Another VPN app (including the MEGA desktop client) must be disconnected while patchbay intercepts: its tunnel becomes the default route, macOS replaces the global proxy state that apps read, and enabling interception fails with "no active network service".

### Firefox-based browsers

Firefox, Zen and other Firefox-based browsers parse the macOS system SOCKS proxy as **SOCKS v4** by default: they resolve DNS locally and send only IP addresses, so no domain rule can match and the traffic goes direct. Chrome and Safari assume v5 and need no setup.

In the browser's `about:config`, set (applies immediately, no restart):

- `network.proxy.default_pac_script_socks_version` → `5`

Keep the browser's proxy setting on "Use system proxy settings" (the default); "No proxy" and "Auto-detect proxy settings" bypass patchbay entirely. This path also resolves DNS at the proxy, which is what allows domain rules to match.

## Layout

- `crates/patchbay-core` — config, rules, sing-box config generation, daemon lifecycle, macOS platform integration
- `crates/patchbay-cli` — `patchbay`: `intercept on|off` and `smoke`
- `crates/patchbay-gui` — `patchbay-gui`: the gpui-kit desktop app

`crates/patchbay-core/tests/fixtures/` holds the byte-exact reference output of the retired TypeScript renderer; the Rust generator must keep matching it.

## Credits

Built with [gpui](https://github.com/zed-industries/zed) and [gpui-kit](https://gpui-kit.com); the tunnel is [sing-box](https://github.com/SagerNet/sing-box).
