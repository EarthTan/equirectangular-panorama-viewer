# Surface Max-Texture Clamp — Design

**Date**: 2026-07-07 (revised after manual smoke-test discovery: 2026-07-07)
**Project**: `equirectangular-panorama-viewer` (crate `pano-viewer`)
**Status**: Implementation complete (A-scheme, see "Root cause" below)
**Trigger**: `cargo run --release -- ./Qwen-Image-2512_00001_.png` panics on
certain macOS configurations with a wgpu validation error:

> `Surface` width and height must be within the maximum supported texture
> size. Requested was (2560, 1600), maximum extent for either dimension is
> 2048.

## Root cause (as discovered during implementation)

The original design assumed the relevant limit was the *adapter's*
nominal `max_texture_dimension_2d`. That assumption is incorrect.

The wgpu 27 surface-configure check is in
`wgpu-core/src/device/global.rs::validate_surface_configuration` and
validates against `device.limits.max_texture_dimension_2d` — the
**device's** limit, not the adapter's. In `WindowState::new` the
device is created with `required_limits: wgpu::Limits::downlevel_defaults()`,
which in wgpu 27 sets `max_texture_dimension_2d: 2048` (see
`wgpu-types-27/src/lib.rs::downlevel_defaults`). On a Retina display
(`LogicalSize(1280, 800)` × scale factor 2.0 = `PhysicalSize(2560, 1600)`),
the requested surface exceeds the device's 2048 limit and wgpu panics
with the error above.

On the affected machine the *adapter* reports
`max_texture_dimension_2d: 16384`, but the *device* is capped at 2048
by the downlevel-defaults constructor. This is the gap the original
spec failed to model.

**Fix (A-scheme):** Clamp the surface size to
`wgpu::Limits::downlevel_defaults().max_texture_dimension_2d` (= 2048
in wgpu 27) — the same value wgpu actually checks against. The
behavior is therefore "the surface is always ≤ 2048", which means the
clamp engages whenever the window's physical size exceeds 2048 in
either axis. On a non-HiDPI display (scale 1.0) with the default
`LogicalSize(1280, 800)`, the surface stays at 1280×800 (no clamp). On
a Retina display, it clamps to 2048×1600.

## Purpose

Fix the startup panic by clamping the surface size to the device's
effective `max_texture_dimension_2d` before passing it to
`Surface::configure`. Apply the same clamp on `WindowState::resize` so
runtime resizes cannot re-trigger the same panic.

## Motivation

The current code in `WindowState::new` and `WindowState::resize` writes
`window.inner_size()` directly into `SurfaceConfiguration` without
checking the device's `max_texture_dimension_2d`. On a Retina display at
the default `LogicalSize(1280, 800)` (scale factor 2.0), the physical
size is 2560×1600. The device is created with `downlevel_defaults`, so
its effective `max_texture_dimension_2d` is 2048, and wgpu rejects the
configure call — treating it as a fatal error, the process panics, and
the user can never see the panorama.

This bug was not caught by existing tests because the unit suite is
pure-logic (camera, sphere, file IO) and `WindowState` requires a real
window + GPU to construct. It is also not covered by the manual
`TESTING.md` checklist.

## Goals

1. **Eliminate the panic** in `WindowState::new` when the window's
   physical pixel size exceeds the device's effective
   `max_texture_dimension_2d` (2048 in our case).
2. **Eliminate the same panic in `WindowState::resize`** so dragging to
   a 4K display, or enlarging the window past the limit, does not crash.
3. **Make the fix unit-testable** without a winit/wgpu runtime, by
   isolating the clamp logic in a pure function.
4. **Clamp only the offending axis** — when only one of (width, height)
   exceeds the device's max, leave the other axis unchanged. This is
   per-axis `min`, not a uniform scale. It satisfies wgpu (any axis >
   max is fixed) while minimising the visual impact (the side that
   *was* within budget is preserved at its original pixel count).
   Aspect ratio *may* change in the corner case where both axes
   overflow (e.g. 2560×1600 on max=2048 → 2048×1600, aspect 1.6 → 1.28);
   the camera projection reads the new (clamped) surface size via
   `ws.aspect()` in `app.rs:222`, so rendering remains internally
   consistent — it is just rendered into a slightly different aspect.
5. **No new dependencies, no API breakage, no behavior change** for
   windows whose physical size fits within the device's
   `max_texture_dimension_2d`. On a non-HiDPI display with the default
   1280×800 logical size, the surface is configured at 1280×800 exactly
   (no clamp engaged); the change is invisible.

## Non-Goals (YAGNI)

- Not upgrading `wgpu` 27 → 29 (out of scope; deferred to a separate spec).
- Not resizing the winit window itself (P2 in brainstorming). Only the
  surface is clamped; the user-chosen window size is left alone.
