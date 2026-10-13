#version 300 es
// TMP_SDF.shader's PixShader with TMPro.cginc's GetColor (no keywords; the
// face and outline textures are Unity's default white).
precision highp float;

uniform sampler2D u_atlas;
uniform vec2 u_sample_offset;
uniform float u_anisotropy;
uniform vec4 u_face_color;
uniform vec4 u_outline_color;
uniform float u_scale_ratio_a;
uniform float u_outline_width;
uniform float u_outline_softness;

in vec4 v_color;
in vec2 v_atlas;
in vec4 v_param;

out vec4 fragment;

vec4 face_and_outline(float d, vec4 face_color, vec4 outline_color, float outline, float softness) {
    float face_alpha = 1.0 - clamp((d - outline * 0.5 + softness * 0.5) / (1.0 + softness), 0.0, 1.0);
    float outline_alpha = clamp(d + outline * 0.5, 0.0, 1.0) * sqrt(min(1.0, outline));

    face_color.rgb *= face_color.a;
    outline_color.rgb *= outline_color.a;

    face_color = mix(face_color, outline_color, outline_alpha);

    face_color *= face_alpha;

    return face_color;
}

// The atlas value at the pixel centre: one bilinear tap or, with anisotropic
// filtering, `taps` of them along the footprint's major axis, one texel apart
// or closer when the footprint is shorter.
float distance_at_centre() {
    vec2 atlas = v_atlas - dFdx(v_atlas) * u_sample_offset.x - dFdy(v_atlas) * u_sample_offset.y;
    vec2 size = vec2(textureSize(u_atlas, 0));
    vec2 dx = dFdx(v_atlas) * size;
    vec2 dy = dFdy(v_atlas) * size;
    float px = length(dx);
    float py = length(dy);
    float taps = clamp(floor(max(px, py) / max(min(px, py), 1e-6) + 0.25), 1.0, max(u_anisotropy, 1.0));
    float spacing = min(max(px, py) / taps, 1.0);
    vec2 axis = (px >= py ? dx / max(px, 1e-6) : dy / max(py, 1e-6)) * spacing / size;
    float c = 0.0;
    for (int tap = 0; tap < 16; tap++) {
        if (float(tap) >= taps) {
            break;
        }
        c += textureLod(u_atlas, atlas + axis * (float(tap) - (taps - 1.0) * 0.5), 0.0).r;
    }
    return c / taps;
}

void main() {
    float c = distance_at_centre();

    if (c - v_param.x < 0.0) {
        discard;
    }

    float scale = v_param.y;
    float bias = v_param.z;
    float sd = (bias - c) * scale;

    float outline = (u_outline_width * u_scale_ratio_a) * scale;
    float softness = (u_outline_softness * u_scale_ratio_a) * scale;

    vec4 face_color = u_face_color;
    vec4 outline_color = u_outline_color;

    face_color.rgb *= v_color.rgb;

    face_color = face_and_outline(sd, face_color, outline_color, outline, softness);

    fragment = face_color * v_color.a;
}
