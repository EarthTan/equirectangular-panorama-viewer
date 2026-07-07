# Surface Max-Texture Clamp Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the `wgpu` panic at startup when the window's physical pixel size exceeds the device's effective `max_texture_dimension_2d` (e.g. 1280×800 logical @ 2x Retina = 2560×1600 on a device created with `wgpu::Limits::downlevel_defaults()`, which sets the device limit to 2048 in wgpu 27). The same fix protects against runtime resizes that cross the limit.

**Architecture:** Add a pure helper `clamp_surface_size` in `window.rs`. Call it from `WindowState::new` (using the constructor-time `wgpu::Limits::downlevel_defaults().max_texture_dimension_2d` as the bound) and from `WindowState::resize` (using `self.device.limits().max_texture_dimension_2d` — the same value at runtime). The helper is unit-testable with no winit/wgpu runtime by taking `max_extent: u32`.

**Tech Stack:** Rust 1.88, wgpu 27.0.1, winit 0.30.13, thiserror 2.

**Spec:** `docs/superpowers/specs/2026-07-07-surface-max-clamp-design.md` (commit `b4c6875`, with the A-scheme corrections applied after manual smoke-test discovery).

## Global Constraints

- **Rust MSRV**: 1.88 (from spec; `rust-version = "1.88"` in workspace `Cargo.toml`).
- **wgpu**: 27.0.1, `wgpu-core` 27.0.3, `winit` 0.30.13 (locked in `Cargo.lock`; do not run `cargo update`).
- **No new dependencies.** Do not modify `Cargo.toml`. `Cargo.lock` may receive a Cargo-generated metadata re-serialization from `cargo build`, but no dependency versions or graph change.
- **No source changes outside `window.rs`.** `main.rs`, `app.rs`, `error.rs`, `file.rs`, `renderer.rs`, `ui.rs`, `input.rs` are untouched by this plan.
- **Zero clippy warnings.** Project policy since v0.2.0; final run must use `cargo clippy --all-targets --all-features -- -D warnings`.
- **Existing 24 unit tests must still pass.** Final `cargo test` count: 6 new + 24 prior = 30 total.
- **Commit message style:** conventional commits, lowercase `type(scope): summary`; examples already in history: `fix(window): use PRIMARY backends (Vulkan) instead of all` (commit `f8062d3`).

---

## File Structure

| File | Change | Role |
|---|---|---|
| `crates/pano-viewer/src/window.rs` | Add `fn clamp_surface_size`; call in `new` and `resize`; add `#[cfg(test)] mod tests` with 6 cases; remove redundant `.max(1)` in `new` | The only code change |
| `TESTING.md` | Append clause to Scenario 11; add Scenario 13 | Manual verification coverage |
| `CHANGELOG.md` | Add `### Fixed` block under `[Unreleased]` | Release notes |

No new files in `src/`. No restructuring of `window.rs` (function added at module level next to `WindowState`).

---

## Task 1: Add `clamp_surface_size` helper with TDD coverage

