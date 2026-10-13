# Icons

Patchbay's app icon and menu bar icons share one idea: a patch panel with two cables. The grey **direct** cable carries traffic that goes straight out; the orange **VPN** cable crosses over it, carrying only traffic that matches your rules.

## App icon

| File                   | Use                                                                                                    |
| ---------------------- | ------------------------------------------------------------------------------------------------------ |
| `patchbay.svg`         | Master: 1024 × 1024 canvas on the macOS icon grid. Edit this first.                                    |
| `patchbay-mark.svg`    | In-app logo (sidebar, About): the tile without margin or shadow, 64 × 64 so GPUI renders it at 128 px. |
| `patchbay-artwork.svg` | Jacks and cables without the tile, for other platforms (Windows, Linux) and docs.                      |
| `AppIcon.icns`         | Generated; not yet referenced by an app bundle.                                                        |

The three SVGs share the `#artwork` group verbatim; change it in all of them.

### Rules

- The VPN cable is always on top and the most saturated element; the direct cable stays muted.
- Flat shapes, with soft gradients only on the cables. No glass, highlights or bevels, and no small-size or Dark/Clear/Tinted variants.
- The crossing gap is a mask on the direct cable (`#vpn-gap`), not paint, so it works on any background.
- Plugs are solid discs, the same size as the sockets.

### Palette

Tile `#ecdcc9`, jack `#f8efe4`, socket `#3a2c20`. Tile shadow (master only): black 28%, 2 down, blur 2.

|                                     | Direct                | VPN                   |
| ----------------------------------- | --------------------- | --------------------- |
| Cable gradient (plugs use its ends) | `#8f7964` → `#685341` | `#ff903f` → `#ec4826` |
| Shadow (22%, 1.2 down, blur 1)      | `#3a2614`             | `#b4410c`             |

### Geometry

In the 160-unit design grid:

- **Canvas:** the tile spans 824 px centered on 1024 px (100 px margin), per Apple's macOS icon grid.
- **Tile:** continuous-corner square, radius 36 (22.5%), 60% corner smoothing (as in Figma).
- **Artwork:** scaled 1.1× around the tile center (`matrix(1.1 0 0 1.1 -8 -8)`).
- **Jacks:** 3 × 2 at x = 40, 80, 120 and y = 56, 104. Radius 12 (gaps 16 across, 24 down).
- **Cables:** 8 wide, round caps. Plugs and sockets radius 8, equal to the cable width, leaving a 4-wide jack ring. Direct `M80 56 V104`, VPN `M40 56 C40 96 120 64 120 104`.
- **Crossing gap:** 16 wide along the VPN path, leaving 4 clear on each side of the VPN cable, the same as the jack ring.

## Menu bar icon

Template images: black on transparent; macOS uses only the alpha and tints them to match the menu bar. The artwork is the app icon's jacks and cables, redrawn for this size rather than scaled down.

| File                   | Shows                            | When                     |
| ---------------------- | -------------------------------- | ------------------------ |
| `menubar-split-on.svg` | VPN cable over the direct cable  | Routing on, split mode   |
| `menubar-full-on.svg`  | VPN cable only                   | Routing on, full mode    |
| `menubar-off.svg`      | Direct cable only                | Routing off, either mode |
| `menubar-broken.svg`   | Empty panel with a warning badge | Tunnel down, either mode |

Off and Broken ignore the mode. Off restores the previous system proxy, so everything goes direct. Broken leaves the proxy pointing at a stopped tunnel, so apps that use it can't connect at all.

### Rules

- **Shape shows the mode; strength shows what's active.** Only the VPN cable and the warning badge are solid. The direct cable and every unused jack are at 50% (macOS's secondary-label level).
- All six jacks always show: rings when empty, dots when plugged.
- No crossing gap; at this size it breaks the direct cable apart.

### Geometry

In points on a 22 × 16 canvas (menu bar items get at most 22 pt of the 24 pt menu bar):

- **Pixel grid:** keep jack centers on whole points and jacks 4 pt across, so edges land on whole pixels at @1x and @2x.
- **Jacks:** 3 × 2 at x = 3, 10, 17 and y = 4, 12. Plug radius 2; empty ring radius 1.5, stroke 1.
- **Cables:** 1.5 wide. Direct `M10 4 V12`, VPN `M3 4 C3 10.67 17 5.33 17 12` (the app icon's curve, scaled).
- **Warning badge:** on the VPN cable's end jack. Triangle `M17 8.6 L20.6 15 H13.4 Z`, 0.8 round-joined stroke, 1.3 transparent clear space; the "!" (0.9 × 2.5 bar, 0.5-radius dot) is cut out.

GPUI has no status item API; showing these needs the `tray-icon` crate or `NSStatusItem` via `objc2`.

## Export

The exports are committed so builds don't need these tools. Re-run `./build.sh` after editing any SVG and commit the outputs with it.

```sh
cargo install resvg   # once; the same renderer GPUI uses
./build.sh            # AppIcon.icns (16–512 pt, @1x/@2x), menubar-*.png (22 × 16) and @2x.png (44 × 32)
```
