<p align="center">
  <img src="assets/icon/patchbay.svg" width="128" height="128" alt="Patchbay app icon">
</p>

<h1 align="center">Patchbay</h1>

<p align="center">A macOS app for rule-based VPN routing: only matched traffic goes through the VPN, everything else stays direct.</p>

## Features

TBD

## Installation

Download the latest `Patchbay-<version>.dmg` from [Releases](https://github.com/lqmanh/patchbay/releases/latest), open it, and drag `Patchbay` into `Applications`. Builds are not notarized yet, so macOS blocks the first launch: open System Settings → Privacy & Security and click Open Anyway (or run `xattr -dr com.apple.quarantine /Applications/Patchbay.app`).

### Requirements

- macOS 26 or later on Apple Silicon
- [sing-box](https://github.com/SagerNet/sing-box) 1.14.0 or later — `brew install sing-box`
- VPN credential files from your provider in `~/.config/patchbay/servers/`: WireGuard `.conf` (e.g. MEGA VPN) or OpenVPN `.ovpn`

### Build from source

See [Development](#development).

## Usage

Another VPN app (including the MEGA desktop client) must be disconnected while patchbay intercepts: its tunnel becomes the default route, macOS replaces the global proxy state that apps read, and enabling interception fails with "no active network service".

### Firefox-based browsers

Firefox, Zen and other Firefox-based browsers parse the macOS system SOCKS proxy as **SOCKS v4** by default: they resolve DNS locally and send only IP addresses, so no domain rule can match and the traffic goes direct. Chrome and Safari assume v5 and need no setup.

In the browser's `about:config`, set (applies immediately, no restart):

- `network.proxy.default_pac_script_socks_version` → `5`

Keep the browser's proxy setting on "Use system proxy settings" (the default); "No proxy" and "Auto-detect proxy settings" bypass patchbay entirely. This path also resolves DNS at the proxy, which is what allows domain rules to match.

### OpenVPN profiles

OpenVPN profiles use the same engine. Inline `<ca>`, `<cert>`, `<key>` and `tls-auth`/`tls-crypt` blocks are read from the profile; if it uses `auth-user-pass` without a file, create `work.auth` next to `work.ovpn` with the username on the first line and the password on the second. TAP, static-key, PKCS#12 and passphrase-protected keys are not supported.

## Development

```sh
cargo run -p patchbay-gui                       # desktop app
cargo run -p patchbay-cli -- intercept on|off   # headless control (also used by the login agent)
cargo run -p patchbay-cli -- smoke [server]     # start sing-box, verify the tunnel, stop it
cargo test --workspace                          # unit tests
scripts/bundle.sh                               # build dist/Patchbay.app
scripts/dmg.sh                                  # package it as dist/Patchbay-<version>.dmg
```

## Credits

- [gpui](https://github.com/zed-industries/zed) and [gpui-kit](https://gpui-kit.com)
- [sing-box](https://github.com/SagerNet/sing-box)
