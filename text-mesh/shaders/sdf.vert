#version 300 es
// TMP_SDF.shader's VertShader (TextMeshPro/Distance Field, no keywords).
precision highp float;

uniform mat4 u_projection;
uniform vec2 u_screen;
uniform vec2 u_scale_xy;
uniform vec2 u_vertex_offset;
uniform vec2 u_sample_offset;
uniform float u_gradient_scale;
uniform float u_sharpness;
uniform float u_weight_normal;
uniform float u_weight_bold;
uniform float u_face_dilate;
uniform float u_scale_ratio_a;
uniform float u_outline_width;
uniform float u_outline_softness;

in vec2 a_position;
in vec2 a_uv;
in float a_scale;
in vec4 a_color;

out vec4 v_color;
out vec2 v_atlas;
out vec4 v_param;

void main() {
    float bold = step(a_scale, 0.0);

    vec4 vert = vec4(a_position + u_vertex_offset, 0.0, 1.0);
    vec4 position = u_projection * vert;

    vec2 pixel_size = vec2(position.w);
    pixel_size /= u_scale_xy * abs(mat2(u_projection) * u_screen);
    float scale = inversesqrt(dot(pixel_size, pixel_size));
    scale *= abs(a_scale) * u_gradient_scale * (u_sharpness + 1.0);

    float weight = mix(u_weight_normal, u_weight_bold, bold) / 4.0;
    weight = (weight + u_face_dilate) * u_scale_ratio_a * 0.5;

    float bias = (0.5 - weight) + (0.5 / scale);

    float alpha_clip = (1.0 - u_outline_width * u_scale_ratio_a - u_outline_softness * u_scale_ratio_a);
    alpha_clip = alpha_clip / 2.0 - (0.5 / scale) - weight;

    gl_Position = position;
    gl_Position.xy -= u_sample_offset * 2.0 / u_screen * position.w;
    v_color = a_color;
    v_atlas = a_uv;
    v_param = vec4(alpha_clip, scale, bias, weight);
}
