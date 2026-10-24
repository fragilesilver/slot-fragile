// LCD Grid V2: each game pixel drawn as red, green and blue stripes with a dark grid between,
// for an RGB LCD look. lcd-grid-v2 by cgwg, GPL-2.0-or-later, from
// https://github.com/libretro/glsl-shaders/tree/master/handheld, with the preset's own
// values (gain 1.5, gamma 2.2, colour 0.75), in the form without GLSL 1.30 arrays that
// Baggio94's slot-konkr wrote for GLES 2.

#pragma parameter GRID_DARKNESS "Grid darkness" 0.75 0.0 1.0 0.05

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

#ifdef PARAMETER_UNIFORM
uniform float GRID_DARKNESS;
#else
#define GRID_DARKNESS 0.75
#endif

float integral(float z, float a, float b, float d, float e, float g)
{
    float z2 = z * z;
    return z * (1.0 + z2 * (a + z2 * (b + z2 * (d + z2 * (e + z2 * (g + z2 / 13.0))))));
}

float smear_x(float x, float dx)
{
    dx = max(dx, 0.0001);
    float lo = clamp((x - dx * 0.5) / 1.5, -1.0, 1.0);
    float hi = clamp((x + dx * 0.5) / 1.5, -1.0, 1.0);
    return 1.5 * (integral(hi, -2.0 / 3.0, -1.0 / 5.0, 4.0 / 7.0, -1.0 / 9.0, -2.0 / 11.0)
        - integral(lo, -2.0 / 3.0, -1.0 / 5.0, 4.0 / 7.0, -1.0 / 9.0, -2.0 / 11.0)) / dx;
}

float smear_y(float y, float dy)
{
    dy = max(dy, 0.0001);
    float lo = clamp((y - dy * 0.5) / 0.63, -1.0, 1.0);
    float hi = clamp((y + dy * 0.5) / 0.63, -1.0, 1.0);
    return 0.63 * (integral(hi, 0.0, -4.0 / 5.0, 2.0 / 7.0, 4.0 / 9.0, -4.0 / 11.0)
        - integral(lo, 0.0, -4.0 / 5.0, 2.0 / 7.0, 4.0 / 9.0, -4.0 / 11.0)) / dy;
}

vec3 lit(vec2 at)
{
    return pow(1.5 * COMPAT_TEXTURE(Texture, at).rgb, vec3(2.2));
}

void main()
{
    vec2 xy = TEX0.xy * TextureSize;
    vec2 tli = floor(xy - vec2(0.4999));
    vec2 range = InputSize / max(OutputSize, vec2(1.0));
    float sub = (xy.x - 0.4999 - tli.x) * 3.0;
    float dx = range.x * 3.0;
    vec3 left = vec3(smear_x(sub + 1.0, dx), smear_x(sub, dx), smear_x(sub - 1.0, dx));
    vec3 right = vec3(smear_x(sub - 2.0, dx), smear_x(sub - 3.0, dx), smear_x(sub - 4.0, dx));
    float py = xy.y - 0.4999 - tli.y;
    float top = smear_y(py, range.y);
    float bot = smear_y(py - 1.0, range.y);
    vec2 pixel = 1.0 / TextureSize;
    vec2 at = (tli + vec2(0.5)) * pixel;
    vec3 sum = (lit(at) * left + lit(at + vec2(pixel.x, 0.0)) * right) * top
        + (lit(at + vec2(0.0, pixel.y)) * left + lit(at + pixel) * right) * bot;
    FragColor = vec4(pow(max(sum * GRID_DARKNESS, vec3(0.0)), vec3(1.0 / 2.2)), 1.0);
}
#endif
