# Overlays & Bezels

Add PNG overlay files to the system subfolder:
  GBA/
  GB/
  GBC/
  NES/
  SNES/
  PS/   (or PS1/)
  NDS/

Pick an overlay in Settings > Screen > <System> > Overlay, or in the in-game menu while playing.

There is NO coverage or alpha limit: transparent-window bezels, full-panel scanlines,
CRT shadow masks, and pixel grids are all fully supported.

Make your overlay match your screen resolution:
  640 x 480: RG35XX, RG35XX Plus, RG35XX-H, RG35XX Pro, RG35XXSP, RG28XX
  720 x 480: RG34XX, RG34XXSP, RG SP

Companion files for Integer Scale:
For non-native systems (GB, GBC, GBA), you can provide an integer companion file
named "<Name> {k}x.png" (e.g. "Grid 2x.png" for GBA on 640x480, or "Classic 3x.png" for GB/GBC).
The companion is used automatically whenever Integer Scale is enabled. Companions are hidden
from the menu list.

For native systems (NES, SNES, PS1, NDS), overlays are drawn when Integer Scale is off,
and hidden when Integer Scale is enabled.
