# Surface Max-Texture Clamp — Design

**Date**: 2026-07-07
**Project**: `equirectangular-panorama-viewer` (crate `pano-viewer`)
**Status**: Approved for implementation (awaiting plan)
**Trigger**: `cargo run --release -- ./Qwen-Image-2512_00001_.png` panics on
certain macOS configurations with a wgpu validation error:

> `Surface` width and height must be within the maximum supported texture
> size. Requested was (2560, 1600), maximum extent for either dimension is
> 2048.

## Purpose

Fix the startup panic on GPUs whose `max_texture_dimension_2d` is smaller
than the window's physical pixel size. The fix must also apply to runtime
resizes (the same panic would re-occur if the user dragged the window to a
4K display or simply enlarged it past the adapter's limit).

## Motivation

The current code in `WindowState::new` and `WindowState::resize` writes
`window.inner_size()` directly into `SurfaceConfiguration` without checking
the adapter's `max_texture_dimension_2d`. On a Retina display at the
default `LogicalSize(1280, 800)` (scale factor 2.0), the physical size is
2560×1600. Adapters capped at 2048 (low-power iGPUs, some virtualized
GPUs, some remote-display scenarios) reject the configure call, and wgpu
treats that as a fatal error, so the process panics and the user can
never see the panorama.

This bug was not caught by existing tests because the unit suite is
pure-logic (camera, sphere, file IO) and `WindowState` requires a real
window + GPU to construct. It is also not covered by the manual
`TESTING.md` checklist.

## Goals

1. **Eliminate the panic** in `WindowState::new` for adapters where
   `window.inner_size() > adapter.limits().max_texture_dimension_2d`.
2. **Eliminate the same panic in `WindowState::resize`** so dragging to
   a 4K display, or enlarging the window past the limit, does not crash.
3. **Make the fix unit-testable** without a winit/wgpu runtime, by
   isolating the clamp logic in a pure function.
4. **Clamp only the offending axis** — when only one of (width, height)
   exceeds the adapter's max, leave the other axis unchanged. This is
   per-axis `min`, not a uniform scale. It satisfies wgpu (any axis >
   max is fixed) while minimising the visual impact (the side that
   *was* within budget is preserved at its original pixel count).
   Aspect ratio *may* change in the corner case where both axes
   overflow (e.g. 2560×1600 on max=2048 → 2048×1600, aspect 1.6 → 1.28);
   the camera projection reads the new (clamped) surface size via
   `ws.aspect()` in `app.rs:222`, so rendering remains internally
   consistent — it is just rendered into a slightly different aspect.
5. **No new dependencies, no API breakage, no behavior change** for
   adapters whose limit is larger than the window.

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
/// Clamp a window's physical pixel size to the GPU adapter's maximum
/// supported texture dimension. wgpu rejects `Surface::configure` when
/// either dimension exceeds `adapter.limits().max_texture_dimension_2d`,
/// which can happen on HiDPI displays (e.g. 1280×800 logical @ 2x scale
/// = 2560×1600) or on adapters with low texture limits. This is a pure
/// function so it can be unit-tested without winit/wgpu.
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

**`WindowState::new`** (currently lines 53-58):

```rust
let size = window.inner_size();
let max_extent = adapter.limits().max_texture_dimension_2d;  // NEW
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

The pre-existing `.max(1)` on `width`/`height` in `new` is removed
because `clamp_surface_size` already guarantees the output is at least
1×1. Keeping the `.max(1)` would be redundant but harmless; removing it
is a small cleanup that signals the invariant is now centralized in
`clamp_surface_size`.

**`WindowState::resize`** (currently lines 75-82):

```rust
pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    if new_size.width == 0 || new_size.height == 0 {
        return;
    }
    let max_extent = self.device.limits().max_texture_dimension_2d;  // NEW
    let clamped = clamp_surface_size(new_size, max_extent);          // NEW
    self.config.width = clamped.width;
    self.config.height = clamped.height;
    self.surface.configure(&self.device, &self.config);
}
```

The early return on 0×0 is kept as-is — it's a winit-level concern
(avoid configure churn during minimize) orthogonal to the clamp.

### Why `self.device.limits()` in `resize` (and not `adapter`)

`adapter` is consumed inside `WindowState::new` and is not retained on
the struct. The device's limits are fixed at creation time and are
derived from the adapter that created it, so `self.device.limits()` is
guaranteed to equal the `adapter.limits()` value captured in `new`. The
alternative — storing `max_extent: u32` on `WindowState` — would couple
the struct to a value the device already knows; reading from the device
keeps the data source single.

## Data flow

### Startup

```
create_window(LogicalSize 1280×800)
  → instance.create_surface(window)
  → request_adapter → adapter
  → adapter.limits().max_texture_dimension_2d       [read max]
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
  → self.device.limits().max_texture_dimension_2d     [read max]
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
- **Visual artifact**: when the clamp engages, the surface is ~20%
  smaller in each axis than the window's physical size, so egui content
  is centered with a sub-pixel-to-a-few-pixels margin. On a 13"
  Retina this is on the order of 0.5cm — practically invisible.
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
