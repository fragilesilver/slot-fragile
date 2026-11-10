# Changelog

## [1.9.7](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.7)

- Nintendo DS Core Upgrade (DSperate v3.4.1):
  - Upgraded DSperate core to upstream v3.4.1, incorporating incremental VRAMCNT page remapping to eliminate frame stutters during scene transitions and inventory fades (e.g., *Dragon Quest IX*), ARM7 wake-wait sleep fix (e.g., *New Super Mario Bros.*), authentic NDS Phat / NDS Lite colour profiles, fast CPU timing, and multi-threaded 3D geometry rendering with asynchronous command FIFO queue.
  - Zero-copy ROM loading (`need_fullpath = true`) bypassing full ROM memory duplication for fast startup.
  - Firmware dump size validation and resilient FreeBIOS fallback: detects corrupted or non-standard firmware dumps (restricting valid dumps strictly to 128 KB, 256 KB, or 512 KB) and automatically falls back to clean FreeBIOS execution rather than hard-freezing.
  - Slot-2 hardware emulation: adds GBA cartridge passthrough and Rumble Pak simulation support.
  - Accurate CPAD analog controls with game-specific analog sensitivity curves and database lookup.
  - SRAM / Save pre-seeding: automatically sizes initial flash memory arrays to existing `.sav`/`.srm` files to eliminate save-truncation across emulator cores.
- Per-System Controller Rebinding:
  - In-game controller settings now persist per-system configuration files (`Config/controls_<platform>.txt`, e.g., `controls_gba.txt`, `controls_snes.txt`), keeping remapped controls isolated to that system without altering global defaults or affecting other platforms.
  - In-game controller settings menu dynamically filters rows to only display buttons native to the playing platform (e.g., L/R/X/Y hidden on Game Boy and Game Boy Color).
  - Shelf/Main Menu continues configuring the global default controls (`Config/controls.txt`), used as a fallback for any system without a dedicated override.
  - Resetting controls in-game cleanly removes the system override and restores the global configuration.
- Auto Load State:
  - Added `Auto Load State` setting in the Game Quick Menu with `Always`, `Power On`, and `Never` modes.
  - `Always`: Automatically restores the latest save state on every cartridge launch.
  - `Power On`: Automatically loads state only when resuming a seated cartridge directly at device boot; launching carts from the shelf boots cleanly to the title screen.
  - `Never`: Always boots cleanly to the title screen.
- Emulation Real-Time Clock (RTC) Local Timezone:
  - Automatically exports the handheld's configured local UTC offset to libc's POSIX `TZ` environment string and invokes `tzset()`.
  - Ensures libretro cores (mGBA, Gambatte, DSperate) read authentic local time for games with time-of-day mechanics and day/night cycles (*Pokémon*, *Boktai*, *Animal Crossing*) without modifying the host root filesystem.
- Per-System Core Disambiguation:
  - Enhanced `Config/selected_core.txt` to format entries as `<Platform>/<stem> = <core>` with graceful fallback to bare stems.
  - Prevents multi-platform games sharing identical titles or filenames from colliding or overwriting each other's core assignments.
- Gambatte Rumble & Audio Wave RAM Centering:
  - Configures `gambatte_rumble_level = 10` before initialization, enabling tactile rumble support for Game Boy rumble games (*Pokémon Pinball*, *Star Wars Episode I: Racer*).
  - Wave RAM DC offset centering to preserve pristine audio playback on handheld speaker hardware.
- Kid Mode Lockdown:
  - Added `Kid Mode` under General Settings with `Off`, `One Shelf`, and `Shelves` modes.
  - Restricts access to the Quick Menu, in-game menu, core selection, cartridge deletion, and card repair to safeguard handheld state.
  - `One Shelf`: Restricts gameplay to carts on the active shelf and locks shelf switching.
  - `Shelves`: Allows switching between game shelves while keeping settings and system menus locked.
  - Emergency hardware unlock chord: Pressing `Start + Select + L2 + R2` simultaneously instantly unlocks Kid Mode.
- SD Card ROM Filename Sanitizer:
  - Added boot-time sanitization sweep (`sweep_names`) that inspects ROM directories on startup and renames filenames with invalid trailing dots or whitespace across ROM files, `.sav` save files, and `.png` cart art labels.

## [1.9.6](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.6)

