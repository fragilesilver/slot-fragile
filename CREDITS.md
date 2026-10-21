# Credits

Fragile is [slot](https://github.com/BrandonKowalski/slot) 1.8.0 by Brandon T. Kowalski, with
features from community forks. Every fork named here, and slot itself, is GPL-3.0-or-later.
Each ported file or function says where it came from in a `Ported from` comment.

| From | What |
| --- | --- |
| [nickclyde/slot-plus](https://github.com/nickclyde/slot-plus) | The on-screen keyboard (`slot-ui/src/keyboard.rs`); seating the picked one of two same-named carts; dropping a cart face built for a cart that has since moved (`slot/src/cart_faces.rs`); the idea of stepping the Wi-Fi aside for a link session (its cheevos `Watcher`); search and the list shelves from `Config/Lists` (`slot-store/src/find.rs`, `slot-store/src/collections.rs`, `slot/src/app/shelves.rs`, `slot/src/app/shelves/search.rs`) and their tests |
| [roygravy/slot](https://github.com/roygravy/slot) | The Linux EGL headless surface (`slot-gfx/src/headless_linux.rs`); cheats from RetroArch `.cht` files, found per game and remembered per game (`slot/src/cheats.rs`, `RetroCore::set_cheats`); Motion Blur through the cores' frame blending (`slot/src/core.rs`); the idea of the See-Through LCD card shader; layout ideas for the Button Guide |
| [cloudwithax/unslot](https://github.com/cloudwithax/unslot) | Ideas for the list page (`slot-ui/src/list_page.rs`); each system as a row of data (`slot-store/src/spec.rs`) and the cores it names (`slot-store/src/core.rs`); a core's picture at its own size (`slot-retro/src/libretro.rs`); Wi-Fi: the supplicant's control socket, scanning, joining and saved networks (`slot/src/wifi.rs`), and `card/System/slotwifi.sh` with its test |
| [marghost/slot_modded](https://github.com/marghost/slot_modded) | Each shelf's remembered place, the start shelf, hiding Favorites (`slot/src/app/extras.rs`, `slot/tests/shelf_memory.rs`); the 15% and 10% battery warnings and the battery badge (`slot/src/app/battery.rs`); fast forward at its normal pitch (`slot/src/pitch.rs`); Integer Scale (`slot-gfx/src/bezel.rs`, `slot/src/app/screen.rs`) and the Classic GB/GBC bezels on the card (art by mugwomp93, CC BY-SA 4.0, see `licenses/`); the held-direction speed-up upstream adopted in 1.8; layout ideas for the Button Guide (`slot/src/app/guide.rs`) |
| [markbruno/slot-plus](https://github.com/markbruno/slot-plus) | The 12 hour clock (`slot-ui/src/clock.rs`, `slot-ui/src/polaroids.rs`); RetroArch `.glsl` shaders from the card (`slot-gfx/src/retroshader.rs`, `slot-store/src/shaders.rs`) |
| [Baggio94/slot-konkr](https://github.com/Baggio94/slot-konkr) | The GBA Color and LCD Grid V2 shader ports the card shaders are made from (`card/Shaders/`) |
| [fragilesilver/slot-for-android](https://github.com/fragilesilver/slot-for-android) | The mGBA patch that takes a master code's hook out when cheats are cleared (`cores/mgba/zzz-cheat-reset-removes-hooks.patch`); sending the core only the cheats that are on; the plain `CODE # name` cheat lists (its cheat design is after RetroGame66/slot's) |
| [suprsomething/slot.advance](https://github.com/suprsomething/slot.advance) | Card free space (`slot/src/card_space.rs`); per-game settings (`slot-store/src/game_profile.rs`) |

## Cores

Each core's licence text is in `licenses/`. `slot-store/src/core_credits.rs` holds the same table,
and About > Details lists it.

| Core | By | Licence |
| --- | --- | --- |
| [mGBA](https://github.com/libretro/mgba) | Jeffrey Pfau and contributors | MPL-2.0 |
| [gpSP](https://github.com/libretro/gpsp) | Exophase, notaz and the libretro team | GPL-2.0 |
