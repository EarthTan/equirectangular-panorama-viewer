# Manual Testing Checklist

## Setup

1. `cargo build --release`
2. `cargo run --release -- ./Qwen-Image-2512_00001_.png`

Binary path: `target/release/pano-viewer` (or `pano-viewer.exe` on Windows).

The bundled `Qwen-Image-2512_00001_.png` is a 2:1 equirectangular image suitable for testing.

## Scenarios

### 1. Empty state on launch
- [ ] Launch with no arguments.
- [ ] The window opens at ~1280×800 with a centered dashed-border card.
- [ ] The card shows "拖入一张全景图" and "或将图片拖到此处 · 点击选择文件".

### 2. Load image by drop
- [ ] Drag `Qwen-Image-2512_00001_.png` from the file manager into the window.
- [ ] Empty state disappears; panorama fills the window.
- [ ] HUD "拖动旋转 · 滚轮缩放" appears for ~3 seconds.

### 3. Drag to rotate
- [ ] Click and drag horizontally → camera yaws.
- [ ] Click and drag vertically → camera pitches.
- [ ] Drag toward the top → pitch stops near +89° (no flip).
- [ ] Cursor changes to `grabbing` (if the system supports it) during drag.

### 4. Wheel zoom
- [ ] Scroll up → FOV decreases (zooms in).
- [ ] Scroll down → FOV increases (zooms out).
- [ ] FOV clamps at 30° / 100°.

### 5. Auto-rotate stops on first interaction
- [ ] After loading, the scene rotates on its own.
- [ ] First pointer-down or wheel event stops rotation.

### 6. HUD auto-hide
- [ ] HUD appears for ~3 seconds after loading.
- [ ] It fades out without further interaction.

### 7. Load a second image
- [ ] Drop a different image; it replaces the first cleanly.
- [ ] HUD reappears.
- [ ] Auto-rotate does NOT restart.

### 8. Non-image file shows error
- [ ] Drop a `.pdf` or `.txt` file.
- [ ] Red error banner appears with "请拖入图片文件" for ~3 seconds.
- [ ] The viewer state is unchanged.

### 9. Command-line argument
- [ ] `cargo run --release -- ./some.png` loads the image at startup without dropping.

### 10. Non-2:1 image loads with warning
- [ ] Drop a non-2:1 image (e.g. a 1024×1024 photo).
- [ ] The image still displays (stretched).
- [ ] A warning is printed to stderr: `Image aspect ratio is 1024:1024, not 2:1. It will display stretched.`

### 11. Window resize
- [ ] Resize the window.
- [ ] Canvas fills the new size; image is not stretched.
- [ ] On a device whose effective `max_texture_dimension_2d` is small
      (currently 2048 in wgpu 27 with `downlevel_defaults`), the surface
      is clamped to that limit (the window itself is left at the user's
      chosen size).

### 12. Clean exit
- [ ] Close the window via the OS close button.
- [ ] Process exits; no orphan processes remain.

### 13. Low-max-texture device compatibility
- [ ] On a machine where the device's effective
      `max_texture_dimension_2d` is smaller than the window's physical
      pixel size (e.g. 1280×800 logical @ 2x Retina = 2560×1600 on a
      device created with `downlevel_defaults`, whose limit is 2048),
      the app launches successfully instead of panicking with a wgpu
      validation error.
- [ ] The panorama still displays. Camera aspect ratio is computed
      from the (clamped) surface size and remains internally consistent
      with rendering, even if the corner-case aspect shift is
      observable in the viewport margins.

## Unit tests

Run `cargo test`. The following modules have tests:
- `scene::camera` (yaw/pitch/FOV math, first-interaction guard)
- `scene::sphere` (vertex/index counts, UV range, surface distance)
- `file` (PNG load, aspect ratio warning, IO error)
