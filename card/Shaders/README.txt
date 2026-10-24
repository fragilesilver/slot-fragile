# Shaders

Single-pass RetroArch shaders (.glsl) for the game picture. Pick one in Settings > Screen >
Game Boy Advance (or Game Boy, or Game Boy Color) > Card Shader. It is drawn instead of the
Shader picked above it. Off goes back to that one.

Fragile comes with:

  Blend Grid        Each frame blended with the one before, and a faint pixel grid
  GBA Color         The darker, softer colours of the original GBA's unlit screen
  LCD Grid V2       Each pixel as red, green and blue stripes, with a dark grid
  See-Through LCD   Pale pixels let the bare panel show through, dark ones cast a shadow

Add your own: any one-pass .glsl in RetroArch's format. A shader is given the frame
(Texture), the frame before (PrevTexture), TextureSize, InputSize, OutputSize and FrameCount.

Shaders can declare tunable parameters:

  #pragma parameter NAME "Label" default min max step

When a shader has parameters, an in-game "Shader Settings" row appears under Card Shader in
Settings > Screen. Left and Right tune parameters live over the running game, and Reset restores
the defaults. Saved values are kept in Config/Shaders/<name>.txt and shared across systems.

Add the line

  #pragma slot_filter linear

to sample the frame smoothly rather than by nearest pixel. A file that does not compile shows
"Card shader failed to load", keeps the built-in Shader on, and writes the reason to
Logs/slot.log.

For a smeared, ghosting picture without any shader, use Motion Blur on the same page: the
emulator blends frames itself.
