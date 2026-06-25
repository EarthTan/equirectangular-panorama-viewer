# 360 Pano Viewer — Desktop Package Design

> Wraps the existing Vite + Three.js web app in a **Tauri 2** shell so it ships as a double-clickable .app / .exe / .AppImage.

## Goal

A user downloads one file, double-clicks it, and gets the 360° pano viewer in its own window — no browser, no localhost, no install wizard. The existing web app stays the single source of truth; the desktop wrapper is purely additive.

## Non-goals (explicit YAGNI)

- Code signing / notarization (user opted out — Gatekeeper and SmartScreen warnings are accepted)
- Auto-update channel
- OS-level file association (`.jpg` → open with this app)
- Drag-onto-app-icon to open a file
- Custom app icon design (use Tauri's placeholder set; user can swap later)
- Tauri-side menus, tray, global shortcuts
- CI for cross-platform builds (macOS must be built on macOS; documented, not automated)
- CSP hardening (left off; revisit only if a future change requires it)

## Architecture

```
Tauri host (Rust binary, ~3 MB)
  └─ one window, no menu, no tray
     └─ loads dist/index.html via file://
        └─ existing Vite app runs unchanged
```

The web app has **zero awareness of desktop**. No `window.__TAURI__`, no IPC, no new JS APIs. File loading still uses the existing `<input type="file">` + drag-drop into the window. The Tauri Rust side is intentionally dumb: it opens a window, loads the local file, and gets out of the way.

This is the key design decision: **zero changes to the existing app code**. The wrapper is purely additive, which is what makes Tauri the right choice over Electron for a 3-file frontend app.

## Project layout (additions only)

```
vr-show/
├── src/                       (unchanged)
├── tests/                     (unchanged)
├── index.html                 (unchanged)
├── package.json               (1 devDep + 3 scripts added)
├── vite.config.js             (unchanged)
├── docs/
│   ├── superpowers/
│   │   ├── plans/             (new plan: 2026-06-25-desktop-package.md)
│   │   └── specs/             (this file)
│   └── TESTING.md             (appended: desktop manual checklist)
└── src-tauri/                 (new — Tauri Rust project)
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── icons/
    │   ├── 32x32.png          (placeholder)
    │   ├── 128x128.png        (placeholder)
    │   ├── 128x128@2x.png     (placeholder)
    │   ├── icon.icns          (placeholder, macOS)
    │   ├── icon.ico           (placeholder, Windows)
    │   └── icon.png           (placeholder, Linux)
    └── src/
        └── main.rs            (~30 lines: open window, load ../dist/index.html)
```

## Key configuration

### `package.json` changes

Add to `devDependencies`:
- `@tauri-apps/cli@^2`

Add to `scripts`:
- `"tauri": "tauri"`
- `"dist:mac": "tauri build"`
- `"dist:win": "tauri build"`
- `"dist:linux": "tauri build"`
- `"tauri:dev": "tauri dev"`

(Each platform's build command is the same — Tauri auto-detects host OS — but separate scripts document intent and make per-OS CI jobs easy later.)

### `src-tauri/tauri.conf.json` essentials

- `productName: "360 Pano Viewer"`
- `version: "0.1.0"` (mirrors `package.json` — keep both in sync manually for now; no automated sync in scope)
- `identifier: "com.asyncb.vrshow"` (placeholder; reverse-domain format, user can change)
- `frontendDist: "../dist"` (Tauri's release build serves from this static folder)
- `devUrl: "http://localhost:5173"` (used by `tauri dev` only)
- `build.beforeBuildCommand: "npm run build"` (Tauri runs `vite build` before bundling)
- `build.beforeDevCommand: "npm run dev"` (used by `tauri dev` only)
- `windows[0]`:
  - `width: 1280, height: 800`
  - `minWidth: 400, minHeight: 300`
  - `title: "360° 全景图查看器"`
  - `fullscreen: false`
  - `decorations: true`
  - `resizable: true`
- `app.security.csp: null` (existing app uses inline SVG; CSP stays off)
- `bundle.targets`: `"all"` on macOS/Windows, `["deb", "appimage"]` on Linux
- `bundle.icon`: list of placeholder files in `src-tauri/icons/`
- `bundle.category: "Photography"`
- `bundle.shortDescription: "View 360° panoramic images"`

### `src-tauri/src/main.rs`

Minimal: Tauri 2's default boilerplate plus a single-window config block that loads `frontendDist`. ~30 lines, no custom commands, no event handlers, no state. The window does not even need a menu — Tauri 2 ships a sensible default and the user did not ask for one.

## Build pipeline

Per target OS (the user runs these commands on the matching host):

| Command           | What it does                                                      | Output                                                  |
|-------------------|-------------------------------------------------------------------|---------------------------------------------------------|
| `npm run tauri dev` | Vite dev server + Tauri window pointing at `localhost:5173`     | Live-reload dev session                                 |
| `npm run dist:mac`  | `vite build` → Tauri macOS bundler                              | `src-tauri/target/release/bundle/macos/*.app` + `dmg/` |
| `npm run dist:win`  | `vite build` → Tauri Windows bundler                             | `src-tauri/target/release/bundle/{msi,nsis}/*`         |
| `npm run dist:linux`| `vite build` → Tauri Linux bundler                              | `src-tauri/target/release/bundle/{deb,appimage}/*`     |

Existing `npm run build` and `npm test` are unchanged and continue to work for the web-only build path.

## Testing strategy

Two layers, both kept minimal because the wrapper is dumb:

1. **Existing JS tests stay green.** `tests/wireApp.test.js` and `tests/viewer.test.js` are the safety net — if a Tauri change ever touches JS, these must still pass. The test suite runs on every change (`npm test`).

2. **Manual verification checklist appended to `TESTING.md`.** For each target OS:
   - Run `npm run dist:<os>`
   - Launch the produced artifact
   - Drop in a pano, confirm drag / wheel / auto-rotate work
   - Confirm the window opens at the configured size, resizes, and closes cleanly
   - No automated visual regression — Three.js in WebView is the same Three.js in the browser; behavior is already covered by existing tests.

3. **Rust side gets no automated test.** The Rust code is ~30 lines of declarative config; integration-testing it would mean spinning up the whole Tauri runtime in a test harness for no behavioral surface. Verified by the manual checklist above. If a future change adds Rust logic, add a Rust test then.

## What does NOT change

- `src/main.js`, `src/viewer.js`, `src/fileLoader.js`, all UI files
- `index.html`, `style.css`
- `package.json` runtime dependencies
- `vite.config.js`
- All 16 existing tests

## Dependencies to add

- `devDependencies`: `@tauri-apps/cli@^2` (JS-side CLI; Rust toolchain is installed by the CLI on first build via `rustup`)
- System-level (one-time): Rust toolchain via `rustup` (~1 GB download)

## Risk register

1. **WebGL in WKWebView / WebView2 / WebKitGTK** — fine in practice; Three.js runs in millions of WebView-based apps. Mitigation: manual smoke test on each OS is in the test plan.

2. **CSP left off** — acceptable for an offline app. If a future change adds a network request, set CSP then.

3. **`URL.createObjectURL` and `Image` in WebView** — both are standard browser APIs. No code change expected. Verified by manual test.

4. **macOS build requires a Mac** — Tauri does not support macOS cross-compilation from Linux. `TESTING.md` will say: "macOS artifacts must be built on macOS."

5. **First-time Tauri install pulls a lot** — Tauri downloads Rust toolchain + Tauri crates on first `tauri build`. One-time cost, cached afterward. Documented in the plan's setup step.

6. **Linux WebKitGTK version drift** — some older Linux distros ship WebKitGTK 4.1 vs the 4.6 Tauri 2 expects. Documented as a "WebKitGTK 4.6+ required" prerequisite. User is on a modern distro; will note in `TESTING.md`.

## Workflow

1. Spec is this file. User reviews.
2. After approval, invoke `superpowers:writing-plans` to lay out TDD tasks (mostly: scaffold Tauri project, configure, verify build, manual checklist).
3. Plans are executed via `superpowers:subagent-driven-development`.

## Open decisions for the user (none blocking)

- App icon: use Tauri's default placeholder set, or do you want to drop in a real `.icns`/`.ico`/`.png` set now? (Plan will use placeholders; user can swap without code change later.)
- `identifier`: `com.asyncb.vrshow` is a placeholder. User can change in `tauri.conf.json` before the first build.