- NDS Stylus & Analog Stick Navigation:
  - Dual Analog Stick & D-Pad Stylus Navigation: Left Analog Stick or Right Analog Stick smoothly navigates the virtual touchscreen stylus, with physical stick clicks (**L3** or **R3**) triggering touch tap and hold-to-drag.
  - Coexists with **Hold L2 + D-Pad** (with **R2** tap/drag) for classic D-Pad touch control.
  - Dedicated Stylus menu options in Quick Menu and per-game Profile (`Hold L2`, `D-Pad`, and `Off`).
  - Immediate and Timed Reticle Auto-Hide: the touch reticle vanishes instantly when releasing L2 in Hold L2 mode, auto-hides after 60 frames (1 second) of inactivity during stick or D-Pad movement, and never displays when Stylus is set to `Off`.
  - Clean input pass-through: removed experimental `L2 + START` combo to prevent any disruption to in-game DS start/pause buttons.
- Baked-in Bezel Design Kit:
  - Pre-packaged comprehensive Bezel Design Kit across all 7 systems directly on the distribution image under `Bezels/`.
  - Includes layered Adobe Photoshop (`.psd`) templates with labeled artboards, pixel-perfect viewport guides, and cutouts for 640×480 and 720×480 handheld displays.
  - Complete with high-resolution PNG viewport guide overlays (`_Templates/`) and sample overlay presets (`Sample.png`, `Sample 2x/3x.png`) for GBA, GB, GBC, NES, SNES, PS, and NDS.

## [1.9.5](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.5)

- Overlays Across All Systems:
  - Added custom PNG overlay and bezel support across all 7 supported systems (Game Boy Advance, Game Boy, Game Boy Color, NES, Super Nintendo, PlayStation, and Nintendo DS).
  - Renderer has no coverage or alpha limits: transparent-window bezels, full-panel scanlines, CRT shadow masks, and fine pixel grids are all fully supported.
  - Auto-created `Bezels/GBA`, `NES`, `SNES`, `PS`, and `NDS` folders on card initialization alongside dedicated per-system `README.txt` files detailing viewport coordinates and scaling alignment.
  - Flexible directory alias matching: PlayStation bezels placed in `Bezels/PS1` automatically fall back and resolve cleanly alongside `Bezels/PS`.
  - Generalized Integer Scale companion support: `{stem} {k}x.png` automatically switches into place when Integer Scale is enabled for non-native systems (2x for GBA on 640x480, 3x for GB/GBC), and remains hidden from the menu picker. Native systems automatically hide overlays when Integer Scale is active to prevent aspect distortion.
  - Renamed menu row labels across all bezel and overlay options to `"Overlay"` with unified helper guidance.
- Upstream v1.14.0 Alignments:
  - Gambatte BIOS Bootloader: Pre-configures Gambatte's bootloader option before core initialization so card-provided `BIOS/gb_bios.bin` boots with the authentic Game Boy chime, scrolling logo, and hardware intro.
  - Remove from Recents with `X`: Pressing `X` on the Recents shelf removes the highlighted game from the recents list without deleting save data.
- System Atmosphere & Quality of Life:
  - Network Time auto-synchronization via SNTP whenever an active Wi-Fi connection is established.
  - Dynamic shelf wallpapers: automatically loads per-platform and per-game custom backgrounds (`Wallpapers/<platform>.png` or `Wallpapers/<stem>.png`).
  - Bluetooth LE controller persistence and reconnect daemon for paired controllers.
  - CPU governor tuning (`schedutil` on multi-core platforms) for optimal battery runtime and performance.

## [1.9.4](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.4)

- Nintendo DS Cartridge Scale & Authentic Cutout:
  - Rescaled cartridge geometry to authentic NTR-005 card dimensions (300×318 canvas, scaled to 240×254 in slot) ensuring proper proportional fit alongside other systems on the shelf.
  - Restored authentic label well (`(26, 62, 248, 202)`) preserving the debossed downward grip arrow and official header.
  - Restored 20px 45° bottom-left chamfer cutout with 4px corner rounding on cartridge stickers.
- RetroAchievements:
  - Added native ISO9660 reader for Sony PlayStation CHD (`.chd`) disc images and `.m3u` playlists, enabling RetroAchievements game identification and badge unlocks for compressed disc formats.
  - Added transparent `.zip` archive decompression for ROM hashing, ensuring Nintendo DS (and other systems) packaged in `.zip` archives hash the uncompressed ROM payload rather than the zip container.
  - Enabled RetroAchievements badge display for Nintendo DS titles.

## [1.9.3](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.3)

