# Fragile OS: Complete UI/UX, Navigation & Systems Evolution Guide

This document provides a comprehensive technical and design reference for the user interface, menu architecture, UX paradigms, and evolutionary changes implemented in **Fragile** (from the upstream `slot` v1.8.0 baseline through Fragile v1.9.4).

---

## Table of Contents

1. [UI/UX Philosophy & Tactile Aesthetics](#1-uiux-philosophy--tactile-aesthetics)
2. [Shelf Navigation & Main Interface](#2-shelf-navigation--main-interface)
3. [In-Game Quick Menu & HUD Architecture](#3-in-game-quick-menu--hud-architecture)
4. [UI/UX Polish & Modern Comforts](#4-uiux-polish--modern-comforts)
5. [Controls, Hotkeys & Virtual Interactions](#5-controls-hotkeys--virtual-interactions)
6. [Skeuomorphic Media & Shell Engineering](#6-skeuomorphic-media--shell-engineering)
7. [Web Dashboard Interface](#7-web-dashboard-interface)
8. [Evolutionary Changes: Upstream Slot vs. Fragile](#8-evolutionary-changes-upstream-slot-vs-fragile)
9. [Complete Menu Hierarchy & Row Reference](#9-complete-menu-hierarchy--row-reference)

---

## 1. UI/UX Philosophy & Tactile Aesthetics

Fragile is built upon a single central premise: **a retro gaming frontend should feel like handling the physical consoles and physical media themselves, not navigating a generic Android or Kodi launcher.**

```
+--------------------------------------------------------------------------+
|  TOP BAR: 12h/24h Clock  |  Wi-Fi / BT Icons  |  Real-Time Battery HUD    |
+--------------------------------------------------------------------------+
|                                                                          |
|       [ PLATFORM SELECTOR: GBA | GB | GBC | NES | SNES | PS1 | NDS ]      |
|                                                                          |
|                  .-----------------------.                               |
|                  |   SKEUOMORPHIC 3D     |                               |
|                  |      CARTRIDGE        |                               |
|                  |  (Authentic Mould,    |                               |
|                  |   Gloss Specular,     |                               |
|                  |   Physical Chamfer)   |                               |
|                  '-----------------------'                               |
|                ============================= (Acrylic Shelf Rack)        |
|                                                                          |
+--------------------------------------------------------------------------+
|  FOOTER: Dynamic Action Legends (A: Insert  X: Search  Y: Fav  Menu: Config)|
+--------------------------------------------------------------------------+
```

### Core Design Principles
- **Direct-to-Framebuffer Fluidity**: Rendering directly through Linux DRM/KMS + EGL/GLES2 with no X11 or Wayland compositor layer, locking user interactions to a rigid 60 FPS with sub-millisecond input response.
- **Physical Depth & Materiality**: Every cartridge, pak, and CD-ROM features simulated physical depth—including micro-bevel highlights, shadow drops onto the shelf rack, debossed manufacturer stamps, and glossy specular reflection curves.
- **Visual Restraint & Clarity**: Clean vector typography, fixed column layouts, muted background tones that adjust subtly to match the active system's plastic palette, and contextual single-line helper text underneath every menu item.
- **Zero Modal Interruptions**: Seamless transitions between browsing, menu adjustments, save state switching, and gameplay without jarring loading screens or frame drops.

---

## 2. Shelf Navigation & Main Interface

The primary game selection interface is organized as a tactile, multi-shelf physical carousel.

### Shelf Carousel Mechanics
- **Physics-Based Inertial Scrolling**: Navigating left and right through games features smooth ease-out scrolling curves with progressive acceleration when holding the D-Pad or analog stick.
- **Elevated Selected State**: The active game cartridge smoothly elevates above the rack, casting an ambient occlusion shadow onto the acrylic surface and adjacent cartridges.
- **Insertion & Ejection Sequences**:
  - Selecting a game triggers an authentic physical insertion animation where the cartridge slides downwards into the handheld's cartridge slot before launching the core.
  - Returning from a game cleanly reverses the animation, smoothly ejecting the cartridge back onto the shelf rack.
- **Cartridge Back & PCB View**: Tilting or inspecting a cartridge reveals the reverse plastic shell with safety warnings, serial stamps, and for translucent GBA carts, an exposed golden circuit board with soldered mask ROMs and surface-mount components.

### Shelf Memory & Organization
- **Per-Shelf Memory**: Navigating between platforms (e.g. from GBA to NES or PS1) remembers the exact cartridge position on each system rack. Switching back restores your previous cursor immediately.
- **Fast Rack Switching (`L1` / `R1` or Up / Down)**: Jump directly between system shelves without returning to an abstract main menu.
- **Custom Collections & Lists**:
  - Organize titles into bespoke shelves (e.g., *RPGs*, *Pokemon Romhacks*, *Favorites*) stored in `Config/Lists/`.
  - Add or remove titles on the fly via the `Add to List` shortcut (`CarouselAction::AddToList`).
  - Manage lists directly inside the frontend: create new lists, rename them via the on-screen keyboard, or delete lists.
- **Shelf Search Engine (`X` Button)**:
  - Instant on-screen keyboard filtering across the entire library.
  - As you type, the shelf dynamically populates matching titles in real time.

---

## 3. In-Game Quick Menu & HUD Architecture

Pressing `Menu` or `Select + Start` during gameplay pauses the emulation core and brings up the **Quick Menu** overlay.

```
+--------------------------------------------------------------------------+
|  PLAY MENU                                           [Select + Start]    |
+--------------------------------------------------------------------------+
|  > Save State                                                Slot 0      |
|    Load State                                                Slot 0      |
|    State Switcher                                            1/10        |
|    Screen Settings                                              >        |
|    This Game                                                    >        |
|    Cheats                                                       >        |
|    Achievements                                                 >        |
|    Disc Control (PS1)                                       Disc 1/2     |
|    Reset Game                                                            |
|    Quit to Shelf                                                         |
+--------------------------------------------------------------------------+
|  HELP: Save gameplay progress immediately to the selected state slot.    |
|  LEGENDS: [A] Select   [B] Back   [Left/Right] Adjust Value              |
+--------------------------------------------------------------------------+
```

### Visual & Structural Polish
1. **Translucent Frosted Overlay**: The active game remains visible beneath a blurred, desaturated backdrop, maintaining gameplay context while ensuring menu legibility.
2. **Label-Left / Value-Right Layout**:
   - Option names are anchored to the left margin; current values (sliders, toggles, subpage indicators) are aligned to the right.
   - Selected rows are enveloped in an active pill highlight with subtle pulse animation.
3. **Contextual Single-Line Helper Footer (`QuickRow::help()`)**:
   - Every single row across all 157 menu entries displays a descriptive, plain-language explanation along the bottom plate.
   - Explains exactly what each setting does before you change it (e.g., explaining the difference between *Sharp Fit* and *Integer Scale*, or detailing battery saver timeouts).
4. **Scroll Indicators & Pagination**:
   - Clean micro-arrows indicate hidden rows above or below the 6 visible rows.
   - Smooth vertical scrolling maintains anchor positions without disorienting jumps.

### Visual Save State Switcher
- Navigating to **State Switcher** reveals an interactive carousel of save state slots (0–9).
- Each slot displays a high-resolution 3x gameplay snapshot captured at the exact moment of saving, alongside slot index and timestamp metadata.
- **Undo Safety**: Accidental loads can be instantly reverted via the undo shortcut.

### PlayStation Multi-Disc Controller
- Multi-disc games running via `.m3u` playlists present a dedicated **Disc** row:
  - **Eject Virtual Tray**: Simulates opening the PS1 disc door.
  - **Select Disc**: Cycles through Disc 1, Disc 2, Disc 3, etc.
  - **Close Virtual Tray**: Latches the virtual lid, signaling the core to load the new disc track.

---

## 4. UI/UX Polish & Modern Comforts

Fragile introduces extensive UX quality-of-life enhancements designed for daily handheld use.

### Dynamic List Pages (`ListPage`)
Replaces rigid submenus with a unified, lightweight, scrollable list component:
- Used for **Cheats** (toggleable checkmarks), **Wi-Fi Networks** (signal strength meters and security badges), **Bluetooth Devices** (pair / connect / forget actions), **RetroAchievements** (locked vs. unlocked badges with gamerscore), and **Search Results**.
- Supports context-sensitive button legends in the footer (e.g. `[A] Join`, `[X] Forget`, `[Y] Scan`).

### Full-Featured On-Screen Keyboard
- Clean, responsive virtual keyboard matching the console aesthetic.
- **Layers**: Lowercase, Uppercase, Numbers & Symbols.
- **Security**: Wi-Fi passphrases and RetroAchievements passwords feature masked bullet points (`••••••••`) with an optional peek toggle.

### Battery Telemetry & Warning System
- **Real-Time Battery HUD**: Displays a battery outline with dynamic liquid level fill, percentage readout, and charging bolt indicator.
- **Multi-PMIC Sysfs Detection**: Dynamically probes AXP2202 and AXP20x power management chips.
- **Two-Stage Battery Alerts**:
  - **15% Warning**: A subtle amber banner appears momentarily along the bottom edge.
  - **10% Critical Pulse**: A gentle red alert pulses across the HUD, reminding the user to save and connect a charger.

### Polaroid-Style Photo Gallery
- Capturing a screenshot (`Select + X`) renders an authentic instant Polaroid photo.
- In the gallery, screenshots are displayed in heavy-stock white photographic frames featuring:
  - Authentic photo border with handwritten-style game title.
  - Platform emblem and resolution badge.
  - Timestamp in 12-hour or 24-hour format.
  - Carousel navigation with one-button deletion confirmation.

### Toast Notification System
- Minimalist, anti-aliased pill toasts appear along the bottom edge for transient events:
  - State saved / loaded
  - Fast forward mode toggled
  - Screenshot captured
  - RetroAchievements unlocked (with Gamerscore badge)
  - SD card repair / scan results

---

## 5. Controls, Hotkeys & Virtual Interactions

| Shortcut Chord | Function | Interaction Details |
| :--- | :--- | :--- |
| `Menu` or `Select + Start` | **Quick Menu** | Pauses gameplay and opens in-game overlay |
| `Select + R2` | **Fast Forward** | Supports **Hold** mode or **Toggle** mode (latched state) |
| `Select + L2` | **Rewind** | Continuous circular buffer rewind with visual feedback |
| `Select + R1` | **Quick Save** | Writes immediately to the active save state slot |
| `Select + L1` | **Quick Load** | Loads immediately from the active save state slot |
| `Select + Up / Down` | **State Slot Cycle** | Cycles active slot (0–9) with visual snapshot preview |
| `Select + X` | **Screenshot** | Captures pixel-perfect image to Polaroid gallery |
| `Select + Left / Right`| **DMG Palette Cycle** | Cycles color profiles for original monochrome Game Boy |
| `L2 + D-Pad` | **DS Touch Cursor** | Moves virtual stylus on Nintendo DS bottom screen |
| `L2 + A` | **DS Touch Tap** | Taps / clicks at virtual stylus position |
| `L2 + A (Hold)` | **DS Touch Drag** | Drags virtual stylus across touch digitizer |
| `Select + Menu` | **Safe Exit** | Flushes battery SRAM and returns cleanly to shelf |

### Fast Forward Toggle & Hold Modes
- Configurable under **Quick Menu > Settings > Controls > Fast Forward Mode**:
  - **Hold Mode**: Accelerates only while keeping `Select + R2` held.
  - **Toggle Mode**: Tap once to latch fast forward at high speed; tap again to return to 1.0x speed. State persists cleanly across menu opens and gameplay sessions.
- Speeds available: 2x, 4x, 8x, or Uncapped.

### Nintendo DS Touch Simulation
- Enables touch-dependent Nintendo DS titles to be played on non-touch handhelds (RG35XXSP, RG35XX Pro, RG35XX-H).
- The virtual cursor features smooth acceleration curves and hides automatically when `L2` is released.

### Clamshell Sleep Integration (RG35XXSP)
- **Hall Effect Magnetic Lid Sensor**:
  - Closing the lid immediately pauses the core, mutes audio, shuts off the screen backlight, and sets the SoC to low-power suspend.
  - Front power LED pulses in a soft breathing rhythm.
  - Opening the lid instantly wakes the display and unpauses audio/video with zero pop.

---

## 6. Skeuomorphic Media & Shell Engineering

Fragile provides custom-engineered 3D physical models for all 7 supported gaming platforms.

```
+--------------------------------------------------------------------------+
|                     SYSTEM CARTRIDGE ARCHITECTURES                       |
+--------------------------------------------------------------------------+
|  GBA: Curved AGB-002 shell, top lip recess, internal circuit board view  |
|  GB:  DMG-01 off-white pak with thumb notch & debossed "GAME BOY" logo   |
|  GBC: Curved clear bubble pak with "GAME BOY COLOR" badge                |
|  NES: Tall ribbed cart with 3.5px corner radius label well recess        |
|  SNES: US two-tone cart with side wings, 223x99 well, purple banner      |
|  PS1: 120mm black-bottom CD-ROM, polycarbonate hub, shifting sheen      |
|  NDS: NTR-005 card (300x318), 45° BL chamfer, debossed insertion arrow  |
+--------------------------------------------------------------------------+
```

### System Media Specifications

| Platform | Canvas Size | Seated Shelf Size | Label Recess Bounds | Key Physical Traits |
| :--- | :--- | :--- | :--- | :--- |
| **Nintendo DS (NDS)** | 300 × 318 | 240 × 254 | (26, 62, 248, 202) | 20px 45° bottom-left chamfer, 4px corner radius, debossed downward arrow, official header |
| **Game Boy Advance** | 300 × 180 | 240 × 144 | (30, 20, 240, 140) | Curved shoulders, top lip notch, solid/translucent/glitter finishes, internal PCB |
| **Game Boy (GB)** | 270 × 310 | 216 × 248 | (24, 70, 222, 210) | DMG-01 off-white shell, top ribbed grip, debossed header, authentic pea-soup palettes |
| **Game Boy Color (GBC)**| 270 × 310 | 216 × 248 | (24, 70, 222, 210) | Curved top bubble grip, translucent plastics, CGB color correction pipeline |
| **NES** | 270 × 340 | 216 × 272 | (28, 48, 214, 260) | Vertical vent ribs, 3.5px bottom corner radius mask, molded insertion arrow |
| **Super Nintendo (SNES)**| 320 × 200 | 256 × 160 | (48, 30, 223, 99) | Recessed side wings, 3px corner radius, official US purple/black system header |
| **PlayStation (PS1)** | 300 × 300 | 240 × 240 | Ring Label | Black optical disc bottom, clear center spindle clamp, dynamic shifting specular sheen |

### Intelligent Dual Artwork Fitting
- **Square / Wide Cartridge Stickers**: Aspect ratios $\le 1.05$ fill the cartridge label bed completely, masking cleanly to the system-specific corner radiuses and chamfers.
- **Rectangular Front Cover Art**: Aspect ratios $> 1.05$ (e.g. standard DVD or box covers) are cleanly framed between official header banners and bottom lips, avoiding destructive cropping.
- **Procedural Cart Generation**: Titles without artwork receive procedurally rendered vector labels featuring authentic typography, publisher branding, and system emblems.

---

## 7. Web Dashboard Interface

Accessible from any phone, tablet, or PC on your local Wi-Fi network at `http://<device-ip>:8080`:

```
+--------------------------------------------------------------------------+
|  FRAGILE WEB DASHBOARD                                    Device: Online |
+--------------------------------------------------------------------------+
|  [ Cart Studio ]   [ Library & Media ]   [ Saves ]   [ Telemetry ]       |
|                                                                          |
|  +---------------------------+   +------------------------------------+  |
|  |     3D CART STUDIO        |   | PROPERTIES                         |  |
|  |                           |   | System: Nintendo DS                |  |
|  |    [ Interactive 3D ]     |   | Finish: Translucent Glitter        |  |
|  |    [ WebGL Cartridge]     |   | Plastic Color: Ruby Red            |  |
|  |                           |   | Label Upload: [ Browse... ]        |  |
|  +---------------------------+   +------------------------------------+  |
|                                                                          |
|  TELEMETRY: Battery: 88% (Charging) | Temp: 42°C | SD2 Free: 48.2 GB     |
+--------------------------------------------------------------------------+
```

### Features of the Web Dashboard
1. **Interactive Cart Studio**: Full 3D WebGL cartridge renderer replicating the device's physical models in real time. Adjust plastic finishes (Solid, Clear, Glitter), tint colors, and preview custom uploaded stickers before flashing them to the SD card.
2. **Library & Media Manager**: View your library in either floating 3D cartridge shelf view or flat box art grid view. Drag-and-drop ROMs, BIOS files, and artwork directly to device storage.
3. **Save Manager**: Download, backup, or restore `.sav`, `.srm`, `.dsv`, and `.state` files across all systems.
4. **Hardware Telemetry Monitor**: Real-time display of battery percentage, charging state, CPU temperature, RAM usage, and SD card storage metrics.

---

## 8. Evolutionary Changes: Upstream Slot vs. Fragile

The following table contrasts the upstream `slot` baseline with the enhanced **Fragile** operating system:

| Feature Area | Upstream `slot` (v1.8.0) | Fragile OS (v1.9.4) |
| :--- | :--- | :--- |
| **Supported Platforms** | 2 platforms (GB, GBA) | **7 platforms** (GBA, GB, GBC, NES, SNES, PS1, NDS) |
| **Emulation Cores** | mGBA, mock core | **8 tuned cores** (mGBA, gpSP, Gambatte, FCEUmm, Snes9x, Supafaust, PCSX-ReARMed, DSperate) |
| **Cartridge Shells** | Standard GBA cart only | **7 bespoke skeuomorphic moulds** with physical chamfers, vents, optical discs & PCB views |
| **Cartridge Finishes** | Solid plastic only | **Solid, Clear / Translucent, and Glitter** particle finishes with alpha modulation |
| **Menu Layout** | Basic centered rows without descriptions | **Label-Left / Value-Right grid with dynamic 1-line helper text (`QuickRow::help()`)** |
| **Menu Pagination** | Hard-coded page arrays | **Modular pages** with `ListPage` dynamic scrolling for cheats, SSIDs, and devices |
| **On-Screen Keyboard** | None | **Full QWERTY virtual keyboard** with symbols layer and password masking |
| **Fast Forward** | Hold button only | **Hold or Toggle mode** (`Select + R2`), latching active across button releases |
| **Touch Emulation** | None | **Virtual Touch Stylus** (`L2 + D-Pad` cursor, `L2 + A` tap/drag) for Nintendo DS |
| **RetroAchievements** | None | **Softcore & Hardcore client** with PS1 CHD track-1 reading and zip decompression |
| **Multi-Disc Games** | None | **PS1 `.m3u` virtual tray controller** with eject, disc swap, and tray close |
| **Display Scaling** | Fit and 2x/3x Integer | **Fit, Sharp Fit (sharp-bilinear anti-shimmer), and Integer Scale** |
| **Shaders & Tuning** | Static preset shaders | **User `.glsl` shaders with live `#pragma parameter` uniform tuning in menu** |
| **Screenshots** | None | **In-game hotkey capture** with interactive **Polaroid-style photo gallery** |
| **Storage Safety** | Direct writes | **Decoupled autosaves, atomic `.tmp` staging, `syncfs` flushing & 3-crash rollback** |
| **System Updates** | Manual card flashing | **Over-The-Air (OTA) background downloads** with SHA-256 verification and atomic swap |
| **Network Management**| None | **Wi-Fi WPA2/WPA3 supplicant, Bluetooth audio/controllers, SSH/SFTP, and Web Dashboard** |
| **Clamshell (RG35XXSP)**| Basic sleep | **Magnetic Hall-effect lid sleep, breathing sleep LED, and zero-pop audio resume** |

---

## 9. Complete Menu Hierarchy & Row Reference

Fragile organizes its 157 menu rows into a structured, discoverable hierarchy:

```
[MAIN MENU]
├── [GENERAL]
│   ├── Start Shelf (Last Used, GBA, GB, GBC, NES, SNES, PS1, NDS, Favorites)
│   ├── Show Favorites Shelf (On / Off)
│   ├── Clock Format (12-Hour / 24-Hour)
│   ├── Battery Badge (Percentage / Icon / Off)
│   ├── Idle Dim Timeout (1m, 2m, 5m, Off)
│   ├── Screensaver Mode (Fade, Retro Logo, OLED Blackout)
│   ├── Screensaver Timeout (2m, 5m, 10m, Off)
│   └── Auto Sleep / Shutdown Timeout
│
├── [CONTROLS]
│   ├── Button Cheat Sheet (Live dynamically generated mapping sheet)
│   ├── Fast Forward Mode (Hold / Toggle)
│   ├── Fast Forward Speed (2x, 4x, 8x, Uncapped)
│   ├── Keep Fast Forward Pitch (On / Off)
│   ├── Rewind Buffer Size (10s, 30s, 60s, Off)
│   ├── Left Analog as D-Pad (RG35XX-H / Pro)
│   └── System Remap Pages (GBA, GB, NES, SNES, PS1, NDS)
│
├── [SCREEN SETTINGS]
│   ├── Screen: GBA (Scaling: Fit / Sharp Fit / Integer; Shaders; GBA Color; Bezels)
│   ├── Screen: GB  (Scaling; Palettes: Pea Soup, Pocket, Cyan; LCD Grid; Bezels)
│   ├── Screen: GBC (Scaling; CGB Color Correction; LCD Grid; Bezels)
│   ├── Screen: NES (Scaling; Shaders; Bezels)
│   ├── Screen: SNES(Scaling; Shaders; Bezels)
│   ├── Screen: PS1 (Scaling; Shaders; Dithering)
│   └── Screen: NDS (Screen Layout: Top/Bottom, Side-by-Side, Focus; Touch Cursor Speed)
│
├── [NETWORK & SERVICES]
│   ├── Wi-Fi Power (On / Off)
│   ├── Wi-Fi Networks (Scan & Join SSIDs via On-Screen Keyboard)
│   ├── Wi-Fi Status & IP Address Display
│   ├── Bluetooth Radio (On / Off)
│   ├── Bluetooth Devices (Pair headphones & controllers via slotbt.sh)
│   ├── SSH / SFTP Server (On / Off with dynamic root password display)
│   ├── Web Dashboard (On / Off at port 8080)
│   ├── Network Time Sync (NTP synchronization)
│   ├── RetroAchievements (Enable, Hardcore/Softcore, Login via Keyboard)
│   ├── Game Art Scraper (Download missing cartridge & box art over Wi-Fi)
│   └── System Update (OTA background check, download & atomic apply)
│
└── [ABOUT & SYSTEM INFO]
    ├── Fragile Release Version (v1.9.4)
    ├── BaseOS Kernel & System Build
    ├── Hardware Target (RG35XXSP / Pro / Plus / H)
    ├── Library Statistics (Total games per system)
    └── Storage Capacity (SD1 OS free space, SD2 content free space)
```

```
[IN-GAME QUICK MENU (PLAY)]
├── Save State (Select slot 0-9 and write)
├── Load State (Select slot 0-9 and load)
├── State Switcher (Visual 3x thumbnail carousel with undo support)
├── Disc Control (PS1 only: Eject, Select Disc 1-N, Insert)
├── Screen Settings (Live Scaling, Shader selector, and Shader Parameter tuning)
├── This Game (Per-game overrides: Scaling, Shaders, Palettes, FF Speed, Custom Label)
├── Cheats (Toggle GameShark/ActionReplay .cht codes with per-game persistence)
├── Achievements (Live in-game achievement badges and Gamerscore progress)
├── Reset Game (Soft reset emulation core)
└── Quit to Shelf (Safely commit battery SRAM and exit)
```

---

*This guide documents the full design, user interface mechanics, and technical architecture of Fragile OS.*
