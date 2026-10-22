# Fragile — port fork features into slot (v1.8.0 → "Fragile" for RG35XX Pro on BaseOS)

The project/fork is named **Fragile**: use that name for the branch, About page, README/CREDITS, build artifacts (where renaming is cheap), and docs. Keep internal crate names (`slot-*`) unchanged so upstream merges stay easy.

## Context
Upstream slot (BrandonKowalski, v1.8.0 @ f4b7bc3) is a polished GB/GBA frontend but lacks features that 10 community forks (cloned in `forks/`) built. The user wants those features in slot with the **same frontend look, polished** (label-left/value-right rows, one-line help per row, new pages), including changes to the main menu and the in-game menu. Target: **RG35XX Pro on BaseOS** (BaseOS/ untracked in repo), staying compatible with upstream devices.

Selected: Cheats, RetroAchievements, FF/per-game tuning + battery HUD, Screen extras, Search + list shelves, Button cheat sheet, Shelf QoL, About/system info + 12h clock, Wi-Fi + NTP, Gambatte, NES/SNES/PS1. Not selected: lid suspend, full redesign, GLES cores.

Key facts that shape the plan:
- All forks are based on v1.4–v1.7; nothing cherry-picks cleanly onto v1.8's binding `Table`. Each feature is **re-implemented against v1.8 using fork files as reference** (and their tests ported).
- `rg35xxpro` is already in `LEFT_STICK_TARGETS` (`crates/slot/src/input/evdev.rs:20`); slot reads `BASEOS_TARGET` (`device_app.rs:130`).
- BaseOS leaves Wi-Fi association to the frontend (`BaseOS/docs/05-runtime-power-network.md` §3) but ships `wpa_supplicant`, `wpa_cli`, `udhcpc` + script, and `timedatectl set-ntp` → `/run/baseos-ntp-synchronized`.
- `slot-retro` assumes 240x160 (`libretro.rs:311`, `GBA_W/H` ×34), memory-loaded ROMs, no geometry/disc/cheat API. `Platform`/`Core` are closed enums.
- `slot_state.rs::parse` rejects files missing required keys → new settings must never be required.

## Integration rules (keep upstream merges cheap — nickclyde's approach)
- Work on a branch (`fragile`), not `main`. One commit series per phase, tests green at each commit.
- Each feature = Cargo feature in `crates/slot/Cargo.toml` (`cheats`, `cheevos`, `wifi`, `search`, `systems`; default on), code in `crates/slot/src/<feature>/{mod,live,off}.rs`; core files (`app.rs`, `emu.rs`, `frontend.rs`) get one-line hooks only. One `extras: Extras` field on `App`, one `overlay: Option<Overlay>` for dynamic pages.
- New settings live in `Config/extras.state` (new `crates/slot-store/src/extras.rs`, all keys optional, via `ini.rs`). `slot.state` untouched except the Phase 7 migration.
- New `QuickRow`s grouped under `// --- extras ---`; extra page arrays in `crates/slot-ui/src/quick_menu_extras.rs`. Every new row must hit: `ALL`, page array, `page/parent/child/label/help/opens` (quick_menu.rs), `quick_value` :941, `open_quick_row` :2488, `change_setting` :2574, `hidden_rows` :1241 / `in_game_hidden` :894 (app.rs). Add a test that every row in `ALL` is on exactly one page and has non-empty `help()`.
- Ported files carry `// Ported from <fork>@<sha>, <path>`; add `CREDITS.md` + license files.
- Add `forks/`, `BaseOS/`, `release/` to `.gitignore` (or keep untracked) so they don't get committed.

## Phases

### 0 — Device baseline
README device row for RG35XX Pro; confirm left stick + record right-stick codes in `input/evdev.rs`. Test: target parsing for `rg35xxpro`.

### 1 — UI foundation (menu polish)
- `QuickRow::help()` + help line under every menu (generalise `quick_description_face`, quick_menu.rs:869; prerender in frontend.rs:821; replace special case app.rs:1799). Label-left/value-right alignment and scroll arrows in `QuickMenu::draw` (:1026).
- New `crates/slot-ui/src/list_page.rs` — dynamic list page styled like a QuickMenu page (for cheats, SSIDs, search results, lists, discs, achievements, button sheet) with small label-texture LRU in frontend.rs.
- Port `forks/nickclyde-slot-plus/crates/slot-ui/src/keyboard.rs` + symbols layer + password mask.
- `slot-store/src/extras.rs` (`ExtrasState`).
- Tests: `slot-ui/tests/list_page.rs`, `keyboard.rs`, `slot-store/tests/extras.rs`.

