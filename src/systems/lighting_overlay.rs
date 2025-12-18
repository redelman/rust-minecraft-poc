use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::components::CameraController;
use crate::world::{Chunk, ChunkCoord, CHUNK_SIZE};
use crate::systems::NeedsRemesh;

/// Resource to track lighting overlay state
#[derive(Resource)]
pub struct LightingOverlayState {
    pub enabled: bool,
    /// Force update on next frame (e.g., after chunk modification)
    pub needs_update: bool,
    /// Delay counter - wait this many frames before rebuilding
    /// This ensures despawn commands are processed before new spawns
    rebuild_delay: u8,
}

impl Default for LightingOverlayState {
    fn default() -> Self {
        Self {
            enabled: false,
            needs_update: false,
            rebuild_delay: 0,
        }
    }
}

/// Component to mark lighting overlay markers for cleanup
#[derive(Component)]
pub struct LightingOverlayMarker;

/// Light level threshold - blocks with light < 7 can spawn hostile mobs in Minecraft
const MOB_SPAWN_THRESHOLD: u8 = 7;

/// How far (in chunks) to render the overlay
const OVERLAY_RENDER_DISTANCE: i32 = 2;

/// Toggle lighting overlay with F7
pub fn toggle_lighting_overlay(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut overlay_state: ResMut<LightingOverlayState>,
) {
    if keyboard_input.just_pressed(KeyCode::F7) {
        overlay_state.enabled = !overlay_state.enabled;
        overlay_state.needs_update = true; // Force update when toggled
        let state = if overlay_state.enabled { "ON" } else { "OFF" };
        info!("Lighting overlay: {}", state);
    }
}

/// System to detect chunk modifications and trigger overlay update
pub fn detect_chunk_changes(
    mut overlay_state: ResMut<LightingOverlayState>,
    remesh_query: Query<Entity, With<NeedsRemesh>>,
) {
    // If any chunks need remeshing, we should update the overlay
    if !remesh_query.is_empty() && overlay_state.enabled {
        overlay_state.needs_update = true;
    }
}

/// System to update lighting overlay markers
pub fn update_lighting_overlay(
    mut commands: Commands,
    mut overlay_state: ResMut<LightingOverlayState>,
    camera_query: Query<&Transform, With<CameraController>>,
    chunk_query: Query<(&Chunk, &Transform)>,
    existing_markers: Query<Entity, With<LightingOverlayMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    remesh_query: Query<Entity, With<NeedsRemesh>>,
) {
    // Don't render if disabled
    if !overlay_state.enabled {
        // Despawn any existing markers when disabled
        for entity in existing_markers.iter() {
            commands.entity(entity).despawn();
        }
        overlay_state.needs_update = false;
        overlay_state.rebuild_delay = 0;
        return;
    }

    // Wait for remeshing to complete before updating overlay
    if !remesh_query.is_empty() {
        // Mark that we need an update once remeshing is done
        overlay_state.needs_update = true;
        return;
    }

    // Check if we need to rebuild the overlay
    let markers_exist = !existing_markers.is_empty();
    let should_rebuild = overlay_state.needs_update || !markers_exist;

    if !should_rebuild {
        return;
    }

    // Use a 2-frame delay to ensure despawn commands are processed
    // Frame 1: Despawn old markers, start delay
    // Frame 2: Wait for despawns to apply
    // Frame 3: Spawn new markers
    if overlay_state.rebuild_delay == 0 {
        // First frame: despawn old markers and start delay
        for entity in existing_markers.iter() {
            commands.entity(entity).despawn();
        }
        overlay_state.rebuild_delay = 2;
        return;
    } else if overlay_state.rebuild_delay > 1 {
        // Waiting for despawns to be applied
        overlay_state.rebuild_delay -= 1;
        return;
    }

    // Delay complete, now rebuild
    overlay_state.rebuild_delay = 0;
    overlay_state.needs_update = false;

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let player_chunk = ChunkCoord::from_world_pos(camera_transform.translation);

    // Create materials for different light levels
    // Red: light level < 7 (mobs can spawn now)
    let red_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.0, 0.0, 0.9),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    // Yellow: light level >= 7 but only from sky light (could become dangerous at night)
    let yellow_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 0.0, 0.9),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    });

    // Collect all positions that need markers
    let mut red_positions: Vec<Vec3> = Vec::new();
    let mut yellow_positions: Vec<Vec3> = Vec::new();

    // Iterate through nearby chunks
    for dx in -OVERLAY_RENDER_DISTANCE..=OVERLAY_RENDER_DISTANCE {
        for dz in -OVERLAY_RENDER_DISTANCE..=OVERLAY_RENDER_DISTANCE {
            // Check all Y levels that might be loaded
            for dy in -2..=2 {
                let chunk_coord = ChunkCoord {
                    x: player_chunk.x + dx,
                    y: player_chunk.y + dy,
                    z: player_chunk.z + dz,
                };

                // Find the chunk with this coordinate
                for (chunk, chunk_transform) in chunk_query.iter() {
                    if chunk.coord == chunk_coord {
                        // Scan the top surface of blocks in this chunk
                        collect_low_light_positions(
                            chunk,
                            chunk_transform.translation,
                            &mut red_positions,
                            &mut yellow_positions,
                        );
                        break;
                    }
                }
            }
        }
    }

    // Create meshes for all markers
    if !red_positions.is_empty() {
        let red_mesh = create_x_markers_mesh(&red_positions);
        commands.spawn((
            Mesh3d(meshes.add(red_mesh)),
            MeshMaterial3d(red_material),
            Transform::IDENTITY,
            LightingOverlayMarker,
        ));
    }

    if !yellow_positions.is_empty() {
        let yellow_mesh = create_x_markers_mesh(&yellow_positions);
        commands.spawn((
            Mesh3d(meshes.add(yellow_mesh)),
            MeshMaterial3d(yellow_material),
            Transform::IDENTITY,
            LightingOverlayMarker,
        ));
    }
}

