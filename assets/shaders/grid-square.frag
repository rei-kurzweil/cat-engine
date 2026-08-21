#version 450

layout(location = 0) in vec3 v_world_pos;
layout(location = 1) in vec2 v_uv;
layout(location = 2) in vec4 v_color;
layout(location = 3) in vec2 v_grid_local_pos;
layout(location = 4) in vec3 v_grid_normal;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
    mat3 camera2d;
    vec2 viewport;
    vec2 _pad0;
    vec3 ambient_light;
    float _pad1;
} ubo;

layout(set = 1, binding = 0) uniform MaterialUBO {
    vec4 base_color;
    float quant_steps;
    uint emissive;
    uvec2 _pad0;
} mat;

layout(set = 1, binding = 1) uniform sampler2D base_tex;

float grid_axis_alpha(float v, float pixels) {
    float fw = max(fwidth(v), 1e-6);
    return 1.0 - smoothstep(0.0, fw * pixels, abs(v));
}

float grid_square_alpha(vec2 world_xz, float spacing, float thickness) {
    vec2 uv = world_xz / max(spacing, 1e-5);
    vec2 deriv = max(fwidth(uv), vec2(1e-6));
    vec2 dist = abs(fract(uv - 0.5) - 0.5) / (deriv * thickness);
    return 1.0 - min(min(dist.x, dist.y), 1.0);
}

void main() {
    float encoded_spacing = mat.quant_steps;
    float spacing = max(abs(encoded_spacing), 1e-5);
    bool world_space = encoded_spacing < 0.0;

    vec2 grid_pos = v_grid_local_pos;
    if (world_space) {
        // Choose the two world axes most parallel to this plane. Ties are
        // resolved X, then Y, then Z by the strict comparisons below.
        vec3 alignment = abs(v_grid_normal);
        int first = alignment.x <= alignment.y && alignment.x <= alignment.z ? 0
            : (alignment.y <= alignment.z ? 1 : 2);
        int second;
        if (first == 0) second = alignment.y <= alignment.z ? 1 : 2;
        else if (first == 1) second = alignment.x <= alignment.z ? 0 : 2;
        else second = alignment.x <= alignment.y ? 0 : 1;
        grid_pos = vec2(v_world_pos[first], v_world_pos[second]);
    }

    float minor = grid_square_alpha(grid_pos, spacing, 1.0);
    float major = grid_square_alpha(grid_pos, spacing * 8.0, 1.8);
    float axis = max(grid_axis_alpha(grid_pos.x, 2.0), grid_axis_alpha(grid_pos.y, 3.0));

    vec3 line_rgb = v_color.rgb;
    //vec3 major_rgb = mix(line_rgb, vec3(1.0,1.0,1.0), 0.18);
    //vec3 axis_rgb = vec3(1.0, 0.32, 0.32);

    float cam_dist = length(grid_pos);
    float fade = 1.0 - smoothstep(32.0, 96.0, cam_dist);

    vec3 rgb = vec3(0.0);
    rgb = mix(line_rgb, vec3(minor * 0.45), 0.5);
    //rgb = mix(rgb, major_rgb, major * 0.75);
    //rgb = mix(rgb, axis_rgb, axis);

    float alpha = max(max(minor * 0.45, major * 0.75), axis) * v_color.a * fade;
    if (alpha <= 0.001) {
        discard;
    }

    f_color = vec4(rgb, alpha);
}
