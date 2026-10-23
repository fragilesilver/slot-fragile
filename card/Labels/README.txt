# Labels

Add a .png to GBA, GB or GBC, named after the ROM. Labels are the same on
every device.

  Games/GBA/Metroid Fusion.gba  ->  Labels/GBA/Metroid Fusion.png

  Cart              Shape    Size
  GBA               1.94:1   1280 x 661
  Game Boy          1.13:1   1024 x 901
  Game Boy Color    1.12:1   1024 x 901

Other sizes are scaled to fill the label and cropped from the centre.
Cart Studio makes these: https://studio.slot-cfw.fyi

Or let Fragile find them: Main menu > Network > Game Art, with the Wi-Fi on.
Automatic (on unless turned off) finds a label for every game with none, by
its CRC and file name, from libretro-thumbnails; A on All systems (or on one
system) asks at once. Each shows on its cart as soon as it is found. Game
Art keeps its labels in System/Cache/Scraper/Labels, so a label of your own
here always wins and is never touched.

Label Style draws them as Box Art (a disc's art on a PlayStation disc), Cart
Label (the cart's own label as printed) or Logo. A game can keep its own on
the in-game Settings > This Game > Label. A new style is drawn from the art
kept in System/Cache/Scraper/Art, without the internet.

Disc art and printed cart labels come from ScreenScraper alone, asked first
when Config/screenscraper.conf has a developer ID (devid=, devpassword=).
Without one, those styles draw the box. Sign in on Game Art's ScreenScraper
row to add your own account (ssid=, sspassword=) for a bigger daily quota.


# Shells

cart_shell.txt sets a cart's shell, one line per game:

  Metroid Fusion = auto 6a3fa0 solid

  Outline   auto, notched or rounded
  Colour    hex RGB
  Finish    solid, clear or glitter
