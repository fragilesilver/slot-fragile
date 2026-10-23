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

## Phase 8 (done 2026-10-21)

- Files (`slot/src/cheats.rs`, roygravy's): `Cheats/<GBA|GB|GBC>/` holds RetroArch `.cht` files
  named like the ROM (a code kind in brackets allowed, else matched by title before the first
  bracket), all merged; also plain `CODE # name` lists as `.txt` there or loose in `Cheats/`
  (the Android port's). Which are on: `Config/Cheats/<dir>/<stem>.txt`, names, gone when none.
  `card/Cheats/README.txt` ships with the card.
- Menu: in-game PLAY page gains Cheats after States, hidden when the game has none or a link
  is up. It opens a list page over the game (`Overlay::Cheats`, `app/cheats.rs`); A turns a
  cheat on or off, B goes back to the row, MENU closes the menu.
- Core: `RetroCore::set_cheats(codes)`; the libretro core resets then sets only the codes that
  are on (mGBA ignores the enabled flag), through optional symbols. The emu keeps the list and
  sets it again after a state load. `App::take_cheats` hands the session the codes whenever
  they differ from what the core has: on core ready, on a toggle, none while linked, back after.
- mGBA: `cores/mgba/zzz-cheat-reset-removes-hooks.patch` (from the Android port) so clearing a
  GBA master code puts the real opcode back instead of leaving the game spinning on the hook.
  Checked here: the patch applies on our pinned mGBA with our other patches and the core builds
  on Linux. gpSP at our pin implements `retro_cheat_set`/`reset` itself (unencrypted codes).
- Not tested here with a real core and ROM, so not yet known to work on device: that codes take
  effect in mGBA and gpSP, and that toggling a master code no longer freezes. No cheat icon on
  the HUD (the plan's roygravy icon); the Cheats row is the only sign.
- Upstream tests changed: the PLAY page rows, the opening rows list, and the in-game hidden
  rows list (Cheats is hidden in those tests' games).

## Phase 9 (done 2026-10-21)

- Main menu gains **Network** (before About): Wi-Fi On/Off (extras `wifi`), Networks, Status,
  IP Address (the three hidden while the Wi-Fi is off) and Time Sync (`app/network.rs`).
- Radio (`slot/src/wifi.rs`, unslot's): a worker thread owns it; `card/System/slotwifi.sh` starts
  wpa_supplicant on wlan0 with `Config/wifi.conf` and a `wpa_cli -a` watcher that runs BaseOS's
  `udhcpc` on connect; scanning, joining (SSID as hex, password checked 8..63 printable or 64
  hex) and forgetting go over the control socket in `/var/run/wpa_supplicant`, never a shell.
  Off the device `SimWifi` stands in (Cafe Guest open, Home-5G password "password").
- Link play: `slotlink.sh` (same as unslot's) writes `/run/slotlink.session` and, for a join,
  stops our supplicant to run its own on wlan0. The worker watches that mark: while it is
  there it asks nothing of the radio (Status says "Link play"); when it goes, the Wi-Fi comes
  back if it was on. `slotwifi.sh up` also waits for the mark to clear.
- Networks page: each network with Connected / Joining / Saved / Strong / Good / Weak /
  Can't join; A joins (open or saved at once, else the keyboard for the password, START or OK
  joins, a bad length says why on the keyboard); X forgets a saved one. Footer: Back, Forget
  (saved only), Join.
- Time Sync (`slot/src/timesync.rs`): `timedatectl set-ntp yes|no`; reads `/data/ntp-enabled` and
  `/run/baseos-ntp-synchronized` for Off / On, waiting / On, synced. Slot's clock is the system
  UTC plus its own offset, so the time zone set on the clock screen is kept.
- The keyboard keeps letter case on its keys and field now (it was all capitals, wrong for a
  lower-case password).
- `Config/wifi.conf` is on the card and wpa_supplicant writes the password into it in plain
  text; FAT has no file modes, so anyone with the card can read it.
- Not done: the Wi-Fi icon in the corner. Not run on the device yet: joining, the DHCP lease,
  reconnecting after a reboot, a link session taking and giving back the radio, NTP syncing.
- Upstream tests changed: the main menu labels and walk, and the opening rows list.

## Phase 10 (done 2026-10-21)

- nickclyde's RetroAchievements, softcore only: `crates/slot-cheevos` (rcheevos v12.5.0 vendored
  with his two patches, `fetch-rcheevos.sh` to refresh it, a C shim, `rc_client` with HTTP on its
  own thread over `ureq` 2 + rustls), `slot-retro/src/memory.rs` (the core's memory map) and
  `slot/src/cheevos/` behind the `cheevos` feature (on for host and device). Hooks:
  `frontend.rs` start and overlay; `emu.rs` loaded, idle, frame, and reset after a state load
  and after the cable restore; guarded by `slot/tests/cheevos_hooks.rs`.
- Not as nickclyde has it: his own Wi-Fi script and `Watcher` are left out; sign-in waits for
  `crate::wifi::online()` (Phase 9's radio: on, connected, not yielded to a link). The user
  agent says Fragile. Fragile adds `Cheevos::achievements()` and `logout()`, and a shim function
  over `rc_client_create_achievement_list`.
- Network page gains RetroAchievements (On/Off, extras `cheevos`, default on; off stops new
  games being followed) and Account (Sign in / Waiting for Wi-Fi / Signing in / the user's
  name / Sign-in failed). A on Account types the username, then the password; both go to
  `Config/retroachievements.txt` (a `host=` line is kept) and the client signs in, signing the
  old account out first. Once the server answers, the password is replaced by its token.
- In game, PLAY gains Achievements after Cheats, hidden until the playing game's achievements
  are known; it lists each with Unlocked or its points. Unlock, mastery, offline and sign-in
  toasts are nickclyde's overlay.
- Pinned to releases at least two weeks old: rustls 0.23.45, rustls-webpki 0.103.15,
  webpki-roots 1.0.9 (newer ones were days old).
- Device: the TLS stack (ring, rustls, ureq) and a cc-built C object were cross-built in the
  same arm64 container the device build uses and the binary ran there. Then `task build:device`
  ran in full on this PC for the first time (Docker `rust:1-bullseye` under qemu, 23 minutes):
  `target-device/device/slot`, aarch64, 8.7 MB. Docker leaves `target-device/` owned by root.
  The cores and fsck are not built yet (`task core:device`, `core:gpsp`, `fsck:exfat`).
- Not run on the device yet: signing in, a game being recognised, an unlock arriving.
- Upstream tests changed: the PLAY page rows, the opening rows list, the in-game hidden rows.

## Phase 11 (done 2026-10-22)

- Gambatte (gambatte-libretro@d9d6cd0, 2026-08-21) is a third core and the default for GB and
  GBC (`Spec.cores = [Gambatte, Mgba]`); GBA is unchanged. Your choice: Gambatte by default,
  mGBA kept for link play. START on a GB cart opens the picker with Gambatte (left) and mGBA
  (right); the picker's two choices now come from the Spec row (`CorePicker::open_among`), the
  board draws those two sockets, and a game's saved choice is read with `core_for_platform`.
- Options (`slot/src/gambatte.rs`, roygravy's): boot ROM on before `retro_init`
  (`LibretroCore::open_with_options`), hardware Auto, GBC colour correction "GBC only" or off
  in accurate mode, front light central, a chosen palette as `internal` + its Gambatte name
  (all 48 map), Motion Blur as `gambatte_mix_frames`. No `gambatte_frameskip` (Gambatte has
  none). Checked against a host build of the pinned core: every value is one it declares, for
  every palette, and a game runs (`tests/gambatte_core.rs`, needs `SLOT_GAMBATTE_HOST`).
- Link play: `Core::links()` is false for Gambatte only. A Gambatte game can open the link
  screen; when the link reloads it, `App::core_to_open` opens it on mGBA from its save file,
  and both cores' resumes become saved states of their own core (`StateRing::keep_resume_as_state`,
  no picture), so neither later rolls the save back. Progress since the last in-game save
  stays in those states.
- Build: `cores/gambatte/build.sh`, `task core:gambatte` (in the arm64 container; built here:
  `vendor/gambatte_libretro.so`, aarch64), shipped by `dist:device` and the deploy tasks with
  its source archive in `System/licenses`. `licenses/gambatte-GPL-2.0.txt`; core credit added.
- Linux host: the device's cores are .so in `vendor/` like a host core would be, so the
  upstream real-core tests now skip a library built for another machine
  (`slot_retro::runs_here`, reading the ELF header).
- Upstream tests changed for the new default and picker: `app.rs` (START on a GB cart),
  `render_shelf.rs`, three in `slot-store/tests/core.rs`, and an unreachable arm in `gpsp.rs`.
- Not run on the device yet: Gambatte playing, its boot logo with a boot ROM, palettes, and a
  GB link session handing over to mGBA.

## First device test (2026-10-22)

The user ran `dist-device/` on the RG35XX Pro (BaseOS): shelves and About, Gambatte, Wi-Fi and
time sync, RetroAchievements, cheats, search and lists all work. Fixed after it:
- Pages longer than six rows scroll (`scrolls()` in `slot-ui/src/quick_menu.rs`): Gameplay (8),
  Game Boy's screen page (7) and Network (7) ran into the status icons and the help line.
- The RetroAchievements toast is centred across the top (`cheevos/overlay.rs` `rect(canvas_w)`).
- Lists rename (X, the keyboard with the old name) and delete (Y twice) on the Add to List
  page; `rename_list`/`delete_list` in `slot-store/src/collections.rs`. The shelf keeps its
  place in name order and its remembered cart; deleting the shelf on show moves to another.
- The device container's files are given back to the user on Linux (`device:sh`).
- GB palettes did not change on Gambatte: a palette picked while playing, and the one put back
  when a game starts, were sent as mGBA's `mgba_gb_colors` whatever the core. Now
  `gambatte::palette_options` gives each core its own (Gambatte: `gambatte_gb_colorization` =
  internal + `gambatte_gb_internal_palette`); checked on a host build of the core that the
  picture recolours mid-game.
- Phases 13–17 added to `FRAGILE_PLAN.md` at the user's request (art + ScreenScraper,
  Bluetooth, SSH + SFTP, HDMI, web dashboard), after Phase 12.

Phase 12 (NES, SNES, PS1) follows; then Phases 13–17 in `FRAGILE_PLAN.md`.

## Phase 12: NES, SNES, PlayStation (in progress, 2026-10-22)

- 12a/12b (28a9383 .. c20a41c): `Platform::{Nes, Snes, PlayStation}` as Spec rows
  (`slot-store/src/spec.rs`: dirs NES, SNES, PS; PS1 BIOS list for PCSX ReARMed) and
  `Core::{Fceumm, Snes9x, Supafaust, PcsxRearmed}` with credits and licences. Their folders
  are made at boot (`root.rs`), Start Shelf can pick them. Shelf art in `slot-ui/src/media.rs`
  (unslot's module): the disc is unslot's render; the NES and SNES shells are Fragile's, drawn by
  `tools/fragile/media/draw_carts.py` as lit height fields, measured off the user's face-on
  photographs (the user went back and forth on these; their final word is c20a41c). The SNES
  sticker keeps its own black and red under the game's tint (`Media::sticker`).
- 12c: the cores, from pinned source archives that ship beside them. `cores/libretro/build.sh`
  holds each one's make recipe (C++ ones link their C++ runtime statically; git is kept from
  reading Fragile's commit as theirs); `task core:more` builds all four in the arm64 container
  (`core:fceumm`, `core:snes9x`, `core:supafaust`, `core:pcsx_rearmed`), `dist:device` and the
  deploy tasks ship them with `<name>-<sha>.tar.gz` + `.meta` in `System/licenses`. PCSX
  ReARMed's device build uses its ARM64 dynarec (`new_dynarec`).
- `task core:more:host` builds them for the PC into `target/host-cores`, where
  `crates/slot/tests/more_cores.rs` finds them: each runs a program made in the test (an NES
  cart, a SNES cart, a PS-X EXE on PCSX's own BIOS), with every option Fragile sets declared,
  frames of a size slot shows, sound, and a state that saves and loads. Skipped without them.
- Options (`crates/slot/src/more_cores.rs`, hooked from `core.rs`): the frame skip option was
  `<core>_frameskip` for every core; only mGBA and gpSP have it, PCSX ReARMed's is
  `pcsx_rearmed_frameskip_type`. slot answers a core only about options it has set, and FCEUmm's
  code starts its overscan crop at 0 though it declares 8, so Fragile sets the top and bottom
  crop. FCEUmm declares its options when a game loads, not before.
- 12d, PlayStation:
  - Scanning (`slot-store/src/discs.rs`, from a closure the scan loop in `scan.rs` now shares):
    for a system with `Spec::discs` (PS1), games one folder down count too, and every disc an
    `.m3u` lists (paths from its own folder, case ignored, `\` taken as `/`) is kept off the
    shelf behind it. Discs no `.m3u` lists stay separate games.
  - Disc swapping: libretro's disk control interface (`slot-retro/src/disc.rs`, env 13/57/58,
    version 1) behind `RetroCore::{discs, set_disc}`; a swap opens the tray, changes the disc
    and shuts it (PCSX keeps the lid open two seconds by itself). The emulator thread publishes
    the disc and count after loading, a state load and a swap (`emu::Cmd::SetDisc`,
    `Shared::discs`); the session hands them to the app each frame and the asked-for disc back.
  - In game, a Disc row on the menu's first page ("2 of 3", left and right round the ends,
    `app/discs.rs`), shown only for a game on more than one disc.
  - No PS1 BIOS in `BIOS/` (any of the Spec's names, any case): PCSX ReARMed runs on its own,
    and the first time a game starts that way a toast says so (`Toast::BuiltInBios`, "Built-in
    BIOS: add scph5501.bin"), remembered per system as `bios_noted.<dir>` in extras.state.
  - Tests: `slot-store/tests/discs.rs`, `slot/tests/discs.rs` (the row, the note, the emulator
    thread on a mock with three discs), and the real PCSX ReARMed hands slot its disc interface.
    Not tested on a real multi-disc image (none on the PC).
- 12e, menus and buttons:
  - Settings › Screen has NES, Super Nintendo and PlayStation pages: Shader (the Spec's list,
    in extras.state as `shader.<dir>`), Integer Scale (`integer_<dir>`) and Card Shader. In game,
    Screen's Shader row is the playing system's; Color Correction and Bezel hide for systems
    with none (`app/pads.rs` `system_hidden`).
  - Integer Scale on a system drawn at its own size: the game pass draws the frame at the
    largest whole multiple that fits, square pixels, centred (`GamePass::set_whole`,
    `whole_multiple` in `slot-gfx/src/pipeline.rs`; letterboxed as before when not even one
    fits); a SNES frame on the Pro is 512x448. `App::integer_native`, hooked in `frontend.rs`.
  - Controls has X, Y, L2 and R2 rows (`QuickRow::GameX` ...), already bound to the same
    buttons; in game each pad shows only its own buttons (no turbo on SNES or PS1, no L2/R2
    but on PS1).
  - A single-button shortcut on one of the playing pad's own X/Y/L2/R2 gives it to the game
    (`App::yield_to_pad`, from `session.rs`): on PS1, R2 and L2 are the game's, not fast forward
    and rewind (rewind is off for PS1 anyway; fast forward needs a chord there). The Game Boy
    family's pads have none of those, so theirs are upstream's.
  - Clashes (`input/table.rs`): two console buttons clash only when some system's pad has both
    (`GameButton::on_one_pad`: Turbo A and X both on the X button is fine), and a shortcut never
    clashes with a pad's own X/Y/L2/R2, since it gives way to them in game.
  - Tests: `slot/tests/systems.rs`, and a render of a SNES game at Integer Scale in
    `render_media_shelves.rs` (on the mock core, 256x224).
- Phase 12 is code-complete.

### Second device test (2026-10-23, build 4ad5451)

The user ran Phase 12 on the Pro with their own games (kept in the git-ignored
`dist-device/Games`, with discs of an .m3u in `.hidden/`). Shelves fine. Found, reproduced on
the PC with the host cores and those games, and fixed:
- NES and SNES crashed and PS1 froze on MENU: `quick_value` asked `ColourCorrection` of a
  system with no row of its own, which `playing_row` gave back unchanged, so it asked itself
  forever (stack overflow); then the in-game menu indexed its three Game Boy bezel faces by
  the NES's platform number. Both guarded; `systems.rs` and `render_media_shelves.rs` open the
  menu over each new system. The Disc row ("1 of 2") showed once the menu opened.
- Wi-Fi: Forget sat across the open slot. The quick menu now plugs the slot under a page whose
  footer spreads a legend across the middle (`App::footer_spread`), as under a binding row.
  Every footer page's render was checked; no other overlaps.
- PS1 fast forward and rewind: with R2/L2 the game's, they move to SEL+R2 / SEL+L2 (in place of
  the Game Boy palette shortcuts, which do nothing there); the Button Guide says so and lists
  only the playing pad's buttons. `Spec::rewind` was never read (PS1 was snapshotted every other
  frame, 4.3 MB each); it is gone, and a state is saved for rewind every
  `rewind::snapshot_every` frames by size (256 KB a frame: GBA 2, Snes9x 4, PCSX 17).
- Prefetch read an .m3u's text and whole disc images; it now reads an .m3u's first disc, and
  of any disc image only its first 32 MB.
- Open: "GBA card shader failed to load". The four shipped shaders pass `glslangValidator` as
  GLSL ES 1.00 exactly as the device compiles them, and are pure ASCII; the driver's message is
  in slot.log (`slot: shader: ...`) on the device, asked of the user.
- The card's log shows FAT errors on the games partition (`invalid cluster chain`).

Then, on build 5135f74:
- Changing disc froze the game. The swap was made at once while the menu had the game paused,
  and PCSX ReARMed's lid reads open only two wall-clock seconds after it shuts, so the game never
  saw the lid lift. Now the Disc row's pick goes to the core when the menu shuts; the core opens
  the tray (`RetroCore::open_tray`), the game runs `TRAY_OPEN_FRAMES` (60) with it open, then the
  disc goes in and it shuts. Checked on the PC core with FF IX. Nothing swaps on its own: the
  player changes disc when the game asks.
- GB palettes did nothing: SEL+L2/R2 and the in-game Palette row needed Settings › Screen › GB ›
  Palettes on (off by default), and said nothing. Now a monochrome GB game can always take one
  (`core::takes_palette`, which no longer passes NES/SNES files), and the first step turns
  palettes on. Checked Gambatte recolours at run time on Link's Awakening. The `gb_palettes.rs`
  tests run on a header-only cart when Catrap is not on the machine.
- `task dist:device` and `task clean` keep `dist-device/{Games,Cheats,Wallpapers}`.
- Blend Grid failed on every core: the device's driver refuses the quotes of its `#pragma
  parameter` lines (`0:13: L0001: Unknown character '"'(34)`, from the user's slot.log; desktop
  Mesa and glslangValidator ignore them). `retroshader::stage_source` now blanks those lines, as
  it does `#version`, so RetroArch shaders with parameters load too. The other three loaded.
- The card log: `mmcblk0p7` was set read-only at boot over FAT errors; fsck asked of the user.

## Phase 13: Game Art (2026-10-23)

- unslot's scraper (`slot/src/scrape.rs`, `json.rs`) cut to the six systems, fetching through
  ureq (a direct dependency of `slot` now, the same 2.x RetroAchievements uses) instead of
  BaseOS's curl; Cart Studio's Logo Only label from `slot-ui/src/scraped.rs`, unchanged.
- Main menu → Network → **Game Art** (`slot/src/app/game_art.rs`, an overlay page like
  Networks): a status row, All systems, each system with "n of m" labelled, ScreenScraper
  (sign in on the keyboard; an empty name signs out). A finds the missing ones; a label made
  goes on its cart on every shelf, and `CartFaces::rebuild` builds the face again.
- ScreenScraper needs a developer ID: `devid=`/`devpassword=` in `Config/screenscraper.conf`,
  or one built in from `FRAGILE_SS_DEVID`/`FRAGILE_SS_DEVPASSWORD`. None is built in now, so
  libretro-thumbnails alone is used.
- Off the device the scraper is pretend (`SimFetch`) unless `SLOT_SCRAPE_LIVE=1`. Live on the PC
  with four of the user's games: all four labelled in 23 s (Pokémon Yellow by CRC under a
  home-made name, FF Tactics' .chd by name); the PS1 redump database is 3.9 MB, cached in
  `System/Cache/Scraper`.
- Not run on the device yet. A multi-disc `.m3u` named unlike the database (e.g. "FF IX ATB")
  finds nothing.
- Label styles (the user's call: Fragile is its carts and discs, so the default is the game's
  own label). `slot_store::LabelStyle` Label (default) / Art / Logo: Label is the printed cart
  label (ScreenScraper `support-texture` only: its `support-2D` is the whole cart drawn round
  the label, as omega-zero's label_sync notes), falling back to the logo, which is laid out
  for a label; Art is the box cut to the label; on a PS1 disc Label and Art are the disc scan
  (`support-texture`, then `support-2D`), then the box. The user first took "cart label" for
  box art; box art on a 2:1 GBA label is a crop. Global in extras
  `label_style` (Game Art → Label Style), per game in `game_profiles.txt` `.label` (in-game
  This Game → Label; Defaults clears it). A wide label cuts the box near its top
  (`slot_ui::box_label`), so a GBA cart shows the title.
- Game Art's labels go to `System/Cache/Scraper/Labels/<sys>/`, which `scan` reads when
  `Labels/` has none: the player's own always win and are never redrawn. Each game's
  downloads stay in `System/Cache/Scraper/Art/<sys>/<stem>.{logo,box,scan}.png` with a
  `.looked` marker, so a new style redraws offline and a game is fetched once.
- Automatic (extras `game_art_auto`, default on): with the internet up and the scraper idle,
  the games with no label not yet asked for since boot. Only with a live scraper (the device,
  or `SLOT_SCRAPE_LIVE=1`); tests hand in `SimFetch` through `App::use_scraper`.
- Live on the PC: Golden Sun, Link's Awakening and FF Tactics as box art in 12 s, then drawn
  again from the cache with no fetch.
- Each label's style is kept as `<stem>.style` beside its art; once after boot every Game Art
  label goes to the worker, which draws again only those whose style differs (the labels the
  Pro drew as box art before the default changed).
- A `.looked` marker says `screenscraper` when ScreenScraper was asked; a game looked for
  without it is looked for again once there is a developer ID (at boot, or on signing in), its
  libretro art kept where ScreenScraper has nothing.

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