### 2 — Shelf QoL, About, clock
- marghost: startup shelf, per-shelf cart memory (`tests/shelf_memory.rs`), accelerated scroll (`slot-ui/src/shelf.rs`), hide Favorites.
- nickclyde: Recents/Favorites same-name GB/GBC fix, late-face fix in `cart_faces.rs`.
- slot.advance `card_space.rs`; markbruno 12h clock (`slot-ui/src/clock.rs`, polaroid timestamps).
- Menus: General → Start Shelf, Show Favorites, Clock Format. New About page: Version, BaseOS version, Device, Games, Free SD1/SD2, Battery.

### 3 — Per-game tuning, FF pitch, battery
- slot.advance `slot-store/src/game_profile.rs` (`Config/game_profiles.txt`, keyed `<platform dir>/<stem>`).
- marghost `slot/src/pitch.rs` (Keep FF Pitch) into the audio/resample path.
- marghost battery badge + 15%/10% low-battery banners merged into upstream `slot-ui/src/battery.rs` (`tests/battery_warnings.rs`).
- In-game: Settings → **This Game** page (Shader, Colour, FF Speed, FF Sound, Use Defaults). Gameplay → FF Pitch. General → Battery Badge.

### 4 — Screen extras
- marghost integer 3x GB/GBC scale (`video_mode.rs`, `slot-gfx/tests/scale.rs`) + classic bezels (`panel_bezel.rs`; CC BY-SA attribution).
- markbruno user `.glsl` from `Shaders/` (`slot-gfx/src/retroshader.rs`, `slot-store/src/shaders.rs`) with fail-safe fallback.
- Baggio94 GBA shader presets (branch `origin/extract/konkr-production-0.2.1`, `slot-gfx/src/shaders.rs`); roygravy motion blur / see-through LCD (prev-frame texture in `fbo.rs`; measure on Mali-G31).
- `Shader` → `ShaderPick { Builtin, User(name) }`, persisted backward-compatibly (`user:<name>`).
- Rows on SCREEN_* and PLAY_SCREEN: Scale, Ghosting, User Shader.

### 5 — Button cheat sheet
List page generated from live `input::table::Table` (`Shortcut::ALL`, `CarouselAction::ALL`, current chords) — layout ideas from marghost `cheat_sheet.rs` / roygravy `buttons_help.rs`. Row on Controls page and in-game PLAY. Test: rebinding changes the sheet.

### 6 — Search + list shelves
nickclyde `slot-store/src/find.rs`, `collections.rs` (`Config/Lists/*.txt`), `app/shelves/search.rs`. `Rack` gains `Search`, `List(u8)` (`shelves_of` app.rs:463). New `CarouselAction::Search` (X) and `AddToList` in `slot-store/src/bindings.rs` + `input/table.rs` + SHELF_KEYS page. Tests: find/collections + clash test.

### 7 — Platform/Core generalisation (foundation for 11–12)
- `Platform` → data `Spec` table modelled on `forks/cloudwithax-unslot/crates/slot-store/src/platform.rs` (cores, canvas, shaders, bezels, shoulders/triggers/turbo/rewind/link, bios). `Core` data-driven.
- Per-platform `SlotState` fields → map keyed by platform dir; still read **and** write legacy `shader_gba/gb/gbc` etc. Golden-file migration test.
- `slot-retro`: variable canvas, `SET_GEOMETRY`/AV info, aspect, `need_fullpath`, `SET_DISK_CONTROL(_EXT)`, optional cheat symbols (from unslot `libretro.rs`). Emu jobs (`emu.rs:75`): `SetCheats`, `Reset`, `Disc(u32)`. Variable-size game texture, thumbnails, `StateRing`.
- `GameButton` gains X/Y/L2/R2/L3/R3, gated by `Spec`, wired through `table.rs`.
- All existing GB/GBA tests must pass unchanged; mock core gains a 256x224 mode.

### 8 — Cheats
markbruno `.cht` parser + `RetroCore::set_cheats`; roygravy per-game state `Config/Cheats/<dir>/<stem>.txt` + cheat icon. In-game PLAY → **Cheats** row (hidden when no `.cht`/unsupported) opening a toggle list page. Skip during link play. Tests: parser, persistence, reapply after state load.

### 9 — Wi-Fi + NTP (BaseOS-backed)
- `slot/src/wifi/` with `Radio` trait: device impl drives `wpa_supplicant -B -i wlan0 -c Config/wifi.conf -C /run/slot-wpa`, control-socket client from unslot `wifi.rs`, then BaseOS `udhcpc` script. Host impl = fake.
- Yield radio to link play (`card/System/slotlink.sh`) using nickclyde's `Watcher` pattern; restore after.
- NTP: `timedatectl set-ntp yes|no`, status from `/run/baseos-ntp-synchronized`.
- Settings → **Network** page: Wi-Fi on/off, Network (SSID list → keyboard), Status, IP, Time Sync; Wi-Fi status icon. `wifi.conf` mode 600 (document plaintext PSK).

