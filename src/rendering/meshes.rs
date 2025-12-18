use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

pub fn create_voxel_mesh() -> Mesh {
    let vertices = vec![
        // Front face
        ([-0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ([0.5, -0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 0.0]),
        ([0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [1.0, 1.0]),
        ([-0.5, 0.5, 0.5], [0.0, 0.0, 1.0], [0.0, 1.0]),
        // Back face
        ([0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 0.0]),
        ([-0.5, -0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 0.0]),
        ([-0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [1.0, 1.0]),
        ([0.5, 0.5, -0.5], [0.0, 0.0, -1.0], [0.0, 1.0]),
        // Right face
        ([0.5, -0.5, 0.5], [1.0, 0.0, 0.0], [0.0, 0.0]),
        ([0.5, -0.5, -0.5], [1.0, 0.0, 0.0], [1.0, 0.0]),
        ([0.5, 0.5, -0.5], [1.0, 0.0, 0.0], [1.0, 1.0]),
        ([0.5, 0.5, 0.5], [1.0, 0.0, 0.0], [0.0, 1.0]),
        // Left face
        ([-0.5, -0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 0.0]),
        ([-0.5, -0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 0.0]),
        ([-0.5, 0.5, 0.5], [-1.0, 0.0, 0.0], [1.0, 1.0]),
        ([-0.5, 0.5, -0.5], [-1.0, 0.0, 0.0], [0.0, 1.0]),
        // Top face
        ([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [1.0, 0.0]),
        ([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [1.0, 1.0]),
        ([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 1.0]),
        // Bottom face
        ([-0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [0.0, 0.0]),
        ([0.5, -0.5, -0.5], [0.0, -1.0, 0.0], [1.0, 0.0]),
        ([0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [1.0, 1.0]),
        ([-0.5, -0.5, 0.5], [0.0, -1.0, 0.0], [0.0, 1.0]),
    ];

    let indices: Vec<u32> = vec![
        0, 1, 2, 2, 3, 0, // front
        4, 5, 6, 6, 7, 4, // back
        8, 9, 10, 10, 11, 8, // right
        12, 13, 14, 14, 15, 12, // left
        16, 17, 18, 18, 19, 16, // top
        20, 21, 22, 22, 23, 20, // bottom
    ];

    let positions: Vec<[f32; 3]> = vertices.iter().map(|(p, _, _)| *p).collect();
    let normals: Vec<[f32; 3]> = vertices.iter().map(|(_, n, _)| *n).collect();
    let uvs: Vec<[f32; 2]> = vertices.iter().map(|(_, _, uv)| *uv).collect();

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
