# Changelog

All notable changes to **pano-viewer** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Project renamed from `vr-show` to `pano-viewer`** (binary, crate, package metadata, deb name).
- Crate version bumped to `0.3.0` (was `0.2.0`) to align with the existing `v0.3.0` git tag.

### Added
- Full multi-platform release pipeline: macOS `.dmg` (arm64 + x86_64 + universal), Windows NSIS `.exe` + portable `.zip`, Linux `.deb` + `.AppImage` + `.tar.gz`, plus SHA256 checksums.
- Homebrew tap (`EarthTan/tap`) and Scoop bucket (`EarthTan/scoop-bucket`) auto-updated on every release.
- crates.io publishing (no manual bump needed).

### Fixed
- Surface configuration no longer panics on GPUs whose
  `max_texture_dimension_2d` is smaller than the window's physical size
  (e.g. low-power iGPUs, 1280×800 logical @ 2x Retina = 2560×1600 on
  adapters capped at 2048). The window/surface size is now clamped to
  the adapter's limit in both `WindowState::new` and `WindowState::resize`.

## [0.2.0] — 2026-06-30

### Changed
- **Complete rewrite** from Vite + Three.js + Tauri 2 to a single pure-Rust desktop application.
- New tech stack: `winit` + `wgpu` + `egui`.

### Added
- 360° equirectangular panorama rendering on an inward-facing sphere.
- Drag to rotate, scroll wheel to zoom (FOV 30°–100°).
- Auto-rotate with first-interaction stop.
- Drag-and-drop image loading.
- Command-line argument loading.
- egui overlay: empty state, HUD, error banner.
- 24 unit tests.

[Unreleased]: https://github.com/EarthTan/equirectangular-panorama-viewer/compare/v0.3.0...HEAD
[0.2.0]: https://github.com/EarthTan/equirectangular-panorama-viewer/releases/tag/v0.2.0
