# Cheats

Put a game's cheats in the folder for its console: GBA, GB or GBC. In game, open the menu and
pick Cheats (it shows only when the game has some); A turns a cheat on or off.

RetroArch .cht files work as they come, as in the libretro cheat database:

  GBA/Pokemon - Emerald Version (USA, Europe) (Code Breaker).cht

A file is the game's when it is named like the ROM, with or without a code kind in brackets
after it, or else when it has the same title before the first bracket. All of a game's files
are read into one list.

A plain list works too, one cheat a line, the code then # and its name:

  GBA/Pokemon - Emerald Version (USA, Europe).txt
      D8BAE4D9 4864DCE5 # Master code
      A86CDBA5 19BB1D4C # Max money

Codes go to the core as they are written: mGBA and gpSP take GameShark, Action Replay and
Code Breaker codes for the GBA (gpSP not encrypted ones), and Game Genie and GameShark codes
for the GB and GBC (mGBA). Cheats are left off while two consoles are linked.

Which cheats are on is kept per game in Config/Cheats.
