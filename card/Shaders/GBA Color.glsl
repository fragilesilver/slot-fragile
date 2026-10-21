// GBA Color: the colours of the original GBA's unlit screen, darker and less saturated than a
// modern panel shows them. The colour matrix and gammas are the official gba-color shader's,
// "Color Mangler" by hunterk, modified by Pokefan531, public domain, as used by Baggio94's
// slot-konkr presets. https://github.com/libretro/glsl-shaders/tree/master/handheld

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

void main()
{
    vec3 c = pow(max(COMPAT_TEXTURE(Texture, TEX0.xy).rgb, vec3(0.0)), vec3(3.2));
    c = mat3(0.82, 0.125, 0.195, 0.24, 0.665, 0.075, -0.06, 0.21, 0.73) * (c * 0.94);
    FragColor = vec4(pow(clamp(c, 0.0, 1.0), vec3(1.0 / 2.2)), 1.0);
}
#endif