- Not adding an in-app "GPU too small" warning banner.
- Not changing CI workflows, not adding GPU matrix tests.
- Not bumping the crate version. The fix lands in `[Unreleased]`.

## Approach

Introduce a pure helper `clamp_surface_size` in `window.rs`, and call it
from both `WindowState::new` and `WindowState::resize` before building or
re-building the `SurfaceConfiguration`.

### Why a pure helper (and not inline clamp or a `wgpu::Limits` extension)

| Option | Testability | Verdict |
|---|---|---|
| Pure `fn clamp_surface_size(PhysicalSize, u32) -> PhysicalSize` | Trivial, no wgpu | **Chosen** |
| `fn max_surface_extent(&wgpu::Adapter) -> u32` then inline | Requires mocking `Adapter` in tests (wgpu 27 has no mock API) | Rejected |
| Inline `min(...)` at both call sites | Duplication; no unit-testable seam | Rejected |

The helper takes `max_extent: u32` (not `&Adapter`) precisely so it can
be exercised by `cargo test` with no GPU present.

## Architecture

### Files changed

| File | Change |
|---|---|
| `crates/pano-viewer/src/window.rs` | Add `fn clamp_surface_size`; call it in `new` and `resize`; add `#[cfg(test)] mod tests` with 6 cases |
| `TESTING.md` | Append a clause to Scenario 11; add new Scenario 13 |
| `CHANGELOG.md` | Add `### Fixed` block under `[Unreleased]` |

No other source file changes. `main.rs`, `app.rs`, `error.rs`, `file.rs`,
`renderer.rs`, `ui.rs`, `input.rs` are untouched. `Cargo.toml` is
untouched. `Cargo.lock` may pick up a Cargo-generated metadata
re-serialization from `cargo build`, but no dependency versions or
graphs change (no `cargo add`, no `cargo update`).

### Function shape

```rust
/// Clamp a window's physical pixel size to the GPU device's effective
/// maximum supported texture dimension. wgpu rejects `Surface::configure`
/// when either dimension exceeds the *device's* `max_texture_dimension_2d`
/// (see `wgpu-core/src/device/global.rs::validate_surface_configuration`),
/// which can happen on HiDPI displays (e.g. 1280×800 logical @ 2x scale =
/// 2560×1600) when the device is created with restrictive
/// `required_limits`. This is a pure function so it can be unit-tested
/// without winit/wgpu.
fn clamp_surface_size(
    size: winit::dpi::PhysicalSize<u32>,
    max_extent: u32,
) -> winit::dpi::PhysicalSize<u32> {
    winit::dpi::PhysicalSize::new(
        size.width.min(max_extent).max(1),
        size.height.min(max_extent).max(1),
    )
}
```

The function is module-private (not `pub`) — single caller, YAGNI.

### Call-site changes

**`WindowState::new`** (currently lines 69-78 in the post-fix code):