- Nintendo DS:
  - Updated cartridge shell to photorealistic NTR-005 mould with authentic dark charcoal finish, debossed downward insertion arrow, and official "NINTENDO DS®" header.
  - Eliminated screenshot fetching (`Named_Titles`), prioritizing authentic cartridge sticker artwork and official front cover art.
  - Dual artwork support: full-bleed cartridge stickers (aspect $\le 1.05$) cover the sticker well with authentic 45° bottom-left chamfer; box covers (aspect $> 1.05$) are framed cleanly between the official header and footer.
- Fast Forward Controls:
  - Fixed chorded fast forward (`Select + R2`) in Toggle mode: latches active on press, persists across button release, and toggles off on next tap.
- Artwork & Cutouts:
  - SNES cartridge sticker expanded to full 223x99 well with 3px rounded corners.
  - NES cartridge label fitted with bottom corner radius mask (3.5px) matching recess.
  - Removed destructive percentage cropping on existing stickers across all systems.
  - Expanded media scan to discover local `media/boxart`, `media/images`, and `media/covers`.

## [1.9.2](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.2)

- Nintendo DS & Media Cartridges:
  - Local cartridge artwork discovery prioritizing `media/cartart/`, `media/carts/`, and `cartart/` over box art across library scan, scraper, and web dashboard.
  - Transparent/Clear and Glitter finishes applied cleanly to media face shells (NES, SNES, PS1, NDS) with authentic alpha modulation and sparkle flecks.
- Web Dashboard:
  - Dynamic PMIC battery sysfs detection across AXP PMICs (`axp2202-battery`, `axp20x-battery`, etc.) for real-time battery status and charging indicators.
- RetroAchievements:
  - Broadened core memory fallback (`core_memory`) mapping System RAM and Save RAM for non-GBA/GB cores (FCEUmm, Snes9x, PCSX-ReARMed, DSperate).
  - Raised live ROM hashing limit to 512MB for optical discs and large NDS paks.
  - Informative toast notices when achievements are loaded or when a game is unrecognised.
- OTA Updates:
  - Release asset download URL resolution fixed for seamless in-place updates.

## [1.9.1](https://github.com/fragilesilver/slot-fragile/releases/tag/v1.9.1)

- 7 emulated systems with authentic skeuomorphic cartridge & media artwork:
  - Game Boy Advance (curved shell, solid/clear/glitter tinting, interior PCB board)
  - Game Boy & Game Boy Color (notched and rounded paks, custom palettes)
  - Nintendo Entertainment System (tall ribbed cart, authentic label well)
  - Super Nintendo Entertainment System (curved cart, official sticker header)
  - Sony PlayStation (circular optical disc, clear hub hole, reflective bevel)
  - Nintendo DS (authentic NTR-005 card mould with solid dark shell, bottom-left 45° chamfer cut, debossed arrow, white sticker paper, and official Nintendo DS header)
- Nintendo DS Touch Cursor: navigate touch screen using L2 + D-Pad without needing an external stylus.
- Fast Forward Hold / Toggle mode configurable in Quick Menu and persisted across sessions.
- In-game screenshot capture (hotkey) and Polaroid-style photo viewer with in-game management.
- Over-The-Air (OTA) updates engine: background streaming downloads, SHA-256 integrity verification, and crash-safe atomic staging.
- Redesigned Web Dashboard:
  - Live 3D cartridge rendering powered by native device moulds.
  - Cart Studio live previews for plastic colours, finishes, and outlines.
  - View toggle between floating 3D Cartridges and flat Cover Art in Labels & Media.
  - Save Manager battery backup restore with DeSmuME .dsv, .sav, and .srm support.
  - Nintendo DS support across system filters and ROM dropzone.
- Storage endurance and crash safety: syncfs flushing, crash watchdogs, decoupled autosaves, and 3-crash rollback safeguards.
- Screensavers & auto-doze: configurable Dim, Clock, CartArt, and Blackout styles.
- Softcore RetroAchievements over Wi-Fi.
- Two-device wireless link cable multiplayer.
- Bluetooth audio & controller management.

## [1.8.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.8.0)

