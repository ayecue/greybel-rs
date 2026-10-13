#version 300 es
// TMP_Sprite.shader's vert.
precision highp float;

uniform mat4 u_projection;
uniform vec4 u_color;
uniform vec2 u_screen;
uniform vec2 u_sample_offset;

in vec2 a_position;
in vec2 a_uv;
in vec4 a_color;

out vec4 v_color;
out vec2 v_texcoord;

void main() {
    gl_Position = u_projection * vec4(a_position, 0.0, 1.0);
    gl_Position.xy -= u_sample_offset * 2.0 / u_screen * gl_Position.w;
    v_texcoord = a_uv;
    v_color = a_color * u_color;
}
