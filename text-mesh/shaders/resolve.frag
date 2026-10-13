#version 300 es
// The multisample resolve: the average of every sample's frame (one layer
// each).
precision highp float;
precision highp sampler2DArray;

uniform sampler2DArray u_samples;
uniform int u_count;

out vec4 fragment;

void main() {
    ivec2 pixel = ivec2(gl_FragCoord.xy);
    vec4 sum = vec4(0.0);
    for (int sample_index = 0; sample_index < 16; sample_index++) {
        if (sample_index >= u_count) {
            break;
        }
        sum += texelFetch(u_samples, ivec3(pixel, sample_index), 0);
    }
    fragment = sum / float(u_count);
}
