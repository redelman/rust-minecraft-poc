#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// Vertex input from mesh
// Bevy's standard attribute locations:
// - ATTRIBUTE_POSITION = 0
// - ATTRIBUTE_NORMAL = 1
// - ATTRIBUTE_UV_0 = 2
// - ATTRIBUTE_UV_1 = 3
// - ATTRIBUTE_TANGENT = 4
// - ATTRIBUTE_COLOR = 5
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    // UV2 for overlay texture (0,0 means no overlay)
    @location(3) uv2: vec2<f32>,
    // Color: RGB = tint color, A = pre-calculated brightness (0.1-1.0)
    @location(5) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) uv2: vec2<f32>,
};

struct TerrainMaterialUniform {
    // x: min light level (ambient), y: unused, z: unused, w: unused
    settings: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> material: TerrainMaterialUniform;

@group(2) @binding(1)
var base_texture: texture_2d<f32>;

@group(2) @binding(2)
var base_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // Transform position to world space
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.clip_position = position_world_to_clip(out.world_position.xyz);

    // Transform normal to world space
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

    // Pass through other attributes
    out.uv = vertex.uv;
    out.color = vertex.color;
    out.uv2 = vertex.uv2;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample base texture
    var base_color = textureSample(base_texture, base_sampler, in.uv);

    // Check if we have an overlay (uv2 != 0)
    let has_overlay = in.uv2.x > 0.001 || in.uv2.y > 0.001;

    if (has_overlay) {
        // For faces with overlay (grass sides):
        // - Base texture (dirt) should NOT be tinted with grass color
        // - Overlay texture (grass) SHOULD be tinted with grass color (vertex RGB)
        // - Both should have the same brightness/lighting applied

        // Sample overlay texture
        let overlay = textureSample(base_texture, base_sampler, in.uv2);

        // Tint only the overlay with grass color (vertex RGB holds the grass tint)
        let overlay_tinted = vec4<f32>(overlay.rgb * in.color.rgb, overlay.a);

        // Alpha blend: base (untinted) with overlay (tinted)
        base_color = vec4<f32>(
            mix(base_color.rgb, overlay_tinted.rgb, overlay.a),
            base_color.a
        );
    } else {
        // No overlay - apply vertex color tint normally (for grass tops, colored blocks, etc.)
        base_color = vec4<f32>(base_color.rgb * in.color.rgb, base_color.a);
    }

    // Apply brightness from vertex color alpha
    // The brightness value is pre-calculated by mesh generation (already includes
    // face shading, light propagation, and minimum ambient), so apply directly
    let brightness = in.color.a;
    base_color = vec4<f32>(base_color.rgb * brightness, base_color.a);

    // Discard fully transparent pixels
    if (base_color.a < 0.01) {
        discard;
    }

    return base_color;
}
