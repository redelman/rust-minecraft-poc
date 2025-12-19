use bevy::prelude::*;
use crate::components::{CameraController, BlockHighlight};
use crate::resources::ChunkManager;
use crate::world::{ChunkCoord, Chunk, CHUNK_SIZE};

pub fn update_block_highlight(
    mut commands: Commands,
    camera_query: Query<(&Transform, &CameraController), With<CameraController>>,
    existing_highlights: Query<Entity, With<BlockHighlight>>,
    chunk_manager: Res<ChunkManager>,
    chunks: Query<&Chunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn existing highlight
    for entity in existing_highlights.iter() {
        commands.entity(entity).despawn();
    }

    let Ok((camera_transform, _controller)) = camera_query.get_single() else {
        return;
    };

    // Raycast from camera to find which block we're looking at
    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward();
    let max_distance = 10.0;

    if let Some(hit_pos) = raycast_terrain(ray_origin, *ray_direction, max_distance, &chunk_manager, &chunks) {
        // Create wireframe box around the block
        let mesh = create_block_highlight_mesh();
        let mesh_handle = meshes.add(mesh);

        let material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.4), // Black with transparency
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });

        // Blocks use 0-to-1 coordinates, highlight mesh matches this
        let highlight_pos = hit_pos;

        commands.spawn((
            Mesh3d(mesh_handle),
            MeshMaterial3d(material),
            Transform::from_translation(highlight_pos),
            BlockHighlight,
        ));
    }
}

fn raycast_terrain(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_manager: &ChunkManager,
    chunks: &Query<&Chunk>,
) -> Option<Vec3> {
    let step = 0.1;
    let mut current = origin;
    let direction = direction.normalize();

    for _ in 0..((max_distance / step) as i32) {
        current += direction * step;

        // Check if this position hits a block
        let block_x = current.x.floor() as i32;
        let block_y = current.y.floor() as i32;
        let block_z = current.z.floor() as i32;

        let chunk_coord = ChunkCoord::from_world_pos(current);

        if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&chunk_coord) {
            if let Ok(chunk) = chunks.get(chunk_entity) {
                // Convert world position to local chunk position
                let local_x = (block_x - chunk_coord.x * CHUNK_SIZE as i32) as usize;
                let local_y = (block_y - chunk_coord.y * CHUNK_SIZE as i32) as usize;
                let local_z = (block_z - chunk_coord.z * CHUNK_SIZE as i32) as usize;

                if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE && local_z < CHUNK_SIZE {
                    if !chunk.get_block(local_x, local_y, local_z).is_air() {
                        // Hit a block! Return its position
                        return Some(Vec3::new(block_x as f32, block_y as f32, block_z as f32));
                    }
                }
            }
        }
    }

    None
}

fn create_block_highlight_mesh() -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    // Create a wireframe box using 0-to-1 coordinates (matching block convention)
    // Slightly larger than a block to avoid z-fighting
    let min = -0.005;
    let max = 1.005;
    let positions = vec![
        // Bottom corners
        [min, min, min],
        [max, min, min],
        [max, min, max],
        [min, min, max],
        // Top corners
        [min, max, min],
        [max, max, min],
        [max, max, max],
        [min, max, max],
    ];

    let indices: Vec<u32> = vec![
        // Bottom square
        0, 1,
        1, 2,
        2, 3,
        3, 0,
        // Top square
        4, 5,
        5, 6,
        6, 7,
        7, 4,
        // Vertical edges
        0, 4,
        1, 5,
        2, 6,
        3, 7,
    ];

    let normals = vec![[0.0, 1.0, 0.0]; 8];

    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}
