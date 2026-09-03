#version 450

// Uses toon-mesh.vert so it receives the ordinary static-mesh UV and per-instance
// color streams, but deliberately performs no lighting or emissive work.
layout(location = 2) in vec2 v_uv;
layout(location = 3) in vec4 v_color;

layout(location = 0) out vec4 f_color;

// Keep the standard material-set layout: all mesh materials bind this UBO and
// the texture together. Unlit's UBO color is neutral white, so this does not
// introduce a second authored color source.
layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    float quant_steps;
    uint emissive;
    uint _pad0;
    uint _pad1;
} mat;

layout(set = 1, binding = 1) uniform sampler2D base_tex;

void main() {
    vec4 base_rgba = texture(base_tex, v_uv) * v_color * mat.base_color;
    if (base_rgba.a <= 0.001) {
        discard;
    }
    f_color = base_rgba;
}
