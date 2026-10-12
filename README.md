<p align="center">
  <img src="assets/icon/patchbay.svg" width="128" height="128" alt="Patchbay app icon">
</p>

<h1 align="center">Patchbay</h1>

<p align="center">A macOS app for rule-based VPN routing: only matched traffic goes through the VPN, everything else stays direct.</p>

## Features

## Installation

### Requirements

- [sing-box](https://github.com/SagerNet/sing-box) — `brew install sing-box`
- WireGuard credential files from your VPN provider (e.g. MEGA VPN `.conf`) in `~/.config/patchbay/servers/`

### Build from source

See [Development](#development).

## Usage

Another VPN app (including the MEGA desktop client) must be disconnected while patchbay intercepts: its tunnel becomes the default route, macOS replaces the global proxy state that apps read, and enabling interception fails with "no active network service".

### Firefox-based browsers

Firefox, Zen and other Firefox-based browsers parse the macOS system SOCKS proxy as **SOCKS v4** by default: they resolve DNS locally and send only IP addresses, so no domain rule can match and the traffic goes direct. Chrome and Safari assume v5 and need no setup.

In the browser's `about:config`, set (applies immediately, no restart):

- `network.proxy.default_pac_script_socks_version` → `5`

Keep the browser's proxy setting on "Use system proxy settings" (the default); "No proxy" and "Auto-detect proxy settings" bypass patchbay entirely. This path also resolves DNS at the proxy, which is what allows domain rules to match.

## Development

Rust 1.98.1, pinned in `rust-toolchain.toml`.

```sh
cargo run -p patchbay-gui                       # desktop app
cargo run -p patchbay-cli -- intercept on|off   # headless control (also used by the login agent)
cargo run -p patchbay-cli -- smoke [server]     # start sing-box, verify the tunnel, stop it
cargo test --workspace                          # unit tests
```

## Credits

- [gpui](https://github.com/zed-industries/zed) and [gpui-kit](https://gpui-kit.com)
- [sing-box](https://github.com/SagerNet/sing-box)
