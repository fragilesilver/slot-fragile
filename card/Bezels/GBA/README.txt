# Game Boy Advance Overlays

Add .png overlay files to this folder.
Pick one in Settings > Screen > Game Boy Advance > Overlay, or in the in-game menu while playing.

There is NO coverage or alpha limit: transparent-window frames, full-panel scanlines,
or LCD pixel grids are all supported.

Recommended resolution:
  640 x 480 (4:3 devices)
  720 x 480 (3:2 devices)

Game viewport alignment on 640 x 480 screens:
  Fit / Sharp Fit: x 0, y 27, 640 x 427
  Integer Scale (2x): x 80, y 80, 480 x 320

Companion file for Integer Scale:
Provide "<Name> 2x.png" (e.g. "Scanlines 2x.png" for "Scanlines.png").
It will be drawn automatically when Integer Scale is enabled.