**Files:**
- Modify: `crates/pano-viewer/src/window.rs:1-91` (add function near top of file, add `mod tests` at bottom)
- Test: `crates/pano-viewer/src/window.rs` (same file, in `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing (first task)
- Produces:
  - `fn clamp_surface_size(size: winit::dpi::PhysicalSize<u32>, max_extent: u32) -> winit::dpi::PhysicalSize<u32>`
  - Behavior: per-axis `min` with `max_extent`, then `.max(1)` on each axis (defensive). See spec §"Function shape" for full body.

**TDD order:** write all 6 tests first → run to confirm red → write the function → run to confirm green → commit.

- [ ] **Step 1.1: Read current `window.rs` to confirm exact line numbers**

Read: `crates/pano-viewer/src/window.rs`.

Confirm the current state matches what the spec assumes (lines 1-91, function `WindowState::new` at lines 14-73, `resize` at lines 75-82). If anything has drifted, stop and re-derive line numbers before continuing.

- [ ] **Step 1.2: Write the 6 failing tests at the bottom of `window.rs`**

Append the following block at the end of `crates/pano-viewer/src/window.rs` (after the last line, line 91). Do not modify any existing code in this step.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalSize;

    #[test]
    fn clamp_under_max_is_identity() {
        let s = PhysicalSize::new(1920, 1080);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 1920);
        assert_eq!(out.height, 1080);
    }

    #[test]
    fn clamp_over_max_caps_both_axes() {
        // The actual bug case: 1280x800 logical @ 2x Retina on a max=2048 device.
        // Per-axis min caps width to 2048; height (1600) is within budget and is
        // preserved unchanged. Aspect shifts from 1.6 to 1.28 in this corner case.
        let s = PhysicalSize::new(2560, 1600);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 1600);
    }

    #[test]
    fn clamp_caps_only_offending_axis_when_one_axis_over() {
        // Only width overflows; height (1024) is within budget and preserved.
        let s = PhysicalSize::new(4096, 1024);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 1024);
    }

    #[test]
    fn clamp_zero_returns_one() {
        let s = PhysicalSize::new(0, 600);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 1);
        assert_eq!(out.height, 600);
    }

    #[test]
    fn clamp_with_max_zero_returns_one() {
        let s = PhysicalSize::new(800, 600);
        let out = clamp_surface_size(s, 0);
        assert_eq!(out.width, 1);
        assert_eq!(out.height, 1);
    }

    #[test]
    fn clamp_exact_max_is_identity() {
        let s = PhysicalSize::new(2048, 2048);
        let out = clamp_surface_size(s, 2048);
        assert_eq!(out.width, 2048);
        assert_eq!(out.height, 2048);
    }
}
```

- [ ] **Step 1.3: Run the tests and confirm they fail (RED)**

Run from repo root:
```bash
cargo test -p pano-viewer window::tests
```

**Note:** do NOT use `--lib`. `pano-viewer` is a binary crate and has
no library target; `cargo test -p pano-viewer --lib` errors out with
`no library targets found in package 'pano-viewer'`.

Expected: **6 failures**, each with a compile error like `error[E0425]: cannot find function 'clamp_surface_size' in this scope` (or a linker / "function not found" error after the test module compiles but the function is missing). All 24 prior tests still pass.

If fewer than 6 fail or any pass, stop — something is wrong with the test code.

- [ ] **Step 1.4: Write the `clamp_surface_size` function (GREEN)**

Insert the following function **above** the `impl WindowState` block (i.e. between the `use` statements at the top and `pub struct WindowState`). The current `use` block ends at line 3, and `pub struct WindowState` is at line 5 — insert after line 3.

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

- [ ] **Step 1.5: Re-run the new tests and confirm they pass (GREEN)**

Run:
```bash
cargo test -p pano-viewer window::tests
```

Expected: **6 passed, 0 failed** within the `window::tests` module. Output should show all 6 test names from Step 1.2.

- [ ] **Step 1.6: Run the full test suite to confirm no regression**

Run:
```bash
cargo test -p pano-viewer
```

Expected: **30 passed, 0 failed** (24 prior + 6 new). If any prior test fails, stop — the helper must not affect existing behavior.

- [ ] **Step 1.7: Commit Task 1**

```bash
git add crates/pano-viewer/src/window.rs
git commit -m "test(window): add failing tests for clamp_surface_size" -m "" -m "Will be followed by the implementation in a separate commit per TDD discipline."

# Then a second commit for the implementation:
git add crates/pano-viewer/src/window.rs
git commit -m "feat(window): add clamp_surface_size pure helper"
```

**Note:** Per project commit history, commits are split by intent (test vs feat). If the reviewer prefers a single commit `feat(window): add clamp_surface_size with tests`, that is also acceptable — pick one. Default: two commits as shown.

---

## Task 2: Wire `clamp_surface_size` into `WindowState::new` and `WindowState::resize`

**Files:**
- Modify: `crates/pano-viewer/src/window.rs:14-73` (`new` body)
- Modify: `crates/pano-viewer/src/window.rs:75-82` (`resize` body)

**Interfaces:**
- Consumes: `clamp_surface_size` (Task 1), `wgpu::Limits::downlevel_defaults().max_texture_dimension_2d` (the constructor constant used in `new`), `wgpu::Device::limits().max_texture_dimension_2d` (the realized device limit used in `resize`).
- Produces: `WindowState::new` configures a clamped surface; `WindowState::resize` reconfigures with a clamped surface. Both call sites must use the helper.

- [ ] **Step 2.1: Modify `WindowState::new` to call the helper**

In the `new` function, find the block that begins with `let size = window.inner_size();` and ends with `desired_maximum_frame_latency: 2,` (this is the `SurfaceConfiguration` literal). It currently looks like:

```rust
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
```

Replace it with:
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
        let size = clamp_surface_size(size, max_extent);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
```

Two semantic changes:
- `let max_extent = ...` + `let size = clamp_surface_size(...)` added.
- The two `.max(1)` calls on `width` and `height` are **removed** (clamp guarantees ≥ 1; see spec §"Why `self.device.limits()` in `resize`" for rationale).

**Note on line numbers:** the literal line numbers in the original spec and earlier steps will have shifted because Task 1 added the `clamp_surface_size` function and a `#[cfg(test)] mod tests` block to this same file. Use the **text content** of the block above to locate it, not line numbers.

- [ ] **Step 2.2: Modify `WindowState::resize` to call the helper**

Find the `pub fn resize` method body in `WindowState`. It currently looks like:

```rust
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }
```

Replace it with:
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

The early return on 0×0 is **kept as-is** (winit-level concern, orthogonal to clamp; see spec §"Call-site changes").

**Note on line numbers:** same as Step 2.1 — use the text content above, not line numbers.

- [ ] **Step 2.3: Build to confirm the wiring compiles**

Run:
```bash
cargo build -p pano-viewer
```

Expected: compiles cleanly with no warnings. If `cargo build` complains that `max_texture_dimension_2d` is not a field on `wgpu::Limits`, stop and verify the wgpu version matches `Cargo.lock` (`wgpu = "27.0.1"`).

- [ ] **Step 2.4: Run the full test suite**

Run:
```bash
cargo test -p pano-viewer
```

Expected: **30 passed, 0 failed** (no new tests in this task; the 6 from Task 1 should still pass since the helper is unchanged). The 24 prior tests must remain green.

- [ ] **Step 2.5: Run clippy with the project's strict policy**

Run:
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: zero warnings, zero errors. Project has enforced this since v0.2.0.

If clippy suggests style fixes (e.g. `let size = ...` could be `let mut` and never mutated — it isn't, so no fix needed; but if it complains, address only the warning it names, do not refactor).

- [ ] **Step 2.6: Build the release binary**

Run:
```bash
cargo build --release -p pano-viewer
```

Expected: builds successfully. The binary at `target/release/pano-viewer` is the same artifact that was panicking in the original report.

- [ ] **Step 2.7: Commit Task 2**

```bash
git add crates/pano-viewer/src/window.rs
git commit -m "fix(window): clamp surface size to device's max_texture_dimension_2d"
```

---

## Task 3: Documentation, manual smoke test, and final DoD verification

**Files:**
- Modify: `TESTING.md` (Scenario 11 + new Scenario 13)
- Modify: `CHANGELOG.md` (new `### Fixed` block under `[Unreleased]`)

**Interfaces:**
- Consumes: the binary built in Task 2.
- Produces: updated manual test checklist; updated changelog; verified Definition of Done.

- [ ] **Step 3.1: Update `TESTING.md` Scenario 11**

Open `TESTING.md`. Find Scenario 11 ("Window resize"):

```markdown
### 11. Window resize
- [ ] Resize the window.
- [ ] Canvas fills the new size; image is not stretched.
```

Replace it with:

```markdown
### 11. Window resize
- [ ] Resize the window.
- [ ] Canvas fills the new size; image is not stretched.
- [ ] On a device whose effective `max_texture_dimension_2d` is small
      (currently 2048 in wgpu 27 with `downlevel_defaults`), the surface
      is clamped to that limit (the window itself is left at the user's
      chosen size).
```

- [ ] **Step 3.2: Add `TESTING.md` Scenario 13**

Find the end of Scenario 12 in `TESTING.md` ("Clean exit"):

```markdown
### 12. Clean exit
- [ ] Close the window via the OS close button.
- [ ] Process exits; no orphan processes remain.
```

Append the following **after** Scenario 12 (do not modify Scenario 12's body):

```markdown

### 13. Low-max-texture GPU compatibility
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
```

- [ ] **Step 3.3: Update `CHANGELOG.md`**

Open `CHANGELOG.md`. Locate the `[Unreleased]` section header (the file is in Keep a Changelog format per the existing 0.2.0 / 0.3.0 / 0.4.0 entries).

Find the line that begins with `## [Unreleased]`. Immediately under that header (and under any existing `### Added` / `### Changed` sub-headings that may already be present for `[Unreleased]`), add a new `### Fixed` block. If `### Fixed` already exists under `[Unreleased]`, append the bullet to the existing block instead of creating a duplicate.

The text to add (one bullet, wrapped as shown):

```markdown
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
```

- [ ] **Step 3.4: Run the full Definition of Done automated checks**

Run, in order, from the repo root:

```bash
cargo build --release -p pano-viewer
cargo test -p pano-viewer
cargo clippy --all-targets --all-features -- -D warnings
```

Expected:
- `cargo build --release`: succeeds.
- `cargo test`: **30 passed, 0 failed** (24 prior + 6 new). Confirm the count.
- `cargo clippy`: clean, no warnings, no errors.

If any of these fail, stop and fix before proceeding to manual verification.

- [ ] **Step 3.5: Manual smoke test on the affected machine**

The reviewer / user runs the release binary with the bundled panorama:

```bash
target/release/pano-viewer ./Qwen-Image-2512_00001_.png
```

Expected:
- The window opens (no panic, no wgpu validation error in stderr).
- The panorama fills the window.
- Drag with the mouse rotates the camera (horizontal yaw, vertical pitch; pitch stops near ±89°).
- Mouse wheel zooms (FOV clamps at 30° / 100°).
- Resizing the window works; image is not stretched.
- Closing the window exits cleanly (no orphan processes).

If any of these regresses compared to the v0.4.0 baseline behavior, stop and investigate before committing.

- [ ] **Step 3.6: Verify the `Cargo.lock` diff is metadata-only**

Run:
```bash
git diff --stat Cargo.lock
```

Expected: either no changes, or a tiny metadata-only re-serialization. If a dependency **version** or **graph** changed, stop — the plan's global constraints forbid it.

- [ ] **Step 3.7: Commit Task 3**

```bash
git add TESTING.md CHANGELOG.md
git commit -m "docs: document surface max-texture clamp fix"
```

- [ ] **Step 3.8: Confirm clean tree and final commit history**

Run:
```bash
git --no-pager log --oneline -5
git status
```

Expected:
- Top 5 commits show, in order: (Task 3) `docs: document surface max-texture clamp fix`; (Task 2) `fix(window): clamp surface size to device's max_texture_dimension_2d`; (Task 1) `feat(window): add clamp_surface_size pure helper` (and optionally the prior `test(window): add failing tests...` commit); and the spec commit `docs: add spec for surface max-texture clamp` (`b4c6875`).
- `git status`: working tree clean.

---

## Self-Review (per writing-plans skill)

**1. Spec coverage** — cross-referencing `docs/superpowers/specs/2026-07-07-surface-max-clamp-design.md`:

| Spec section / requirement | Covered in |
|---|---|
| `clamp_surface_size` pure helper | Task 1 |
| 6 unit tests (all cases) | Task 1, Step 1.2 |
| Call site in `new` | Task 2, Step 2.1 |
| Call site in `resize` | Task 2, Step 2.2 |
| Redundant `.max(1)` removed in `new` | Task 2, Step 2.1 |
| `TESTING.md` Scenario 11 clause | Task 3, Step 3.1 |
| `TESTING.md` Scenario 13 | Task 3, Step 3.2 |
| `CHANGELOG.md` `### Fixed` | Task 3, Step 3.3 |
| Manual smoke test (DoD) | Task 3, Step 3.5 |
| Clippy zero-warning | Task 2, Step 2.5 (and re-run in Task 3, Step 3.4) |
| `cargo test` 30 passing | Task 1, Step 1.6 and Task 2, Step 2.4 and Task 3, Step 3.4 |
| Cargo.lock metadata-only | Task 3, Step 3.6 |
| Commit message style | Task 1 Step 1.7, Task 2 Step 2.7, Task 3 Step 3.7 |

All spec requirements are covered. ✅

**2. Placeholder scan** — no occurrences of `TBD`, `TODO`, "implement later", "appropriate error handling", "handle edge cases", "similar to Task N", or any step that describes a change without showing the code. ✅

**3. Type / signature consistency**:
- `clamp_surface_size` is defined in Task 1 with signature `(PhysicalSize<u32>, u32) -> PhysicalSize<u32>`. Both call sites in Task 2 use the same signature: `let size = clamp_surface_size(size, max_extent);` (Task 2 Step 2.1) and `let clamped = clamp_surface_size(new_size, max_extent);` (Task 2 Step 2.2). The argument names (`size`, `max_extent`) match the function definition. ✅
- `wgpu::Limits::downlevel_defaults().max_texture_dimension_2d` (Task 2, Step 2.1) and `self.device.limits().max_texture_dimension_2d` (Task 2, Step 2.2) are the same `u32` (2048 in wgpu 27) by construction: the device is created with the same `downlevel_defaults` limits. The two call sites read the value from different sources — one from the constructor constant (independent of any runtime state), the other from the realized device — which is intentional and documented in the spec. ✅
- Tests in Task 1 Step 1.2 use `PhysicalSize::new(w, h)` and `out.width` / `out.height` — these match the `PhysicalSize<u32>` API. ✅
- `cargo test -p pano-viewer window::tests` (no `--lib`) is used in Task 1. `pano-viewer` is a binary crate; `--lib` would error. ✅
- Test #2 (`clamp_over_max_caps_both_axes`) asserts `(2048, 1600)`, which the per-axis `min` function body in Step 1.4 produces. Test #3 (`clamp_caps_only_offending_axis_when_one_axis_over`) asserts `(2048, 1024)`, also consistent with per-axis `min`. Spec Goal #4 in the spec was updated to match. ✅
