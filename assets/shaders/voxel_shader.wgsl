#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct VoxelMaterial {
    data: vec4<f32>,  // x: time, y: brightness, z: unused, w: unused
}

@group(2) @binding(100)
var<uniform> voxel_material: VoxelMaterial;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Get the standard PBR input
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // Apply brightness multiplier to the base color
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * voxel_material.data.y,
        pbr_input.material.base_color.a
    );

    // Add subtle pulsing effect based on time and position
    let pulse = sin(voxel_material.data.x * 2.0 + in.world_position.y * 3.0) * 0.05 + 1.0;
    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb * pulse,
        pbr_input.material.base_color.a
    );

    // Add edge highlighting based on normal
    let view_dir = normalize(in.world_position.xyz - in.world_position.xyz);
    let edge_factor = pow(1.0 - abs(dot(in.world_normal, view_dir)), 2.0);
    let edge_color = vec3<f32>(0.2, 0.3, 0.4) * edge_factor * 0.3;

    pbr_input.material.base_color = vec4<f32>(
        pbr_input.material.base_color.rgb + edge_color,
        pbr_input.material.base_color.a
    );

    // Alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // Deferred rendering
    let out = deferred_output(in, pbr_input);
#else
    // Forward rendering
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
