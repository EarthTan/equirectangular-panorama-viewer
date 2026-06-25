# Manual Testing Checklist

This is a frontend 3D project — most behavior is verified by eye in a real browser. Run through these 10 scenarios after each change to the viewer/UI.

## Setup

1. `npm install` (if not done)
2. `npm run dev` — open http://127.0.0.1:5173/
3. Have at least one equirectangular 2:1 image ready (the bundled `Qwen-Image-2512_00001_.png` works). Have a non-image file (e.g. a `.pdf` or `.txt`) ready for the error test.

## Scenarios

### 1. Empty state on launch
- [ ] Page loads showing "拖入一张全景图 / 或将图片拖到此处 · 点击选择文件" centered on a black background.
- [ ] The empty state border is dashed gray.

### 2. Load image by drop
- [ ] Drag the equirectangular image from your file manager onto the page.
- [ ] Empty state fades out (0.3s).
- [ ] The panorama appears wrapped around the viewer.

### 3. Drag to rotate
- [ ] Click and drag horizontally → camera yaws (no limit on rotation).
- [ ] Click and drag vertically → camera pitches.
- [ ] Drag toward the top → pitch stops at +89° (no flip).
- [ ] Cursor changes to `grabbing` during drag, back to `grab` on release.

### 4. Wheel zoom
- [ ] Scroll up → FOV decreases (zooms in, scene appears closer).
- [ ] Scroll down → FOV increases (zooms out, scene appears farther).
- [ ] FOV clamps at 30° (max zoom-in) and 100° (max zoom-out) — no NaN, no flicker.

### 5. Auto-rotate stops on first interaction
- [ ] After loading the image, the scene slowly rotates on its own.
- [ ] On first pointer-down or wheel event, the rotation stops immediately.

### 6. HUD auto-hide
- [ ] After loading the image, a "拖动旋转 · 滚轮缩放" hint appears at the bottom.
- [ ] It fades out within ~3 seconds (no interaction needed).
- [ ] If the user interacts before 3s, the hint fades immediately.

### 7. Load a second image
- [ ] Drop a different panoramic image after the first.
- [ ] The new image replaces the old one (no flicker, no leftover artifacts).
- [ ] Empty state does NOT reappear; HUD shows again (then fades).
- [ ] Auto-rotate does NOT restart (only fires on first load).

### 8. Non-image file shows error
- [ ] Drop a `.pdf` or `.txt` file.
- [ ] A red banner appears at the top: "请拖入图片文件".
- [ ] The banner disappears after ~3 seconds.
- [ ] Viewer state is unchanged (no crash, no flash).

### 9. Window resize
- [ ] Resize the browser window.
- [ ] Canvas fills the new size, image is not stretched or distorted.

### 10. Non-2:1 image loads with warning
- [ ] Drop a non-panoramic image (e.g. a 4:3 photo).
- [ ] Image still loads and displays (it will look stretched, that's expected).
- [ ] Browser DevTools console shows: `Image aspect ratio is W:H, not 2:1. It will display stretched.`

---

# Desktop Build (Tauri)

Tauri 2 wraps the same web app in a system WebView (WKWebView / WebView2 / WebKitGTK). The existing 10 web scenarios all still apply in the desktop window — drop behavior, drag rotation, wheel zoom, auto-rotate, HUD, error banner, and resize all work the same way because the web code is unchanged.

## Build prerequisites

| OS       | Prerequisite (one-time)                                                                 |
|----------|------------------------------------------------------------------------------------------|
| macOS    | Xcode Command Line Tools (`xcode-select --install`). Build must be run ON macOS.         |
| Windows  | Microsoft C++ Build Tools (VS 2022 Build Tools or equivalent) + WebView2 (preinstalled on Win10+). |
| Linux    | `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`, `librsvg2-dev`, `libayatana-appindicator3-dev` (build host only). Install with `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev libayatana-appindicator3-dev`. |
| All      | Rust toolchain via `rustup` (https://rustup.rs). ~1 GB, one-time.                        |

## Build commands

| Command           | Output (on the matching host OS)                                                |
|-------------------|----------------------------------------------------------------------------------|
| `npm run tauri:dev` | Dev session: vite dev server + Tauri window pointing at `http://localhost:5173` |
| `npm run dist:mac`  | `src-tauri/target/release/bundle/macos/360 Pano Viewer.app` + `dmg/`            |
| `npm run dist:win`  | `src-tauri/target/release/bundle/{msi,nsis}/...`                                |
| `npm run dist:linux`| `src-tauri/target/release/bundle/{deb,rpm,appimage}/...`                        |

The first build takes 5–15 minutes (downloads + compiles Tauri Rust crates). Subsequent builds are incremental and fast.

## Per-OS manual smoke

Run these after building, on the target OS. Each one is a single launch + interaction.

### D11. Desktop launch (macOS / Windows / Linux)
- [ ] Double-click the produced artifact (`.app` on macOS, `.exe` or installed app on Windows, `.AppImage` or installed `.deb` on Linux).
- [ ] Window opens at ~1280×800 with title "360° 全景图查看器".
- [ ] Empty state appears centered ("拖入一张全景图").
- [ ] Resize the window — canvas fills, no distortion.

### D12. Drag-drop a panorama
- [ ] Drag an equirectangular image from the file manager into the desktop window.
- [ ] Empty state fades, pano appears. (Identical to web scenario 2.)

### D13. Drag-rotate + wheel-zoom
- [ ] Click-and-drag horizontal → camera yaws (FPS-style, drag right = look right).
- [ ] Click-and-drag vertical → camera pitches, clamped at ±89°.
- [ ] Mouse wheel → FOV zooms in/out, clamped at 30°–100°.
- [ ] Auto-rotate stops on first interaction.

### D14. Non-image file shows error
- [ ] Drop a `.pdf` or `.txt` into the window.
- [ ] Red banner appears with "请拖入图片文件", then auto-dismisses.

### D15. Quit / relaunch
- [ ] Close the window with the OS close button.
- [ ] Process exits cleanly (check `ps` / Activity Monitor / Task Manager — no zombie process).
- [ ] Relaunch — empty state appears, no stale state.

## Cross-platform notes

- **macOS Gatekeeper**: unsigned builds will be blocked on first launch. Right-click the .app → "Open" → confirm the warning. After that, double-click works.
- **Windows SmartScreen**: unsigned builds show "Windows protected your PC" → click "More info" → "Run anyway".
- **Linux AppImage**: `chmod +x` the downloaded file, then run it. No install needed.
- **WebKitGTK version**: Tauri 2 requires WebKitGTK 4.6+ at runtime on Linux. If the user's distro only ships 4.4, the app will fail to start. (The build host is fine — it can be a different distro than the user's machine.)