### 10 — RetroAchievements (softcore)
nickclyde `crates/slot-cheevos` (rcheevos MIT + shim) and `slot/src/cheevos/*`, `slot-retro/src/memory.rs`; hooks in `emu.rs`/`frontend.rs` guarded by `tests/cheevos_hooks.rs`. Network page → RetroAchievements (on/off, sign-in via keyboard); in-game **Achievements** list page. Verify aarch64 TLS cross-build early.

### 11 — Gambatte
roygravy `cores/gambatte/build.sh`; `core:gambatte` tasks in `taskfile.yml`; Core table entry for GB/GBC; per-core option table replacing hard-coded `apply_core_options` (`crates/slot/src/core.rs:188`); GB palette mapping. GPL-2 license file.

### 12 — NES, SNES, PS1
- Cores: fceumm (NES), snes9x (non-commercial license — document; allow snes9x2005/supafaust), pcsx_rearmed (PS1); build scripts from `forks/oauramos-omega-zero/cores/`.
- `Spec` rows, region handling (omega `region.rs`, `snes.rs`, `ps1.rs`).
- PS1: `.cue/.chd/.m3u`, BIOS check in `BIOS/` with refusal screen (`slot-ui/src/refusal.rs`), in-game **Disc** row, loading indicator.
- Cart/disc art from unslot `slot-ui/src/media.rs` into `cart.rs`/shell art; new shelves per platform.

### 13 — Cart art + ScreenScraper (added 2026-10-22, after the first device test)
Port cloudwithax/unslot's Game Art: names from libretro-database, art from libretro-thumbnails
into `Labels/<console>/<stem>.png` (where slot already reads labels), styled as Cart Studio does.
ScreenScraper with the user's login typed on the keyboard (`Config/screenscraper.conf`).
Risk: ScreenScraper's API also needs a developer ID; unslot's built-in one cannot be reused, so
the user registers one (`devid=` in the conf) or only libretro-thumbnails is used. Needs Wi-Fi
(Phase 9). Reference: omega-zero `label_sync.rs`, pvaibhav's LaunchBox fetcher.

### 14 — Bluetooth
Port unslot's Bluetooth page and `card/System/slotbt.sh` (pair, connect, forget; controllers and
headphones) over BaseOS's BlueZ + BlueALSA (`systemctl` shim). Steps aside for link play like
the Wi-Fi does.

### 15 — SSH + SFTP
Port the services switch (unslot `slotservices.sh`, omega-zero `ssh.rs`): dropbear (MIT) built
for aarch64 in the device container, SFTP with it rather than plain FTP, the password shown on
the Network page. Off by default.

### 16 — HDMI out
New (no fork). Probe the Pro first: BaseOS switches HDMI through the vendor display driver
(debugfs; validated only on the RG40XX V). Offer 720p/1080p and 50/60 Hz only if the driver
allows them; the compositor already lays out from the panel size.

### 17 — Web dashboard
New. A small LAN-only, password-protected page served by the Pro over Wi-Fi to edit cart shells
and labels (writes `Labels/cart_shell.txt` like Cart Studio; label PNG upload), lists and
favourites. Needs Wi-Fi.

## Verification (each phase)
1. `task fmt:check`, `task lint`, `task test` (add GL/core tests to `IN_PROCESS` + `SLOT_IN_PROCESS` in `taskfile.yml`).
2. `task run` on host: inspect new pages/menus at 640x480 and 720x480.
3. `task lint:device` / device build; copy to card; on RG35XX Pro check boot time, frame pacing (`latency.rs`), both SD cards, battery.
4. Phase-specific on device: Wi-Fi join → reboot → reconnect → link session yields/restores → NTP sync (9); unlock softcore achievement (10); multi-disc PS1 swap (12).
5. Regression on one other upstream device (e.g. RG34XX 720x480) after phases 1, 7, 9.

## Risks
- Phase 7 is the largest upstream divergence (≈90 `Platform::` uses) — land in small commits.
- Wi-Fi vs link-play radio contention.
- `slot.state` wipe if a new key becomes required — keep new keys in `extras.state`, test old files.
- Licenses: snes9x non-commercial, GPL shaders/cores, CC BY-SA bezels — track in `licenses/`, exclude non-redistributable bits from release zips.
