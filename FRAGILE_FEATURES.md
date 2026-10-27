# Fragile OS: Complete Feature Specification & Guide

**Fragile** is a bespoke, skeuomorphic retro gaming operating system and frontend engineered in Rust for Anbernic handheld consoles powered by the Allwinner H700 quad-core processor (RG35XXSP, RG35XX Pro, RG35XX Plus, RG35XX-H, RG28XX, RG34XX, and RG34XXSP).

Built from the ground up to replace generic launcher interfaces with tangible, tactile physical console artefacts, Fragile delivers 60 FPS direct-to-framebuffer rendering, cycle-accurate emulation, over-the-air updates, softcore RetroAchievements, comprehensive save protection, and modern wireless connectivity.

---

## Table of Contents

1. [Hardware & Architecture](#1-hardware--architecture)
2. [Skeuomorphic Cartridge & Media Engine](#2-skeuomorphic-cartridge--media-engine)
3. [Artwork & Media Discovery Pipeline](#3-artwork--media-discovery-pipeline)
4. [Emulation Systems & Cores](#4-emulation-systems--cores)
5. [In-Game Controls, Hotkeys & Gestures](#5-in-game-controls-hotkeys--gestures)
6. [RetroAchievements Integration](#6-retroachievements-integration)
7. [Storage Endurance & Crash Protection](#7-storage-endurance--crash-protection)
8. [Over-The-Air (OTA) Updates](#8-over-the-air-ota-updates)
9. [Web Dashboard & Network Suite](#9-web-dashboard--network-suite)
10. [Screenshots & Polaroid Gallery](#10-screenshots--polaroid-gallery)
11. [Power Management & Screensavers](#11-power-management--screensavers)
12. [In-Game Quick Menu & Overlays](#12-in-game-quick-menu--overlays)
13. [Supported File Formats](#13-supported-file-formats)

---

## 1. Hardware & Architecture

### Target Platform
- **SoC**: Allwinner H700 (Quad-Core ARM Cortex-A53 @ 1.5 GHz)
- **GPU**: ARM Mali-G31 MP2 (OpenGL ES 2.0 / 3.2, Vulkan 1.1)
- **RAM**: 1GB LPDDR4
- **Display**: 640×480 3.5" IPS (4:3) / 640×480 2.8" IPS / 720×480 3.4" IPS
- **Supported Devices**:
  - Anbernic RG35XXSP (Clamshell with magnetic Hall-effect lid sensor)
  - Anbernic RG35XX Pro (Dual analog sticks, left stick D-pad routing)
  - Anbernic RG35XX Plus (Standard vertical layout)
  - Anbernic RG35XX-H (Horizontal layout with dual analog sticks)
  - Anbernic RG28XX (Compact vertical layout)
  - Anbernic RG34XX & RG34XXSP

### Core Architectural Principles
- **Pure Rust Direct Rendering**: Direct Linux DRM/KMS + EGL/GLES2 rendering pipeline bypassing heavy desktop servers (no X11, no Wayland compositor overhead).
- **Rock-Solid 60 FPS Interface**: Lock-free triple buffering with sub-millisecond input polling and minimal frame latency.
- **Multithreaded Isolation**:
  - Main UI / Render thread: Composites the 3D cartridge shelf and handles user navigation.
  - Core Execution thread: Executes libretro emulator cores in an isolated environment.
  - Background Audio Worker: Low-latency ALSA ring-buffer with anti-stall safeguards.
  - Network & I/O Worker: Handles OTA downloads, RetroAchievements API calls, and Web Dashboard requests asynchronously without dropping frames.

---

## 2. Skeuomorphic Cartridge & Media Engine

Every game in your library is rendered as an authentic, tactile physical medium tailored to its historical platform.

### Game Boy Advance (GBA)
- **Authentic AGB Shell**: Faithfully curved shoulders, top lip recess, and official front indentation.
- **Plastic Shell Finishes**:
  - **Solid Opaque**: High-durability matte injection-molded plastic with ambient occlusion and bevel highlights.
  - **Clear / Translucent**: Multi-layer alpha transparency showing the internal logic board.
  - **Glitter**: Real-time metallic sparkle flecks suspended inside translucent colored plastic.
- **Color Palette**: 8 authentic cartridge pigments (Neutral Grey, Ruby Red, Sapphire Blue, Emerald Green, Gold, FireRed, LeafGreen, Jet Black).
- **Internal PCB Board View**: Realistically rendered circuit board beneath clear shells, featuring gold trace routings, surface-mount capacitors, mask ROM IC, battery retainers, and the GBA connector edge.
- **Label Recess**: Micro-embossed border with realistic high-gloss label finish and specular reflection.

### Game Boy & Game Boy Color (GB / GBC)
- **DMG-01 Grey Pak**: Classic notched cartridge with top finger grip recesses, debossed "GAME BOY" header, and authentic matte off-white finish.
- **GBC Curved Bubble Pak**: Protruding top bubble grip, "GAME BOY COLOR" badge, and translucent resin styling.
- **Custom DMG Color Palettes**: Switchable color profiles for monochrome GB titles:
  - Original Pea Soup (authentic DMG-01 green-tinted passive LCD)
  - Pocket Grey (high-contrast monochrome Game Boy Pocket)
  - Light Cyan (Game Boy Light electroluminescent glow)
  - Warm DMG, BGB, and Super Game Boy classic color profiles.
  - In-game cycling via hotkey without restarting.

### Nintendo Entertainment System (NES)
- **Standard Ribbed Pak**: Full-scale tall grey NES cartridge with authentic horizontal vent ribs along the left column.
- **Authentic Label Well**: Deep recessed label bed with 3.5px bottom-corner rounding and molded insertion direction arrow.
- **72-Pin Edge Connector**: Metallic contacts visible along the bottom cartridge opening.

### Super Nintendo Entertainment System (SNES)
- **US SNES Cartridge**: Iconic two-tone curved front cartridge with side wing recesses and front locking groove.
- **Label Framing**: Expansive 223×99 label well with 3px corner radius and official purple/black system header banner.

### Sony PlayStation (PS1)
- **120mm Compact Disc**: Authentic black-bottom CD-ROM (PlayStation optical discs) with precision radial track grooves.
- **Polycarbonate Spindle Ring**: Clear center clamping ring and transparent spindle hub.
- **Dynamic Specular Sheen**: Real-time holographic rainbow specular reflection that shifts dynamically across the disc surface.
- **Multi-Disc Swapping Engine**: Complete `.m3u` playlist integration with virtual disc eject, disc selection, and tray close directly in the Quick Menu.

### Nintendo DS (NDS)
- **NTR-005 Card Mould**: Proportional 300×318 canvas seated to 240×254 on shelf for seamless shelf alignment.
- **Authentic Keying Chamfer**: Precision 45° bottom-left corner chamfer (20px) preventing incorrect insertion, matching physical DS game cards.
- **Debossed Arrow & Header**: Molded downward thumb insertion arrow and official "NINTENDO DS®" top header.
- **Intelligent Dual Framing**: Full-bleed square cartridge stickers (aspect $\le 1.05$) fit the inner sticker well cleanly, while rectangular box art (aspect $> 1.05$) is neatly framed between the official header and bottom lip.

---

## 3. Artwork & Media Discovery Pipeline

Fragile features a zero-configuration, multi-tier media scanner that automatically locates, parses, and aligns artwork across SD cards:

1. **Cartridge Sticker Art (Primary)**:
   - Scanned from `media/cartart/`, `media/carts/`, and `cartart/`.
   - Applied directly to the cartridge label well with platform-specific corner radiuses and chamfers.
2. **Cover & Box Art (Secondary)**:
   - Scanned from `media/boxart/`, `media/images/`, `media/covers/`, `boxart/`, and `Named_Boxarts/`.
   - Dynamically fitted into the cartridge label window with aspect-aware letterboxing or framing.
3. **Procedural Cart Generation (Fallback)**:
   - When no external art is present, Fragile procedurally generates an authentic, typography-driven cartridge label using clean vector type, system logos, and publisher accents.
4. **Artwork Customizer in Web Dashboard**:
   - Upload custom labels directly from any phone, tablet, or PC via the browser.

---

## 4. Emulation Systems & Cores

Fragile bundles optimized aarch64 cores tuned specifically for the Allwinner H700 quad-core processor:

| System | Primary Core | Secondary Core | Key Features |
| :--- | :--- | :--- | :--- |
| **Game Boy Advance** | mGBA | gpSP | Solar sensor simulation, RTC, cycle accuracy, dynamic recompiler |
| **Game Boy** | Gambatte | — | Cycle-accurate DMG timing, custom palette pipeline, boot ROM support |
| **Game Boy Color** | Gambatte | — | CGB color correction, internal palette banks, link cable emulation |
| **NES** | FCEUmm | — | Broad mapper support (000–255), Famicom Disk System, NSF audio |
| **Super Nintendo** | Snes9x | Supafaust | High compatibility, MSU-1 high-res audio, Super FX chip emulation |
| **PlayStation 1** | PCSX ReARMed | — | NEON-accelerated renderer, CHD track-1 streaming, dynamic recompiler |
| **Nintendo DS** | DSperate | melonDS | Dual-screen touch simulation, 3D NEON rasterizer, microphone trigger |

### Display & Video Enhancements
- **Integer Scaling**: Pixel-perfect scaling mode toggle avoiding interpolation shimmer.
- **GLSL Shaders**:
  - CRT Scanlines: Authentic cathode-ray tube phosphor lines and aperture mask.
  - LCD Grid V2: Authentic handheld dot-matrix sub-pixel grid with adjustable grid strength.
  - GBA Color: Gamma and luminance correction reproducing the original non-backlit AGB-001 screen gamut.
- **System Bezels**: Custom high-resolution decorative bezels filling letterbox areas on non-4:3 titles.
- **Adaptive Frame Pacing**: Dynamic frame pacing engine with audio ring-buffer feedback preventing frame drops and micro-stutter.

---

## 5. In-Game Controls, Hotkeys & Gestures

### Global Hotkey Combinations

| Action | Hotkey Chord | Description |
| :--- | :--- | :--- |
| **Quick Menu / Pause** | `Menu` or `Select + Start` | Opens the Fragile in-game menu overlay |
| **Fast Forward (Hold/Toggle)** | `Select + R2` | Accelerates emulation speed (latches in Toggle mode) |
| **Rewind** | `Select + L2` | Smooth continuous circular buffer gameplay rewind |
| **Quick Save** | `Select + R1` | Immediately writes state to the active slot |
| **Quick Load** | `Select + L1` | Immediately loads state from the active slot |
| **Visual State Switcher** | `Select + Up / Down` | Cycles through save state slots with 3x live screenshots |
| **Screenshot** | `Select + X` | Captures high-res screenshot to Polaroid photo gallery |
| **DMG Palette Cycle** | `Select + Left / Right` | Cycles color palettes for original Game Boy titles |
| **Exit Game** | `Select + Menu` | Safely flushes battery save and returns to shelf |

### Fast Forward Toggle & Hold Modes
- Configurable under **Quick Menu > Settings > Controls > Fast Forward Mode**:
  - **Hold Mode**: Emulation accelerates while holding `Select + R2` and resumes standard speed upon release.
  - **Toggle Mode**: Tapping `Select + R2` latches fast forward active across button releases; tapping again toggles back to 1.0x normal speed.
- Speed multiplier selectable: 2x, 4x, 8x, or Uncapped.

### Nintendo DS Touch Cursor (Virtual Stylus)
- **Navigation**: Hold `L2` + `D-Pad` to move the virtual on-screen stylus across the bottom DS touch screen.
- **Touch / Tap**: Press `A` while holding `L2` to perform a touch tap at the cursor position.
- **Drag & Slide**: Hold `A` while moving the D-Pad to simulate dragging across the touch digitizer.
- **Cursor Reset**: Releasing `L2` hides the cursor overlay without obscuring the screen.

### RG35XXSP Clamshell & Magnetic Hall Sensor
- **Lid Close**: Immediately pauses emulation, mutes audio, powers down screen backlight, and enters low-power suspend.
- **Lid Open**: Instantly wakes display, unpauses emulation, and restores audio with zero pop.
- **Sleep Breathing LED**: Configurable pulsating front power LED during clamshell sleep.

---

## 6. RetroAchievements Integration

Fragile includes a native RetroAchievements client (`slot-cheevos`) connected directly to retroachievements.org:

- **Softcore & Hardcore Modes**: Supports both Hardcore (leaderboards, no save states/rewind) and Softcore modes.
- **Rich ROM Hashing Engine**:
  - Direct cartridge ROM hashing for GBA, GB, GBC, NES, and SNES.
  - **Sony PlayStation CHD Reader**: Custom ISO9660 track-1 reader reading sectors from `.chd` files directly and traversing `.m3u` playlists without needing extraction.
  - **Transparent Zip ROM Decompression**: Automatically decompresses and hashes uncompressed ROM payloads inside `.zip` archives (e.g. NDS and GBA) so zipped games are recognized instantly.
- **Core Memory Mapping**: Full RAM mapping across System RAM, Save RAM, and WRAM across all cores (mGBA, Gambatte, FCEUmm, Snes9x, PCSX ReARMed, DSperate).
- **On-Screen Toast Badges**: Real-time retro notification toasts upon unlocking achievements, showing achievement title, description, and Gamerscore points.
- **Offline Unlocks**: Caches unlocks locally if Wi-Fi drops and synchronizes with RetroAchievements servers upon reconnecting.

---

## 7. Storage Endurance & Crash Protection

Handheld retro consoles frequently suffer from corrupt SD cards due to abrupt power loss. Fragile employs an enterprise-grade storage resilience engine:

- **Decoupled Autosaves**:
  - **Battery Save RAM (`.sav`)**: Flushed to disk every 60 seconds of continuous play to prevent lost progress.
  - **Save State (`.state`)**: Flushed every 300 seconds to minimize SD card write cycles and flash wear.
- **Atomic File Staging**:
  - Every save, configuration, and state file is written to an isolated `.tmp` file and committed via atomic POSIX `rename()`. No file is ever half-written.
- **Explicit `syncfs` Flushing**:
  - Forces OS-level filesystem barrier commits before confirming save toasts.
- **3-Crash Automatic Rollback**:
  - Boot counter and crash markers track startup stability. If an update or faulty core crashes 3 times consecutively, Fragile automatically rolls back to the previous known-good binary.
- **Bundled `fsck.exfat`**:
  - Embedded static exFAT filesystem repair tool automatically checks and repairs dirty card flags on boot.

---

## 8. Over-The-Air (OTA) Updates

Update Fragile directly over Wi-Fi without removing the SD card or using a computer:

- **One-Touch Check**: Navigate to **Quick Menu > Network > System Update**.
- **Cryptographic Verification**:
  - Downloads release metadata and compares local SHA-256 with the remote GitHub Release hash.
- **Background Streaming**:
  - Chunked background download showing real-time progress bar.
- **Atomic Staging**:
  - Downloads directly into `/mnt/sdcard/slot.update`.
  - Atomically replaces the active executable upon restart, guaranteeing an interrupted download will never brick the installation.

---

## 9. Web Dashboard & Network Suite

When connected to local Wi-Fi, Fragile hosts a responsive, modern Web Dashboard accessible from any web browser on your network at `http://<device-ip>:8080`:

### Cart Studio
- **3D Real-Time Cartridge Viewer**: Interactive 3D cartridge rendered via WebGL.
- **Finish & Color Tuning**: Adjust plastic colors, transparency alpha, and glitter particle density in real time.
- **Label Designer**: Upload custom cartridge labels and crop directly to the system mould.

### Library & Media Manager
- **Dual View Modes**: Switch between floating 3D Cartridge shelf and flat Box Cover Art grid.
- **System Filtering**: Browse by platform (GBA, GB, GBC, NES, SNES, PS1, NDS) or search by title.
- **Drag-and-Drop ROM Dropper**: Upload ROMs, BIOS files, and `.m3u` playlists directly into designated system directories.

### Save Manager
- Download and backup `.sav`, `.srm`, `.dsv`, and `.state` files to your PC.
- Restore or swap battery saves seamlessly without manual card mounting.

### Device Telemetry
- Real-time battery level percentage and charging state (dynamic PMIC detection across AXP2202, AXP20x sysfs).
- CPU clock speed, thermal sensor readings, RAM usage, and SD card capacity gauges.

### Wireless Networking
- **Wi-Fi Manager**: Scan for SSIDs, connect to WPA2/WPA3 networks, and toggle Wi-Fi radio.
- **Bluetooth Manager**: Pair wireless Bluetooth headphones (A2DP via BlueALSA) and external wireless gamepads.
- **Two-Device Link Cable**: Wireless ad-hoc GBA link cable emulation between two Fragile handhelds over local Wi-Fi.

---

## 10. Screenshots & Polaroid Gallery

- **Instant Hotkey Capture**: Tap `Select + X` at any point during gameplay to capture a pixel-perfect PNG screenshot.
- **Polaroid-Style Photo Viewer**:
  - View screenshots framed as retro Polaroid instant photos.
  - Displays game title, platform badge, timestamp, and resolution.
- **In-Game Carousel**: Flip through previous captures using `Left` and `Right`.
- **Photo Management**: Delete unwanted screenshots directly from the gallery with a confirmation prompt.

---

## 11. Power Management & Screensavers

- **Multi-Stage Idle Power Saving**:
  1. **Backlight Dim**: Lowers brightness after configurable inactivity (1–5 min).
  2. **Screensaver**: Activates visual screensaver after extended idle.
  3. **Deep Sleep**: Powers down screen and enters low-power SoC sleep.
  4. **Auto-Shutdown**: Powers off the device cleanly after prolonged sleep to prevent complete battery drain.
- **Screensaver Visual Modes**:
  - **Subtle Fade**: Softly dims the cartridge shelf into darkness.
  - **Retro Logo Bouncer**: Floating skeuomorphic logo gently drifting across the screen.
  - **OLED Blackout**: Complete display shutoff for minimal power draw.
- **Instant Wake**: Tap any button or open the RG35XXSP lid to resume instantly with zero audio crackle.

---

## 12. In-Game Quick Menu & Overlays

Press `Menu` or `Select + Start` during gameplay to access the non-blocking overlay:

- **State Management**: Save State, Load State, and Select State Slot (0–9) with visual snapshot preview.
- **Disc Management**: Select disc index, eject disc, swap to next disc, and insert disc for multi-CD PS1 games.
- **RetroAchievements List**: View all achievements for the current game, track progress, and inspect locked/unlocked badges.
- **Cheat Codes**: Toggle GameShark, Action Replay, and Game Genie `.cht` cheats on the fly.
- **Audio & Video Settings**:
  - Brightness and Volume sliders.
  - Integer Scaling toggle.
  - GLSL Shader selector (None, CRT Scanlines, LCD Grid V2, GBA Color).
  - Fast Forward speed and mode (Hold vs. Toggle).
- **Restart & Quit**: Soft-reset game, return to cartridge shelf, reboot system, or power down.

---

## 13. Supported File Formats

| System | Extensions | Notes |
| :--- | :--- | :--- |
| **Game Boy Advance** | `.gba`, `.zip`, `.7z` | Auto-detects save type (SRAM, Flash 64K/128K, EEPROM) |
| **Game Boy** | `.gb`, `.zip`, `.dmg` | Supports monochrome boot ROMs and custom palettes |
| **Game Boy Color** | `.gbc`, `.zip` | Full CGB color support |
| **Nintendo NES** | `.nes`, `.zip`, `.fds` | Supports standard iNES and NES 2.0 headers |
| **Super Nintendo** | `.sfc`, `.smc`, `.zip` | Supports BS-X and MSU-1 enhanced audio |
| **Sony PlayStation** | `.chd`, `.m3u`, `.cue`, `.iso`, `.bin`, `.pbp` | Single-file CHD and multi-disc `.m3u` playlists |
| **Nintendo DS** | `.nds`, `.zip` | Dual screen, virtual touch cursor, mic simulation |
| **Cheat Files** | `.cht` | Standard RetroArch cheat file format |
| **Save Files** | `.sav`, `.srm`, `.dsv` | Seamless cross-compatibility with desktop emulators |

---

*Fragile OS is designed for enthusiasts who appreciate the tactile history of physical cartridges combined with modern operating system resilience.*
