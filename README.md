# pano-viewer

A 360° panoramic image viewer, written in pure Rust (`winit` + `wgpu` + `egui`).

![Platforms: macOS, Ubuntu, Windows](https://img.shields.io/badge/platforms-macOS%20%7C%20Ubuntu%20%7C%20Windows-blue)
![License: MIT](https://img.shields.io/badge/license-MIT-green)

## Install

Pre-built binaries for all platforms are attached to every [release](https://github.com/EarthTan/equirectangular-panorama-viewer/releases).

| Platform | Recommended | Alternative |
|----------|-------------|-------------|
| **macOS** (Apple Silicon) | `pano-viewer-<ver>-macos-arm64.dmg` | `brew install EarthTan/tap/pano-viewer` |
| **macOS** (Intel) | `pano-viewer-<ver>-macos-x86_64.dmg` | `brew install EarthTan/tap/pano-viewer` |
| **Ubuntu / Debian** | `pano-viewer_<ver>-1_amd64.deb` | `sudo dpkg -i pano-viewer_*.deb` |
| **Ubuntu** (portable) | `pano-viewer-<ver>-x86_64.AppImage` | `chmod +x *.AppImage && ./pano-viewer-*.AppImage` |
| **Windows** (installer) | `pano-viewer-<ver>-windows-x86_64.exe` (NSIS) | `scoop install EarthTan/scoop-bucket/pano-viewer` |
| **Windows** (portable) | `pano-viewer-<ver>-windows-x86_64.zip` | extract & run `pano-viewer.exe` |
| **Any** | `cargo install pano-viewer` (from [crates.io](https://crates.io/crates/pano-viewer)) | |

> **macOS first launch:** the binary is **not signed / not notarized**. After opening
> the `.dmg` and dragging the app to Applications, right-click the app → **Open** →
> confirm. Subsequent launches are normal.
>
> **Windows SmartScreen:** the installer is **not signed**. Click *More info* →
> *Run anyway* on the SmartScreen prompt.

## Build from source

```bash
cargo build --release
```

Binary at `target/release/pano-viewer` (or `pano-viewer.exe` on Windows).

### System dependencies (Linux only)

```bash
sudo apt install -y \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libxkbcommon-dev libwayland-dev libxinerama-dev \
  libegl1-mesa-dev libasound2-dev
```

## Run

```bash
cargo run --release -- path/to/equirectangular.jpg
```

Or launch without arguments and drag an image into the window.

## Controls

| Action | Effect |
|--------|--------|
| Click + drag | Rotate camera |
| Scroll wheel | Zoom (FOV 30°–100°) |
| Drop image file | Load panorama |
| "打开文件" button | Open file picker to load a different panorama |
| CLI argument | Load on startup |

First interaction stops auto-rotation.

## Supported formats

PNG and JPEG (equirectangular, 2:1 aspect ratio). Non-2:1 images load but display stretched with a warning.

## Project layout

```
crates/pano-viewer/src/
├── main.rs        # CLI arg parsing, event loop
├── app.rs         # ApplicationHandler, rendering, input, egui
├── window.rs      # Window + wgpu surface + device
├── renderer.rs    # 3D pipeline (sphere + panorama)
├── input.rs       # winit events → typed actions
├── file.rs        # Image loading + aspect ratio check
├── error.rs       # Error types
├── ui.rs          # egui overlay (HUD, error banner)
└── scene/
    ├── camera.rs  # CameraState (yaw/pitch/FOV)
    ├── sphere.rs  # Sphere mesh generation
    └── texture.rs # GPU texture upload
crates/pano-viewer/shaders/
├── pano.vert.wgsl # Vertex shader
└── pano.frag.wgsl # Fragment shader
```

See `docs/superpowers/specs/2026-06-29-pure-rust-port-design.md` for the full design spec.
See `docs/superpowers/plans/2026-06-29-pure-rust-port.md` for the implementation plan.

## License

MIT — see [`LICENSE`](./LICENSE).