```rust
let size = window.inner_size();
// The device is created with `required_limits:
// wgpu::Limits::downlevel_defaults()`, so the device's effective
// `max_texture_dimension_2d` is `downlevel_defaults().max_texture_dimension_2d`
// (2048 in wgpu 27), not whatever the adapter nominally reports.
// wgpu's `validate_surface_configuration` (see
// `wgpu-core/src/device/global.rs`) checks against
// `device.limits.max_texture_dimension_2d`, so this is the bound
// we must clamp to in order to avoid a panic on HiDPI displays
// (e.g. 1280×800 logical @ 2x = 2560×1600).
let max_extent = wgpu::Limits::downlevel_defaults().max_texture_dimension_2d;
let size = clamp_surface_size(size, max_extent);             // NEW (shadows `size`)
let config = wgpu::SurfaceConfiguration {
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    format: surface_format,
    width: size.width,     // (was .max(1); removed because clamp guarantees ≥ 1)
    height: size.height,   // (was .max(1); removed because clamp guarantees ≥ 1)
    present_mode: surface_caps.present_modes[0],
    alpha_mode: surface_caps.alpha_modes[0],
    view_formats: vec![],
    desired_maximum_frame_latency: 2,
};
```

The `max_extent` source is the device's *constructor-time* limit
(`wgpu::Limits::downlevel_defaults()`), not `adapter.limits()`. The two
are not the same in general: `required_limits` lets us set a device
limit that is ≤ the adapter's nominal limit, and wgpu-core validates
against the device limit, not the adapter limit. Reading the
constructor constant directly keeps the call site independent of
`adapter` and matches the value the device will end up with.

The pre-existing `.max(1)` on `width`/`height` in `new` is removed
because `clamp_surface_size` already guarantees the output is at least
1×1. Keeping the `.max(1)` would be redundant but harmless; removing it
is a small cleanup that signals the invariant is now centralized in
`clamp_surface_size`.

**`WindowState::resize`** (currently lines 100-108 in the post-fix code):

```rust
pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    if new_size.width == 0 || new_size.height == 0 {
        return;
    }
    // The device was created with `required_limits:
    // wgpu::Limits::downlevel_defaults()`, so its effective
    // `max_texture_dimension_2d` is the constant from that constructor
    // (2048 in wgpu 27). Reading via `self.device.limits()` keeps this
    // call site consistent with `new()` even if the constructor
    // arguments are later changed.
    let max_extent = self.device.limits().max_texture_dimension_2d;
    let clamped = clamp_surface_size(new_size, max_extent);
    self.config.width = clamped.width;
    self.config.height = clamped.height;
    self.surface.configure(&self.device, &self.config);
}
```

The early return on 0×0 is kept as-is — it's a winit-level concern
(avoid configure churn during minimize) orthogonal to the clamp.

### Why two different `max_extent` sources (and why both are correct)

- **`new` uses `wgpu::Limits::downlevel_defaults().max_texture_dimension_2d`:**
  the constructor constant. This is the value the device will be
  created with, and it is independent of the adapter (which we still
  have in scope but is not the right thing to read).
- **`resize` uses `self.device.limits().max_texture_dimension_2d`:**
  the device's runtime-reported limit. Since the device was created
  with `downlevel_defaults`, this is the same `u32` value the
  constructor chose.

Both sources converge to the same number (2048 in wgpu 27). They
differ in *where the value comes from* — one is the request, the other
is the realized device — which is why both call sites are written the
way they are. A single `const` could replace both, but the
documentation cost of explaining "the same magic number, in two
places" outweighs the savings.

## Data flow

### Startup

```
create_window(LogicalSize 1280×800)
  → instance.create_surface(window)
  → request_adapter → adapter
  → adapter.request_device(required_limits: downlevel_defaults()) → device
        [device.limits.max_texture_dimension_2d := 2048]
  → wgpu::Limits::downlevel_defaults().max_texture_dimension_2d   [read 2048]
  → window.inner_size()  →  PhysicalSize{2560, 1600}  (Retina 2x)
  → clamp_surface_size({2560,1600}, 2048)  →  {2048, 1600}  [per-axis min]
  → SurfaceConfiguration{ width: 2048, height: 1600, ... }
  → surface.configure(&device, &config)              [accepted]
```

In the 2560×1600 case, only the width axis (2560) overflows the 2048
limit; the height axis (1600) is within budget and is preserved
unchanged. Result: 2048×1600, aspect changes from 1.6 to 1.28.

### Resize

```
winit dispatches WindowEvent::Resized(PhysicalSize{w, h})
  → app.rs calls ws.resize(PhysicalSize{w, h})        [app.rs unchanged]
  → self.device.limits().max_texture_dimension_2d     [read 2048, same as new]
  → clamp_surface_size({w,h}, max)
  → self.config.width/height = clamped
  → self.surface.configure(&self.device, &self.config)
```

The implicit re-configure path in `app.rs:render_frame` for
`SurfaceError::Lost | Outdated` (calls `ws.resize(ws.window.inner_size())`)
is automatically protected by the new clamp in `resize`.

## Error handling

| Situation | Before | After |
|---|---|---|
| Initial surface > max | **panic** | Clamped, launches |
| User enlarges window past max | **panic** | Clamped, winit window stays at user-chosen size |
| `inner_size() == 0×0` (theoretical) | `new` had `.max(1)` | `new` drops `.max(1)` (clamp covers it); `resize` still early-returns |
| `max_extent == 0` (impossible) | n/a | Defensive `.max(1)` keeps output ≥ 1×1 |
| `clamp_surface_size` itself fails | n/a | Cannot — pure function |

No new `AppError` variants. No failure mode is added.

## User-visible behavior (P1 strategy)

Per the brainstorming decision (P1: "clamp surface only, do not resize
winit window"):

- **The app launches** instead of panicking.
- **The winit window keeps its user-chosen size** (e.g. 1280×800 logical).
- **The wgpu surface is configured to the clamped size** (e.g. 2048×1600
  physical on a 2560×1600 Retina window — width clamped, height
  preserved because it was already within budget).
- **The non-offending axis is preserved at its original pixel count.**
  Aspect ratio *may* change in the corner case where both axes
  overflow (1.6 → 1.28 in the example). The camera projection in
  `app.rs` (`ws.aspect()`) is computed from the new (clamped) surface
  size at `app.rs:222`, so rendering is internally consistent — just
  rendered into a slightly different aspect. On a 13" Retina this is
  the order of one horizontal cm of content shift, not a "stretched
  image" in any user-noticeable sense.
- **Visual artifact**: when the clamp engages, only the overflowing
  axis shrinks. In the 2560×1600 → 2048×1600 case the X axis is 80%
  of the window's width (a ~20% gap on one side) but the Y axis
  matches the window exactly (no vertical margin). The egui content
  is rendered into the surface, which is centered; the user's
  perception is a slight horizontal margin, not a "stretched image".
- **winit is unaware of the clamp**, so no extra `Resized` events are
  generated.

## Testing

### Strategy

Pure-function unit tests in `crates/pano-viewer/src/window.rs` under a
new `#[cfg(test)] mod tests`. No winit/wgpu needed at test time (the
`winit` crate itself is still compiled because it is a normal
dependency, but no window/event loop is created).

