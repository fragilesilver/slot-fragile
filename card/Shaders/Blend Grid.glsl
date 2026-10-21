// blend-grid: an example shader for slot's Shaders/ folder, in RetroArch's single-pass
// GLSL format, so it loads unchanged in RetroArch as well.
//
// Two things a GBA screen did that a backlit panel does not:
//   1. It was slow. Each frame bled into the next, and games leaned on that: many flicker a
//      sprite on alternate frames to fake transparency. Blending with the previous frame
//      (PrevTexture) gives that back.
//   2. It had visible gaps between pixels. A faint dark line at the edge of each 3x3 cell
//      draws the grid without the colour fringing of a full subpixel mask.
//
// Same licence as slot.

#pragma parameter BLEND "Frame blend" 0.5 0.0 1.0 0.05
#pragma parameter GRID "Grid strength" 0.25 0.0 1.0 0.05

#if defined(VERTEX)

#if __VERSION__ >= 130
#define COMPAT_VARYING out
#define COMPAT_ATTRIBUTE in
#else
#define COMPAT_VARYING varying
#define COMPAT_ATTRIBUTE attribute
#endif

#ifdef GL_ES
#define COMPAT_PRECISION mediump
#else
#define COMPAT_PRECISION
#endif

COMPAT_ATTRIBUTE vec4 VertexCoord;
COMPAT_ATTRIBUTE vec4 TexCoord;
COMPAT_VARYING vec4 TEX0;

uniform mat4 MVPMatrix;

void main()
{
    gl_Position = MVPMatrix * VertexCoord;
    TEX0 = TexCoord;
}

#elif defined(FRAGMENT)

#if __VERSION__ >= 130
#define COMPAT_VARYING in
#define COMPAT_TEXTURE texture
out vec4 FragColor;
#else
#define COMPAT_VARYING varying
#define FragColor gl_FragColor
#define COMPAT_TEXTURE texture2D
#endif

#ifdef GL_ES
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
#define COMPAT_PRECISION mediump
#else
#define COMPAT_PRECISION
#endif

uniform sampler2D Texture;
uniform sampler2D PrevTexture;
uniform COMPAT_PRECISION vec2 TextureSize;
COMPAT_VARYING vec4 TEX0;

#ifdef PARAMETER_UNIFORM
uniform COMPAT_PRECISION float BLEND;
uniform COMPAT_PRECISION float GRID;
#else
#define BLEND 0.5
#define GRID 0.25
#endif

void main()
{
    vec3 now = COMPAT_TEXTURE(Texture, TEX0.xy).rgb;
    vec3 before = COMPAT_TEXTURE(PrevTexture, TEX0.xy).rgb;
    vec3 rgb = mix(now, before, BLEND * 0.5);

    // Where in its source pixel this fragment sits, 0 to 1 on each axis. The last third of the
    // cell, right and bottom, is the gap between pixels.
    vec2 cell = fract(TEX0.xy * TextureSize);
    float edge = max(step(0.667, cell.x), step(0.667, cell.y));
    rgb *= 1.0 - GRID * edge;

    FragColor = vec4(rgb, 1.0);
}
#endif