/// Collect positions where light level is low enough for mob spawning
fn collect_low_light_positions(
    chunk: &Chunk,
    chunk_world_pos: Vec3,
    red_positions: &mut Vec<Vec3>,
    yellow_positions: &mut Vec<Vec3>,
) {
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block = chunk.get_block(x, y, z);

                // Skip non-solid blocks (we want to mark on top of solid blocks)
                if block.is_air() {
                    continue;
                }

                // Check if there's air above this block (spawnable surface)
                let above_y = y + 1;
                if above_y >= CHUNK_SIZE {
                    // Would need to check neighbor chunk - skip for now
                    continue;
                }

                let block_above = chunk.get_block(x, above_y, z);
                if !block_above.is_air() {
                    continue;
                }

                // Get the light level of the air block above
                let light = chunk.get_light(x, above_y, z);

                // World position of the marker (on top of the block)
                let world_pos = chunk_world_pos + Vec3::new(
                    x as f32 + 0.5,
                    (y + 1) as f32 + 0.01, // Slightly above the block surface
                    z as f32 + 0.5,
                );

                if light < MOB_SPAWN_THRESHOLD {
                    // Red: definitely dangerous (light < 7)
                    red_positions.push(world_pos);
                } else if light == 15 {
                    // Light 15 with direct sky access = safe day AND night
                    // Light 15 without direct sky access = yellow (could be from propagation)
                    // Check if there's direct sky access by looking up in the same chunk
                    let has_sky_access = has_direct_sky_access_in_chunk(chunk, x, above_y, z);
                    if !has_sky_access {
                        // No direct sky access but light is 15 - likely propagated from entrance
                        // This could still be dangerous at night since the propagated light
                        // from the entrance will drop
                        yellow_positions.push(world_pos);
                    }
                    // If has_sky_access, it's truly safe - skip
                } else {
                    // Light 7-14: potentially dangerous at night
                    // Yellow marker for all of these since they could drop below 7
                    yellow_positions.push(world_pos);
                }
            }
        }
    }
}

/// Check if a position has direct vertical sky access within the same chunk
/// This is a simplified check - it only looks up within this chunk
fn has_direct_sky_access_in_chunk(chunk: &Chunk, x: usize, y: usize, z: usize) -> bool {
    // Check all blocks above this position within the chunk
    for check_y in (y + 1)..CHUNK_SIZE {
        if !chunk.get_block(x, check_y, z).is_air() {
            return false; // Found a solid block above
        }
    }
    // All blocks above are air within this chunk
    // This doesn't guarantee sky access (chunks above might have blocks)
    // but it's a good approximation for outdoor surface blocks
    true
}

/// Create a mesh containing X markers at all given positions
fn create_x_markers_mesh(positions: &[Vec3]) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // X marker size (half-width)
    let size = 0.35;

    for pos in positions {
        let base_idx = vertices.len() as u32;

        // Create an X shape with two crossing lines
        // Line 1: from (-size, 0, -size) to (+size, 0, +size)
        vertices.push([pos.x - size, pos.y, pos.z - size]);
        vertices.push([pos.x + size, pos.y, pos.z + size]);

        // Line 2: from (-size, 0, +size) to (+size, 0, -size)
        vertices.push([pos.x - size, pos.y, pos.z + size]);
        vertices.push([pos.x + size, pos.y, pos.z - size]);

        indices.extend_from_slice(&[
            base_idx, base_idx + 1,
            base_idx + 2, base_idx + 3,
        ]);
    }

    let normals = vec![[0.0, 1.0, 0.0]; vertices.len()];

    Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_indices(Indices::U32(indices))
}