### Cases (6 total)

| # | Name | Input | Expected | What it locks down |
|---|---|---|---|---|
| 1 | `clamp_under_max_is_identity` | `(1920×1080, 2048)` | `(1920, 1080)` | Below max → unchanged |
| 2 | `clamp_over_max_caps_both_axes` | `(2560×1600, 2048)` | `(2048, 1600)` | The actual bug case; per-axis min caps width to 2048, leaves height 1600 unchanged |
| 3 | `clamp_caps_only_offending_axis_when_one_axis_over` | `(4096×1024, 2048)` | `(2048, 1024)` | Per-axis `min`: only width capped, height (1024) was already within budget |
| 4 | `clamp_zero_returns_one` | `(0×600, 2048)` | `(1, 600)` | `.max(1)` on the width |
| 5 | `clamp_with_max_zero_returns_one` | `(800×600, 0)` | `(1, 1)` | Defensive `.max(1)` on both axes |
| 6 | `clamp_exact_max_is_identity` | `(2048×2048, 2048)` | `(2048, 2048)` | Boundary: equal to max is not over-clamped |

### TDD order

1. Write all 6 tests → `cargo test -p pano-viewer window::tests` (expect 6 failures
   — the function is not yet defined).
2. Write `clamp_surface_size` → `cargo test -p pano-viewer window::tests` (expect
   6 pass, prior tests still pass).
3. Wire it into `new` and `resize` → `cargo build` and `cargo test -p
   pano-viewer` to confirm nothing else broke.

### Coverage gaps (manual, not automated)

These **cannot** be exercised in `cargo test` because they need a real
window + GPU. They are the Definition of Done manual checks:

- Launch on a low-max adapter → no panic, panorama renders.
- Drag the window past the limit at runtime → no panic, surface
  reconfigures, content remains visible.
- Resize the window back below the limit → no panic.

## Documentation changes

### `TESTING.md`

Append to Scenario 11 (Window resize):

```markdown
- [ ] On adapters with a small `max_texture_dimension_2d`, the surface
      is clamped to that limit (the window itself is left at the user's
      chosen size).
```

Add Scenario 13 at the end:

```markdown
### 13. Low-max-texture GPU compatibility
- [ ] On a GPU where the adapter's `max_texture_dimension_2d` is smaller
      than the window's physical pixel size (e.g. ≤2048), the app launches
      successfully instead of panicking with a wgpu validation error.
- [ ] The panorama still displays and the camera aspect ratio is correct.
```

### `CHANGELOG.md`

Add under `[Unreleased]`:

```markdown
### Fixed
- Surface configuration no longer panics on GPUs whose
  `max_texture_dimension_2d` is smaller than the window's physical size
  (e.g. low-power iGPUs, 1280×800 logical @ 2x Retina = 2560×1600 on
  adapters capped at 2048). The window/surface size is now clamped to
  the adapter's limit in both `WindowState::new` and `WindowState::resize`.
```

## Risk

| Risk | Likelihood | Mitigation |
|---|---|---|
| `wgpu::Limits::max_texture_dimension_2d` field absent in wgpu 27.0.1 | Very low (stable since wgpu 0.20) | `cargo build` errors immediately |
| `device.limits()` expensive | Zero — `Limits` is plain data, copy is cheap; `resize` is not on the hot path | n/a |
| Other side effects on 4K displays | Medium (content center shifts by sub-cm) | Documented in P1 trade-off; TESTING.md Scenario 13 describes actual behavior |
| One of the 24 existing unit tests breaks | Low (pure-additive change) | `cargo test` full run before commit |

## Definition of Done

- [ ] `cargo build --release` succeeds.
- [ ] `cargo test` passes (6 new + 24 prior = 30 total).
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
      (project has maintained zero-warning policy since v0.2.0).
- [ ] Manual: `target/release/pano-viewer ./Qwen-Image-2512_00001_.png`
      launches on the affected machine, panorama renders, drag/wheel/
      resize all behave.
- [ ] `CHANGELOG.md` and `TESTING.md` updated.
- [ ] Commit message: `fix(window): clamp surface size to adapter's
      max_texture_dimension_2d`.
