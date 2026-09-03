#version 450

layout(location = 3) in vec4 v_color;
// Roughness selects the frosted filter footprint for this screen-aligned path.
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
// Full-viewport filtered copies of scene_color at 1/2 through 1/32 scale.
// They are renderer-built rather than authored texture mipmaps.
layout(set = 0, binding = 4) uniform sampler2D scene_color_half;
layout(set = 0, binding = 5) uniform sampler2D scene_color_quarter;
layout(set = 0, binding = 6) uniform sampler2D scene_color_eighth;
layout(set = 0, binding = 7) uniform sampler2D scene_color_sixteenth;
layout(set = 0, binding = 8) uniform sampler2D scene_color_thirtysecond;

void main() {
    vec2 viewport = max(ubo.viewport, vec2(1.0));
    vec2 base_uv = gl_FragCoord.xy / viewport;

    float roughness = clamp(v_transmission_roughness, 0.0, 1.0);
    // Frosted transmission is a screen-space diffuse filter, not a second
    // refractive material: preserve alignment with the scene behind the
    // surface as the viewer changes yaw or pitch.
    vec3 sharp = texture(scene_color, base_uv).rgb;
    vec3 captured = sharp;
    if (roughness > 0.0) {
        // The filtered levels are deliberately bounded, but do not yet carry a
        // matching depth-aware footprint; foreground-edge-safe rough filtering
        // is tracked as the next correctness slice.
        vec3 half_color = texture(scene_color_half, base_uv).rgb;
        vec3 quarter_color = texture(scene_color_quarter, base_uv).rgb;
        vec3 eighth_color = texture(scene_color_eighth, base_uv).rgb;
        vec3 sixteenth_color = texture(scene_color_sixteenth, base_uv).rgb;
        vec3 thirtysecond_color = texture(scene_color_thirtysecond, base_uv).rgb;

        // Blend between the fixed, discrete pyramid levels so roughness stays a
        // continuous authored f32 while the renderer's work remains bounded.
        float level = roughness * 5.0;
        captured = level < 1.0
            ? mix(sharp, half_color, level)
            : (level < 2.0
                ? mix(half_color, quarter_color, level - 1.0)
                : (level < 3.0
                    ? mix(quarter_color, eighth_color, level - 2.0)
                    : (level < 4.0
                        ? mix(eighth_color, sixteenth_color, level - 3.0)
                        : mix(sixteenth_color, thirtysecond_color, level - 4.0))));
    }
    vec3 tint = mix(vec3(1.0), clamp(v_color.rgb, 0.0, 1.0), 0.25);
    f_color = vec4(captured * tint, clamp(v_color.a, 0.0, 1.0));
}
