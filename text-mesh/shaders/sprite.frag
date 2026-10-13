#version 300 es
// TMP_Sprite.shader's frag.
precision highp float;

uniform sampler2D u_sprites;
uniform vec4 u_texture_sample_add;
uniform vec2 u_sample_offset;

in vec4 v_color;
in vec2 v_texcoord;

out vec4 fragment;

void main() {
    vec2 texcoord = v_texcoord - dFdx(v_texcoord) * u_sample_offset.x - dFdy(v_texcoord) * u_sample_offset.y;
    fragment = (textureLod(u_sprites, texcoord, 0.0) + u_texture_sample_add) * v_color;
}
