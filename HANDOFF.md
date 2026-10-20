# Fragile handoff

Fragile is slot v1.8.0 plus features ported from the community forks. The full plan is in
`FRAGILE_PLAN.md`. Work happens on the `fragile` branch.

## State when moved off the Windows PC (2026-10-20)

The last commit is a **work in progress that has never been compiled**. The Windows PC ran out
of disk space and WSL crashed before the first test run finished. The first job on the Linux PC is
to build it and fix whatever fails.

Done in Phase 0:
- The README lists the RG35XX Pro. `forks/`, `BaseOS/` and `release/` are gitignored. They are
  reference material only, so copy them across by hand if you want them on the new machine.

Phase 1, written but not compiled:
- `QuickRow::help()` in `crates/slot-ui/src/quick_menu.rs` gives every row a one-line help.
  `QuickMenuFaces::descriptions` is now one entry per row (`Vec<Option<_>>`), built in
  `frontend.rs` from `QuickRow::ALL`. `App::description` in `app.rs` looks the entry up and skips
  the in-game screen panel. The test fixtures in `crates/slot/tests/{quick_menu,in_game_menu}.rs`
  were updated. The new tests are at the end of `crates/slot-ui/tests/quick_menu.rs`.
- `crates/slot-ui/src/keyboard.rs` is the on-screen keyboard ported from nickclyde. It adds
  upper, lower and symbol layers, a Shift key, a length cap and password masking. Tests are in
  `crates/slot-ui/tests/keyboard.rs`. Nothing is wired into App yet.
- `crates/slot-store/src/extras.rs` adds `Config/extras.state` for Fragile-only settings. Every
  key is optional and unknown keys are kept. Tests are in `crates/slot-store/tests/extras.rs`.
- `crates/slot-gfx/src/headless_linux.rs`, ported from roygravy, is an EGL surfaceless surface,
  so the render tests (`HeadlessSurface`) build and run on Linux. Upstream only had the macOS CGL
  surface.

Done since, on Linux:
- `crates/slot-ui/src/list_page.rs`: `ListPage` holds the rows and the cursor. Up and Down wrap,
  L1 and R1 page, and `set_rows` keeps the cursor on the same label. `ListPageView` draws it
  with QuickMenu's pitch, pointer, ground, help line and legends, plus scroll carets
  (`quick_scroll_face`, the side carets turned, because the symbols font is a subset).
  Labels shrink to fit and are then cut, with no ellipsis.
- `crates/slot/src/text_faces.rs`: `TextFaces`, the fixed pool of run-time text textures (an LRU
  updated in place, since textures are never freed). Call `begin_frame` once a frame.
- Tests: `slot-ui/tests/list_page.rs`, `slot-ui/tests/render_list_page.rs` (GL, in-process),
  `slot/tests/text_faces.rs`.

Neither the list page nor the keyboard is wired into App or frontend yet. The first real
user is Phase 5 (button cheat sheet) or Phase 8 (cheats).
Phase 1 item not done: scroll arrows on the long QuickMenu pages (Controls, Shortcuts,
General). `quick_scroll_face` is ready for that.

## First build on the Linux PC (Omarchy, 2026-10-20)

The Phase 1 work compiled first time and `task test` passes, with clippy `-D warnings` clean.
- The render suites (`crates/slot/tests/render*.rs`, `slot-ui/tests/render_row_edges.rs`) were
  still gated `#![cfg(target_os = "macos")]`. They now run on Linux too, through the EGL surface.
- One of those suites failed on Linux: in `render_in_game_menu`, the final check compared the panel
  with one game pixel above it, and the mock pattern happens to be dark there. It now compares
  the panel with the bare game at the same pixel.
- `cargo fmt --all` was applied.
- `slot::emu a_stalled_display_does_not_stall_the_worker` is timing-sensitive. It can fail under a
  bare `cargo nextest run --workspace`, but `task test` runs it in-process and it passes.

## Linux setup

```
sudo apt install -y build-essential pkg-config cmake git curl libasound2-dev libudev-dev \
  libx11-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev libegl1-mesa-dev \
  libgles2-mesa-dev mesa-utils xvfb
curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
curl -sL https://taskfile.dev/install.sh | sh -s -- -d -b ~/.local/bin
curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ~/.cargo/bin
tools/fragile/fr.sh task test
```

The taskfile is macOS-centric:
- `task core` builds `vendor/mgba_libretro.dylib` and checks for Mach-O, so it fails on Linux.
- On Linux, `vendor/mgba_libretro.so` is the aarch64 device core.
- Without a host core, the core tests fall back to the mock or skip.
- Making `core:*:host` Linux-aware (for example `vendor/host/*.so`) is an open task.
