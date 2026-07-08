# Changelog

All notable changes to **pano-viewer** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] — 2026-07-08

### Added
- Full multi-platform release pipeline: macOS `.dmg` (arm64 + x86_64 + universal), Windows NSIS `.exe` + portable `.zip`, Linux `.deb` + `.AppImage` + `.tar.gz`, plus SHA256 checksums.
- Homebrew tap (`EarthTan/tap`) and Scoop bucket (`EarthTan/scoop-bucket`) auto-updated on every release.
- crates.io publishing (no manual bump needed).
- Async panorama loading with a spinner overlay. Selecting a file no
  longer blocks the event loop on disk read + image decode + GPU upload;
  the CPU-side decode now runs on a background thread and the main
  thread only handles the GPU upload. A centered card with a rotating
  arc spinner plus the file name is shown while the load is in flight,
  and the render loop keeps redrawing so the spinner animates smoothly.
  Starting a new load while one is already in flight cancels the
  previous load by dropping its channel (the prior result is silently
  discarded). New `loader` module, `PanoramaTexture::from_rgba`,
  `UiState::begin_loading` / `clear_loading`, and `App::start_async_load`
  + `poll_load_result` make up the pipeline.

### Fixed
- Surface configuration no longer panics when the window's physical
  pixel size exceeds the device's effective `max_texture_dimension_2d`
  (e.g. 1280×800 logical @ 2x Retina = 2560×1600 on a device created
  with `wgpu::Limits::downlevel_defaults()`, which forces the device
  limit to 2048 in wgpu 27). The surface size is now clamped to that
  device limit in both `WindowState::new` and `WindowState::resize`,
  so the same panic cannot reoccur on HiDPI displays or when the user
  enlarges the window. The clamp is per-axis, leaving the
  non-overflowing axis unchanged.
- `WindowState::resize` now skips `surface.configure()` when the
  clamped dimensions haven't changed, avoiding a redundant configure
  (and its validation) on synthetic `Resized` events during window
  creation on macOS.

### Changed
- **Project renamed from `vr-show` to `pano-viewer`** (binary, crate, package metadata, deb name).
- Crate version bumped to `0.3.0` (was `0.2.0`) to align with the existing `v0.3.0` git tag.

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
