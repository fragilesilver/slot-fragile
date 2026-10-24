// See-Through LCD: the reflective screen of a Game Boy or an early GBA. Pale pixels let the
// bare panel behind them show through, and dark pixels cast a faint shadow down and to the
// right onto it, as liquid crystal over a mirror does. A Fragile shader after roygravy/slot's
// See-Through Pixels. Same licence as slot (GPL-3.0-or-later).

#pragma parameter REFLECT "Reflection" 0.62 0.0 1.0 0.05
#pragma parameter SHADOW "Shadow depth" 0.22 0.0 0.6 0.02
#pragma parameter GRID "Grid" 0.12 0.0 0.5 0.02

#if defined(VERTEX)

#if __VERSION__ >= 130
#define COMPAT_VARYING out
#define COMPAT_ATTRIBUTE in
#else
#define COMPAT_VARYING varying
#define COMPAT_ATTRIBUTE attribute
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
#endif

uniform sampler2D Texture;
uniform vec2 TextureSize;
uniform vec2 InputSize;
uniform vec2 OutputSize;
COMPAT_VARYING vec4 TEX0;

const vec3 PANEL = vec3(0.74, 0.76, 0.70);
const vec3 LUMA = vec3(0.2126, 0.7152, 0.0722);

#ifdef PARAMETER_UNIFORM
uniform float REFLECT;
uniform float SHADOW;
uniform float GRID;
#else
#define REFLECT 0.62
#define SHADOW 0.22
#define GRID 0.12
#endif

void main()
{
    vec2 texel = 1.0 / TextureSize;
    vec3 c = COMPAT_TEXTURE(Texture, TEX0.xy).rgb;
    float ink = 1.0 - smoothstep(0.05, 0.88, dot(c, LUMA));
    vec3 lcd = mix(PANEL, c, mix(REFLECT, 0.97, ink));
    float caster = 1.0 - dot(COMPAT_TEXTURE(Texture, TEX0.xy - texel).rgb, LUMA);
    lcd *= 1.0 - SHADOW * caster * (1.0 - ink);
    vec2 cell = fract(TEX0.xy * TextureSize);
    vec2 line = InputSize / max(OutputSize, vec2(1.0));
    float gap = max(step(cell.x, line.x), step(cell.y, line.y));
    FragColor = vec4(lcd * (1.0 - GRID * gap), 1.0);
}
#endif
