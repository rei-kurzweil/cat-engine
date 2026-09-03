#version 450

layout(location = 0) in vec3 v_world_pos;
layout(location = 1) in vec3 v_normal;
layout(location = 3) in vec4 v_color;
// x = IOR, y = effective thickness, z = strength, w = viewport-edge fade.
layout(location = 5) flat in vec4 v_transmission;
// Carried per instance for the next filtering slice. At roughness zero this
// shader deliberately converges with sharp refraction; the blur pyramid is not
// part of this routing/parity slice.
layout(location = 6) flat in float v_transmission_roughness;

layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform CameraUBO {
    mat4 view;
    mat4 proj;
    mat3 camera2d;
    vec2 viewport;
    vec2 _pad0;
    vec3 ambient_light;
    uint renderer_flags;
} ubo;

layout(set = 0, binding = 2) uniform sampler2D scene_color;
layout(set = 0, binding = 3) uniform sampler2D scene_depth;

const uint RENDERER_TRANSMISSION_DEPTH_COMPARE = 1u << 0;

void main() {
    vec2 viewport = max(ubo.viewport, vec2(1.0));
    vec2 base_uv = gl_FragCoord.xy / viewport;

    vec3 position_vs = (ubo.view * vec4(v_world_pos, 1.0)).xyz;
    vec3 incident_vs = normalize(position_vs);
    vec3 normal_vs = normalize(mat3(ubo.view) * v_normal);
    if (!gl_FrontFacing) {
        normal_vs = -normal_vs;
    }

    float ior = max(v_transmission.x, 1.0);
    float thickness = max(v_transmission.y, 0.0);
    float strength = max(v_transmission.z, 0.0);
    vec3 refracted_vs = refract(incident_vs, normal_vs, 1.0 / ior);

    vec2 incident_slope = incident_vs.xy / max(abs(incident_vs.z), 0.15);
    vec2 refracted_slope = refracted_vs.xy / max(abs(refracted_vs.z), 0.15);
    vec2 offset = (refracted_slope - incident_slope) * thickness * strength;

    float edge = min(min(base_uv.x, 1.0 - base_uv.x), min(base_uv.y, 1.0 - base_uv.y));
    float edge_fade = max(v_transmission.w, 1e-5);
    offset *= smoothstep(0.0, edge_fade, edge);
    vec2 refracted_uv = clamp(base_uv + offset, vec2(0.0), vec2(1.0));

    if ((ubo.renderer_flags & RENDERER_TRANSMISSION_DEPTH_COMPARE) != 0u) {
        float candidate_depth = texture(scene_depth, refracted_uv).r;
        const float foreground_bias = 1e-4;
        if (candidate_depth + foreground_bias < gl_FragCoord.z) {
            refracted_uv = base_uv;
        }
    }

    // Roughness is intentionally carried but not consumed until the renderer
    // supplies a bounded, depth-safe scene-color filtering pyramid.
    float roughness = clamp(v_transmission_roughness, 0.0, 1.0);
    vec3 captured = texture(scene_color, refracted_uv).rgb;
    vec3 tint = mix(vec3(1.0), clamp(v_color.rgb, 0.0, 1.0), 0.25);
    f_color = vec4(captured * tint, clamp(v_color.a, 0.0, 1.0));
}
