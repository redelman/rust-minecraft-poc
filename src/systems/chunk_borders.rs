use bevy::prelude::*;
use crate::components::{CameraController, ChunkBorder};
use crate::resources::{ChunkBorderState, ChunkBorderMode};
use crate::world::{ChunkCoord, CHUNK_SIZE};

const CORNER_RENDER_DISTANCE: i32 = 2;  // Show red corners within 2 chunks of player

pub fn update_chunk_borders(
    mut commands: Commands,
    border_state: Res<ChunkBorderState>,
    camera_query: Query<&Transform, With<CameraController>>,
    existing_borders: Query<Entity, With<ChunkBorder>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Despawn existing borders if they exist
    for entity in existing_borders.iter() {
        commands.entity(entity).despawn();
    }

    // Only create new borders if not Off
    if border_state.mode == ChunkBorderMode::Off {
        return;
    }

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let player_chunk = ChunkCoord::from_world_pos(camera_transform.translation);
    let player_y = camera_transform.translation.y;

    // Create materials once
    let red_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.0, 0.0, 0.8),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let corner_mesh = create_corner_mesh(player_y);
    let corner_mesh_handle = meshes.add(corner_mesh);

    match border_state.mode {
        ChunkBorderMode::Off => {}
        ChunkBorderMode::Mode1 => {
            // Red corner lines within 2 chunks of player
            for dx in -CORNER_RENDER_DISTANCE..=CORNER_RENDER_DISTANCE {
                for dz in -CORNER_RENDER_DISTANCE..=CORNER_RENDER_DISTANCE {
                    let chunk_coord = ChunkCoord {
                        x: player_chunk.x + dx,
                        y: player_chunk.y,  // Use player's Y chunk
                        z: player_chunk.z + dz,
                    };

                    // Skip chunks outside circular render distance
                    if chunk_coord.distance_squared(&player_chunk) > CORNER_RENDER_DISTANCE * CORNER_RENDER_DISTANCE {
                        continue;
                    }

                    let chunk_world_pos = chunk_coord.to_world_pos();

                    commands.spawn((
                        Mesh3d(corner_mesh_handle.clone()),
                        MeshMaterial3d(red_material.clone()),
                        Transform::from_translation(chunk_world_pos),
                        ChunkBorder,
                    ));
                }
            }
        }
        ChunkBorderMode::Mode2 => {
            let green_material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.0, 1.0, 0.0, 0.8),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            // Green grid at current chunk edges only
            let grid_mesh = create_grid_mesh(player_y);
            let grid_mesh_handle = meshes.add(grid_mesh);

            let player_chunk_world_pos = player_chunk.to_world_pos();

            commands.spawn((
                Mesh3d(grid_mesh_handle),
                MeshMaterial3d(green_material),
                Transform::from_translation(player_chunk_world_pos),
                ChunkBorder,
            ));

            // Red corner lines within 2 chunks of player
            for dx in -CORNER_RENDER_DISTANCE..=CORNER_RENDER_DISTANCE {
                for dz in -CORNER_RENDER_DISTANCE..=CORNER_RENDER_DISTANCE {
                    let chunk_coord = ChunkCoord {
                        x: player_chunk.x + dx,
                        y: player_chunk.y,  // Use player's Y chunk
                        z: player_chunk.z + dz,
                    };

                    // Skip chunks outside circular render distance
                    if chunk_coord.distance_squared(&player_chunk) > CORNER_RENDER_DISTANCE * CORNER_RENDER_DISTANCE {
                        continue;
                    }

                    let chunk_world_pos = chunk_coord.to_world_pos();

                    commands.spawn((
                        Mesh3d(corner_mesh_handle.clone()),
                        MeshMaterial3d(red_material.clone()),
                        Transform::from_translation(chunk_world_pos),
                        ChunkBorder,
                    ));
                }
            }
        }
    }
}

