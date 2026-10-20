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

Still to do in Phase 1: the dynamic list page widget (`crates/slot-ui/src/list_page.rs`).

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
