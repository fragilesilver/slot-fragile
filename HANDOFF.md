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
- The QuickMenu pages that scroll (Controls, Shortcuts, and General once it has more than six
  rows shown) draw a caret above and below while there are more rows that way
  (`QuickMenuFaces::scroll`).

Phase 1 is done.

## Phase 2 (done 2026-10-21)

- Fixes from nickclyde: the second of two same-named GB/GBC carts on Recents or Favorites
  is the one that goes in (`App::on_shelf_named`), and a cart face finishing after its shelf
  was restocked is dropped and asked for again (`cart_faces.rs`, which now takes the
  `Textures` trait so its tests need no GL).
- `App` keeps `extras` (`Config/extras.state`) through `app/extras.rs`: per-shelf marks
  (`shelf.<rack>`, written on a shelf switch, restart and power off, never while browsing),
  `start_shelf`, `show_favorites` and `clock_12h`. General gains Favorites, Start Shelf and
  Clock, placed so upstream's General tests still pass unchanged.
- The held-direction speed-up was already upstream in 1.8, so nothing was ported.
- About: the sticker stays. A opens Details, the first `Overlay` (`app/overlay.rs`, the one
  `overlay` field on App). It lists Version (`build_info::SLOT_RELEASE`, held to CHANGELOG.md
  by a test), Build, BaseOS and Device (from `/etc/baseos-release`, so not shown on the PC),
  Games, the card the games are on (TF1/TF2 from `/proc/mounts`), the other card when it is
  mounted at `/mnt/system`, and Battery. The frontend uploads the rows through a `TextFaces`
  pool (`sync_overlay`).
- Not checked on the device yet: the TF1/TF2 labels and `/mnt/system`, BaseOS and Device rows,
  and the 640x480 fit of the longest values.
- `CREDITS.md` lists every port so far.

## Between phases 2 and 3 (2026-10-21)

- Sleep Power Off: a short press of power already slept (backlight off, game saved by
  `flush_resume`) and powered off after the Lid Power Off minutes. On a device with no lid
  (`evdev::has_lid`, by BaseOS target, like the left stick) that row is hidden and General
  shows Sleep Power Off, with Never added. It is saved as `sleep_off_min` in extras.state and
  follows `lid_off_min` until it is set. Clamshells are unchanged. Sleep is backlight-only:
  the H700 keeps running, so Never drains the battery. Not measured on the device.