- When the SD card turns read-only, SELECT + START repairs it: slot steps aside, checks FAT32 or exFAT, and restarts. (#72, #111)
- Power off, including the shutdown after the lid has been closed for 3 minutes, no longer hangs on a black screen: the Wi-Fi driver is unloaded first, as Restart already does.
- If slot stops responding with the lid closed, a separate watcher powers off a minute after the Lid Off time.
- Run-Ahead is removed. It scrambled saves in games with a 128 KB flash chip, such as Super Mario Advance 4. (#104)
- Turbo A and Turbo B have their own rows on the In-Game controls page and can be moved off X and Y. (#127)
- Shortcuts and shelf keys take any two buttons held together, such as L1+R1. In a game the first button still reaches the game at once; the second fires the shortcut. SELECT, START and MENU only work as the first button. Brightness and blue light shortcuts also work on the shelf; a shelf key on the same button still moves the shelf.
- Setting a control that something else already uses shows a warning naming it, such as "Same as Fast Forward", and marks both rows with a warning triangle.
- Loading a state while a game is saving to a 128 KB flash chip no longer writes the save into the wrong half of the chip.
- The cart insertion animation no longer stutters, and a large ROM still loading after the cart seats powers the screen on to a blank panel that brightens until the game appears. (#119)
- On Anbernics with a left stick, the stick works as the D-pad. Left Stick as D-Pad on the Gameplay page turns it off. (#106, #107)
- `Labels/`, `Wallpapers/` and `Bezels/` each hold a `README.txt` with the sizes and formats they take, and the site's Shelf questions cover them too. (#122)
- On 640x480 devices a 640x480 wallpaper and a 640x480 bezel now show pixel for pixel. Both used to be scaled to 720x480 first. Bezels, including the default Game Boy and Game Boy Color ones, now reach the top and bottom of the screen. (#122)

## [1.7.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.7.0)

- The slot, carts and shelf now have a skeuomorphic effect, with light and dark themes and bundled wallpapers. Huge thanks to Markus for this amazing work! (#74, @MarkusMaribu)
- An in-game menu: tap MENU to pause the game behind it. A double tap still goes straight to States.
- Favorites: Y stars a cart onto its own shelf. (#89, @diluno)
- A Recents shelf of the last 10 games, switched on from Settings -> General.
- slot remembers the shelf and cart at power off and boots back onto them. (#53)
- Per-platform bezels. Markus again! (#77, @MarkusMaribu)
- On 4:3 panels the shelf fills the screen. (#81, @diluno)
- Game Titles on the General page fades the shelf name into the selected game's name. (#84)
- A 1 frame run-ahead setting for GBA games on mGBA. (#56, @454135431515)
- Wallpaper Tint darkens your own wallpaper on the shelf.
- GBA, GB and GBC each get their own Shader and Color Correction rows. (#83, @paulocoe-creator)
- LCD3x is offered for GB and GBC games. (#88, @paulocoe-creator)
- Game Boy stretch now properly only widens the picture.
- The backlight steps follow a curve, so the low-end goes dimmer. (#54, @flo333)
- Holding a direction repeats it in menus, the clock and the shelf letter jump, but never in play.
- RG34XX uses its own chords for brightness and the menu display. (#85, @paulocoe-creator)
- The RG28XX picture is turned the way BaseOS says.
- Restart no longer hangs on a black screen: the WiFi driver is unloaded first.
- Muted play keeps its frame rate. (#51, @flo333)
- Fast-forward no longer stutters on frames that take longer than average. (#70)
- gpSP no longer crashes when a crowded boot left its JIT cache unmapped.
- Core options follow the platform that is playing. (#82)
- Card logs moved into a `Logs` folder, and the session log rotates. Card alarms are logged too.

## [1.6.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.6.0)

- Runs on the RG28XX: its panel is mounted portrait, so slot turns the picture onto it. (#35, @GamerAnim)
- Pokémon Pinball: Ruby & Sapphire rumble. mGBA now sees the game's Game Boy Player check. You still need to turn rumble on in game settings. (#25)
- A red alarm shows when the SD card turns read-only or a save fails to write.
- A new shortcut for mute: SELECT + VOL+ / VOL-. The 34XX has a fake volume wheel so VOL+ + VOL- isn't possible. (#39)
- Game Boy cartridge clocks are kept in the .sav. (#22)
- The GBA cartridge clock keeps running on gpSP while the game is off.
- Large libraries now should boot faster, and the first frame shows sooner.
- Games start faster: resting on a cart reads its ROM ahead, so big ROMs no longer wait on the SD card when you insert them.
- Wallpapers are dealt from a shuffled deck, so none repeats until all have shown.
- PNG art is turned the way its EXIF orientation tag says.
- When slot crashes it logs where, and keeps that run's log in `crash/` to attach to a bug report.
- The BaseOS startup sound no longer leaves slot silent. (#20)
- A user defined `Config/bootlogo.bmp` will take precedence over the shipped one. (#30)
- Cart shell, core and video mode files are now `.txt` instead of `.ini`, so Chrome on Windows opens them. slot renames them on first boot. (#15)
- The grid shader's lines stay even on the RG34XX and at fractional scales, and the dot shader no longer darkens a white screen as much. (#26, #36)

## [1.5.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.5.0)

- Rumble regression fixed.
- The boot logo is built to the size of the one BaseOS installed, so it fits every panel instead of only 720x480.
- Runs on the RG35XXSP: the picture fills the 4:3 screen's width and a GBA SP bezel sits underneath while a game plays.
- Screen shaders chosen per platform: Off, LCD3x, Grid or Dot for GBA, and Off, Grid or Simpletex for Game Boy and Game Boy Color.
- SELECT + B / A in game steps back and forth through the screen shaders. A game's own soft reset that holds SELECT with A or B no longer reaches it; hold A on the carousel to start a game fresh instead.
- New settings: auto save on eject, turbo buttons and rewind can each be switched off.
- Settings are grouped into Screen and Gameplay pages, and every value wraps from the last choice to the first.
- Label art is cached on the card, so the shelf no longer stutters building cart faces. New labels are cached once at boot behind a Caching New Labels screen.
- Named palettes for Game Boy games: turn on GB Palettes in the Screen settings, then SELECT + L2 / R2 in game steps back and forth through mGBA's 48 palettes.
- The speaker no longer buzzes while nothing is playing. slot lets go of the audio device after 3 seconds of silence and takes it back when sound starts. Thanks to flo333 for finding it.

## [1.4.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.4.0)

- The volume bar shows a headphones icon while they are plugged in.
- Speaker and headphones keep their own volume and mute, and switch the moment you plug / unplug.
- Settings moved from `System/` to a new `Config/` folder, so replacing `System/` on update no longer resets them. slot moves them on first boot.
- The quick menu and power menu wrap from the last item to the first, and back.

## [1.3.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.3.0)

- The SP boots to a slot logo instead of BaseOS's. slot installs `System/bootlogo.bmp` on first run and keeps the original as `bootlogo.baseos.bmp`.

## [1.2.2](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.2)

- Fix for hitch in games caused by autosave. The autosave now writes to the card in the background.

## [1.2.1](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.1)

- Shell colors from Cart Studio now apply to games with accented names.
- Clear GBA carts no longer show their circuit board above the label.

## [1.2.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.2.0)

- Hold X for turbo A and Y for turbo B.
- Carts on the shelf are larger, and shrink into the slot as they go in.
- Carts are drawn at their real proportions, so full label scans fit without cropping.
- Jumping a letter with Up or Down shows the letter in the slot.
- Saves now stick for games whose save is smaller than the core first reports, such as Golden Sun on mGBA.
- The seated cart no longer shows through the slot as a game starts.
- SELECT reaches the game as soon as it can no longer be the start of a shortcut.

## [1.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.1.0)

- Fast-forward runs several game frames per refresh, so it is properly fast, and rewind no longer records while fast forwarding.
- The About label credits BaseOS.

## [1.0.0](https://github.com/BrandonKowalski/slot/releases/tag/v1.0.0)

- Game Boy and Game Boy Color, each on its own shelf. L1 and R1 switch shelves, and carts are drawn at their own size in their own plastics.
- Two cores, mGBA and gpSP. Press START on a cart to open it and swap the chip.
- Link play between two RG SPs over WiFi: the link cable and the Wireless Adapter for Pokémon trades and Advance Wars, mGBA's link mode for Mario Kart, and Game Boy link for Tetris.
- A quick menu on the shelf for fast forward speed and sound, rumble, color correction, the date and time, and About.
- Game Boy games stretch to fill the screen with L1 and go back with R1.
- The real BIOS boot animation plays when the card has one.
- Per-cart shell colors, set in `cart_shell.ini`.
- Up and Down jump through the shelf a letter at a time.
- Your own folder layout on the card is kept, and slot runs on upstream BaseOS.

## [0.1.1](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.1)

- The game holds still behind the power menu.
- Rewind records at every speed and replays exactly what happened.

## [0.1.0](https://github.com/BrandonKowalski/slot/releases/tag/v0.1.0)

- First release: a GBA-only shelf of cartridges with an insert scrape and clunk.
- The game in the slot resumes on the next boot. Closing the lid sleeps and saves.
- Save states with a switcher, rewind, fast-forward, brightness, blue light and volume.
- A power menu on a held POWER, and an About label.