fn create_corner_mesh(player_y: f32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    let size = CHUNK_SIZE as f32;
    // Extend 256 blocks above and below the player's current position
    let bottom = player_y - 256.0;
    let top = player_y + 256.0;

    // 4 red vertical corner lines at the chunk corners
    let positions = vec![
        // Corner (0, 0)
        [0.0, bottom, 0.0],
        [0.0, top, 0.0],
        // Corner (size, 0)
        [size, bottom, 0.0],
        [size, top, 0.0],
        // Corner (size, size)
        [size, bottom, size],
        [size, top, size],
        // Corner (0, size)
        [0.0, bottom, size],
        [0.0, top, size],
    ];

    let indices: Vec<u32> = vec![
        0, 1,  // Corner (0, 0)
        2, 3,  // Corner (size, 0)
        4, 5,  // Corner (size, size)
        6, 7,  // Corner (0, size)
    ];

    let normals = vec![[0.0, 1.0, 0.0]; 8];

    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

fn create_grid_mesh(player_y: f32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    let size = CHUNK_SIZE as f32;
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    // Extend 128 blocks above and below the player for grid visibility
    // (Less than corners to reduce mesh complexity)
    let bottom = player_y - 128.0;
    let top = player_y + 128.0;

    // Create green grid lines at the 4 edges of the chunk
    // Skip corners to avoid overlap with red corner lines

    // X=0 edge (west side) - vertical lines every block, excluding corners
    for z in 1..CHUNK_SIZE {
        let base_idx = positions.len() as u32;
        positions.push([0.0, bottom, z as f32]);
        positions.push([0.0, top, z as f32]);
        indices.extend_from_slice(&[base_idx, base_idx + 1]);
    }

    // X=size edge (east side) - vertical lines every block, excluding corners
    for z in 1..CHUNK_SIZE {
        let base_idx = positions.len() as u32;
        positions.push([size, bottom, z as f32]);
        positions.push([size, top, z as f32]);
        indices.extend_from_slice(&[base_idx, base_idx + 1]);
    }

    // Z=0 edge (north side) - vertical lines every block, excluding corners
    for x in 1..CHUNK_SIZE {
        let base_idx = positions.len() as u32;
        positions.push([x as f32, bottom, 0.0]);
        positions.push([x as f32, top, 0.0]);
        indices.extend_from_slice(&[base_idx, base_idx + 1]);
    }

    // Z=size edge (south side) - vertical lines every block, excluding corners
    for x in 1..CHUNK_SIZE {
        let base_idx = positions.len() as u32;
        positions.push([x as f32, bottom, size]);
        positions.push([x as f32, top, size]);
        indices.extend_from_slice(&[base_idx, base_idx + 1]);
    }

    // Add horizontal lines at each block height (every 1 unit in Y)
    // This creates a full 3D grid on the chunk edges
    for y in (bottom as i32)..=(top as i32) {
        let fy = y as f32;

        // Horizontal lines on X=0 edge (west)
        for z in 0..CHUNK_SIZE {
            let base_idx = positions.len() as u32;
            positions.push([0.0, fy, z as f32]);
            positions.push([0.0, fy, (z + 1) as f32]);
            indices.extend_from_slice(&[base_idx, base_idx + 1]);
        }

        // Horizontal lines on X=size edge (east)
        for z in 0..CHUNK_SIZE {
            let base_idx = positions.len() as u32;
            positions.push([size, fy, z as f32]);
            positions.push([size, fy, (z + 1) as f32]);
            indices.extend_from_slice(&[base_idx, base_idx + 1]);
        }

        // Horizontal lines on Z=0 edge (north)
        for x in 0..CHUNK_SIZE {
            let base_idx = positions.len() as u32;
            positions.push([x as f32, fy, 0.0]);
            positions.push([(x + 1) as f32, fy, 0.0]);
            indices.extend_from_slice(&[base_idx, base_idx + 1]);
        }

        // Horizontal lines on Z=size edge (south)
        for x in 0..CHUNK_SIZE {
            let base_idx = positions.len() as u32;
            positions.push([x as f32, fy, size]);
            positions.push([(x + 1) as f32, fy, size]);
            indices.extend_from_slice(&[base_idx, base_idx + 1]);
        }
    }

    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];

    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}