- The About sticker on the RG35XX Pro (`slot-ui/src/sticker_pro.rs`, by BaseOS target) is the
  device's real back label, redrawn from a photo the user took: the ANBERNIC wordmark
  (`wordmark_anbernic.svg`, the first eight letters of upstream's), the model, ratings and
  warnings as printed, and the charge after the battery rating. The marks and outline are
  `sticker_rg35xxpro.svg`, made by `tools/fragile/sticker_rg35xxpro.py`; the code centres the
  print on the label. Every other device keeps upstream's sticker. The Pro label is taller
  than the printed one (`RG35XXPRO_STICKER_H`; regenerate the SVG at that height): under a rule
  it carries upstream's credits said shorter, the copyright, and "Fork in Brunei Darussalam
  by a cat in pink headphone". About draws the sticker at the face's own size.

## Phase 3 (done 2026-10-21)

- Battery (`app/battery.rs`): a banner at 15% and again at 10% (marghost's levels and tests),
  reset by charging and never shown while asleep; upstream still powers off at 5%. General →
  Battery Badge (`battery_badge`, off by default) draws upstream's gauge bottom right while a
  game plays with no menu up. The two banners come after all of upstream's in `Toast::all`.
- Fast forward pitch: `slot/src/pitch.rs` (marghost's WSOLA stretch, unchanged) sits before the
  resampler in `emu.rs` when fast forward has sound and Gameplay → Fast Forward Pitch
  (`ff_pitch`) is on. `tests/emu.rs` measures 440 Hz kept at 4x through the real loop.
- Per-game settings: `slot-store/src/game_profile.rs` (suprsomething's store, per-platform
  defaults, plus `clear`) in `Config/game_profiles.txt`, and `app/profile.rs`. In game,
  Settings → This Game (`quick_menu_extras.rs`) sets Shader, Color Correction, Fast Forward and
  its sound for the seated game, and Use Defaults clears them. The live values go through
  `ff_speed`, `ff_sound`, `screen_shader` and `colour_correction_for`. Once a game keeps its own
  value, the in-game Screen/Gameplay rows and the shader and colour shortcuts change that value,
  not the menu default. A game without its own leaves upstream's behaviour as it was.
- Upstream tests changed only where a page grew: the Gameplay labels, the Settings rows, and
  the list of rows that open.

## Phase 4 (done 2026-10-21)

- Integer Scale (`app/screen.rs`, `slot-gfx` `integer_rect` and `set_integer_scale`): per
  console on its Screen page, 4:3 panels only. GB/GBC at 3x (480x432), GBA at 2x (480x320),
  centred on 640x480; Stretch wins; the SP frame gives way. A card bezel shows at 3x only through
  `<name> 3x.png` beside it (hidden from the bezel list). marghost's Classic GB/GBC bezels ship
  on the card in both sizes (`card/Bezels/`, CC BY-SA, in `licenses/`).
- Card shaders (`slot-gfx/src/retroshader.rs`, markbruno's, `slot-store/src/shaders.rs`): Card
  Shader per console on its Screen page picks a single-pass RetroArch `.glsl` from `Shaders/`,
  drawn instead of the built-in Shader. `PrevTexture` is the frame before; `TexCoord` and
  `InputSize` follow the picture (GB, Stretch, Integer Scale). A file that fails keeps the
  built-in on and shows "Card shader failed to load". Menu rows can show run-time text
  (`App::named_value`, `QuickMenu::named`, the frontend's `sync_named` through `TextFaces`).
- Shipped in `card/Shaders/`: Blend Grid (markbruno), GBA Color (public domain matrix), LCD Grid
  V2 (cgwg, from Baggio94's GLES 2 port), See-Through LCD (written after roygravy's See-Through
  Pixels). `pipeline.rs` compiles and draws every one of them.
- Motion Blur per console (roygravy's way): the cores' own frame blending
  (`mgba_interframe_blending=lcd_ghosting`, `gpsp_frame_mixing`), handed to the running core
  like colour correction. No GPU cost, unlike a ghosting shader.
- Not as `FRAGILE_PLAN.md` had it: no `ShaderPick` enum in `slot.state`. The card shader is its
  own setting in extras.state beside upstream's Shader. Baggio94's presets ship as card shader
  files instead of new built-in modes, and roygravy's ghosting is a core option, not an
  fbo.rs previous-frame pass. Integer Scale, Card Shader and Motion Blur are on each console's
  Screen page, not the in-game Screen page, which upstream's tests pin.
- Not measured on the Mali-G31 yet: the card shaders' frame cost, LCD Grid V2 most of all.

## Phase 5 (done 2026-10-21)

- Button Guide (`app/guide.rs`): a list page of every button, from Controls (between Shortcuts
  and In-Game) and from the in-game Settings (after This Game, where it opens on its In Game
  part). Shortcuts, shelf keys and game buttons come from the bindings as set, so a remap shows
  at once; the gestures nothing remaps are worded from upstream's own guide (`site/index.html`).
- The overlay (`app/overlay.rs`) now opens from the carousel menu and the in-game menu as well
  as About; it knows which, closes with that menu, and B or MENU go back to its row.
- Upstream tests changed only where the Controls and Settings pages grew a row.

## Phase 6 (done 2026-10-21)

- Search (nickclyde's, `app/shelves/search.rs`): X on the shelf opens the keyboard; matches
  count as you type; START or OK puts them on a Search shelf after Favorites, named by the
  query. B on that shelf clears it and goes back; X there edits the last search.
- List shelves (`slot-store/src/collections.rs`): each `Config/Lists/<name>.txt` is a shelf
  after Search, in name order. A line is `GBA/Name`, a bare name, a ROM file name or a path;
  `#` lines are comments. A list with none of the card's games stays off the ring.
- Add to List (Fragile's own, `app/shelves.rs`): MENU+Y opens a page of the lists with Yes/No
  for the cart under the cursor; A puts it on or takes it off (the file is rewritten, other
  lines kept). New List names one on the keyboard and makes the file.
- Both keys are rebindable on Settings > Controls > Shelf (`CarouselAction::Search`,
  `AddToList` in `carousel` config; defaults X and MENU+Y, which clash with nothing). A plain
  shelf key also sends its button, so the press that opens a page is skipped by it.
- Merge guards: `slot/tests/shelves_hooks.rs` lists the one-line hooks in `app.rs`,
  `overlay.rs` and `frontend.rs`.
- Upstream test changed: the Shelf keys label list (two rows more).

## Phase 7 (done 2026-10-21)

Four commits, each with the suite green; no behaviour change for GB, GBC or GBA.
- 7a `slot-store/src/spec.rs` (after unslot): each system is a `Spec` row (name, dir,
  extensions, native, picture, header, cores, shaders, bezels, shoulders, triggers, turbo, rewind,
  link, bios). `Platform::name/dir_name/extensions/picture` and `Core::runs/default_for/parse`
  read it; the GBA-only checks in app.rs and session.rs read its fields. Adding a system is a
  variant, a `COUNT` bump and a row.
- 7b `slot-store/src/platform_settings.rs`: shader, colour correction and bezel per platform.
  The Game Boy family stays in upstream's `slot.state` keys, unchanged (golden file
  `slot-store/tests/golden/slot.state` must read and write back byte for byte); a later system
  keeps them in extras.state (`shader.<dir>` and so on). Not as the plan had it: no map inside
  `SlotState`, so upstream's file and parser are untouched.
- 7c native picture: a core for a `native` system draws into a canvas of its own size; size and
  aspect (`SET_GEOMETRY`/AV info) ride with each frame (`frames::Shape`); `GamePass` remakes its
  textures on a size change and letterboxes to the aspect; thumbnails keep the size;
  `need_fullpath` cores get the path only. Mock core has a 256x224 4:3 mode
  (`MockCore::set_native`).
- 7d buttons: `GameButton` gains X, Y, L2, R2 (after upstream's twelve; `ALL` unchanged,
  `EVERY` has all 16). `GameButton::of` picks the pad from the Spec row; `Pad::extra_mask` adds
  only that pad's extras; the emu's face-button mode sends X and Y as themselves when the
  system has no turbo. The controls file names an extra button only once it is moved. No L3/R3
  (no `Device` for them on the RG35XX family); `ButtonMask` has the bits.
- Core credits: `slot-store/src/core_credits.rs` (a match, so a new core needs a credit to
  build), CREDITS.md "Cores" table, About > Details "Cores" rows; a test checks each licence file
  and CREDITS.md line.
- Left for the phases that use them: disk control and the emu `Disc`/`Reset` jobs (12), cheat
  symbols and `SetCheats` (8), per-system Controls and Screen pages for new systems (12).

Next is Phase 8 (cheats) in `FRAGILE_PLAN.md`.

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
