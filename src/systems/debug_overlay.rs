use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::components::{CameraController, DebugOverlay};
use crate::blocks::{BlockRegistry, BlockId};
use crate::world::{Chunk, ChunkCoord, CHUNK_SIZE};
use crate::systems::{TimeOfDay, SkyLightLevel};

/// System to update debug overlay with FPS, position, and block info
pub fn update_debug_overlay(
    diagnostics: Res<DiagnosticsStore>,
    camera_query: Query<(&Transform, &CameraController), With<CameraController>>,
    chunk_query: Query<(&Chunk, &Transform), Without<CameraController>>,
    mut debug_text_query: Query<(&mut Text, &DebugOverlay)>,
    block_registry: Res<BlockRegistry>,
    time_of_day: Res<TimeOfDay>,
    sky_light: Res<SkyLightLevel>,
) {
    let Ok((camera_transform, controller)) = camera_query.get_single() else {
        return;
    };

    let Ok((mut text, debug_overlay)) = debug_text_query.get_single_mut() else {
        return;
    };

    if !debug_overlay.visible {
        **text = "".to_string();
        return;
    }

    let camera_pos = camera_transform.translation;
    let mut debug_text = String::new();

    // Add FPS
    if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
            debug_text.push_str(&format!("FPS: {:.0}\n", fps_smoothed));
        }
    }

    // Add time of day info
    let hour = time_of_day.hour();
    let hour_int = hour as u32;
    let minute = ((hour % 1.0) * 60.0) as u32;
    let time_status = if time_of_day.paused { " (PAUSED)" } else { "" };
    let speed_text = if time_of_day.speed != 1.0 { format!(" x{}", time_of_day.speed) } else { String::new() };
    debug_text.push_str(&format!("Time: {:02}:{:02}{}{}\n", hour_int, minute, speed_text, time_status));
    debug_text.push_str(&format!("Sky Light: {}\n", sky_light.level));

    // Add position
    debug_text.push_str(&format!("X: {:.1}\nY: {:.1}\nZ: {:.1}\n", camera_pos.x, camera_pos.y, camera_pos.z));

    // Add cardinal direction
    let direction = get_cardinal_direction(controller.yaw);
    debug_text.push_str(&format!("Facing: {}\n", direction));

    // Get block player is standing on
    // Player's eyes are at camera_pos, feet are 1.6 blocks below
    // Check just below the feet (0.1 blocks down) to get the block they're standing ON
    let standing_on_pos = camera_pos - Vec3::Y * 1.7; // Slightly below feet level
    let player_block_pos = camera_pos - Vec3::Y * 0.5; // Air block at player's body

    let standing_on_block = get_block_at_world_pos(standing_on_pos, &chunk_query);
    let standing_on_name = get_block_name(standing_on_block, &block_registry);

    // Get light level at player position
    let light_at_player = get_light_at_world_pos(player_block_pos, &chunk_query);

    debug_text.push_str(&format!("Standing on: {}\n", standing_on_name));
    debug_text.push_str(&format!("Block Light: {}\n", light_at_player));

    // Raycast to find block player is looking at
    let raycast_result = raycast_block(camera_transform, &chunk_query, 8.0);
    let looking_at_name = get_block_name(raycast_result.block_id, &block_registry);

    // Get light at the face being looked at (the air block in front of the solid block)
    let face_light = raycast_result.air_pos
        .map(|pos| get_light_at_world_pos(pos, &chunk_query))
        .unwrap_or(0);

    debug_text.push_str(&format!("Looking at: {} (light: {})", looking_at_name, face_light));

    **text = debug_text;
}

fn get_cardinal_direction(yaw: f32) -> &'static str {
    // Normalize yaw to 0-2π range
    let mut normalized_yaw = yaw % (2.0 * std::f32::consts::PI);
    if normalized_yaw < 0.0 {
        normalized_yaw += 2.0 * std::f32::consts::PI;
    }

    // Convert to degrees for easier calculation
    let degrees = normalized_yaw.to_degrees();

    // Determine cardinal direction (with 45-degree ranges)
    match degrees {
        d if d >= 337.5 || d < 22.5 => "E",
        d if d >= 22.5 && d < 67.5 => "NE",
        d if d >= 67.5 && d < 112.5 => "N",
        d if d >= 112.5 && d < 157.5 => "NW",
        d if d >= 157.5 && d < 202.5 => "W",
        d if d >= 202.5 && d < 247.5 => "SW",
        d if d >= 247.5 && d < 292.5 => "S",
        d if d >= 292.5 && d < 337.5 => "SE",
        _ => "?",
    }
}

/// Get block ID at world position
fn get_block_at_world_pos<F: bevy::ecs::query::QueryFilter>(
    world_pos: Vec3,
    chunk_query: &Query<(&Chunk, &Transform), F>,
) -> BlockId {
    // Find which chunk this position is in
    let chunk_coord = ChunkCoord::from_world_pos(world_pos);

    for (chunk, transform) in chunk_query.iter() {
        if chunk.coord == chunk_coord {
            // Convert world pos to chunk-local coordinates
            let local_pos = world_pos - transform.translation;
            let x = local_pos.x.floor() as i32;
            let y = local_pos.y.floor() as i32;
            let z = local_pos.z.floor() as i32;

            if x >= 0 && x < CHUNK_SIZE as i32 &&
               y >= 0 && y < CHUNK_SIZE as i32 &&
               z >= 0 && z < CHUNK_SIZE as i32 {
                return chunk.get_block(x as usize, y as usize, z as usize);
            }
        }
    }

    BlockId::AIR
}

/// Get light level at world position
fn get_light_at_world_pos<F: bevy::ecs::query::QueryFilter>(
    world_pos: Vec3,
    chunk_query: &Query<(&Chunk, &Transform), F>,
) -> u8 {
    // Find which chunk this position is in
    let chunk_coord = ChunkCoord::from_world_pos(world_pos);

    for (chunk, transform) in chunk_query.iter() {
        if chunk.coord == chunk_coord {
            // Convert world pos to chunk-local coordinates
            let local_pos = world_pos - transform.translation;
            let x = local_pos.x.floor() as i32;
            let y = local_pos.y.floor() as i32;
            let z = local_pos.z.floor() as i32;

            if x >= 0 && x < CHUNK_SIZE as i32 &&
               y >= 0 && y < CHUNK_SIZE as i32 &&
               z >= 0 && z < CHUNK_SIZE as i32 {
                return chunk.get_light(x as usize, y as usize, z as usize);
            }
        }
    }

    0
}

/// Result of raycast including block and position info
struct RaycastResult {
    block_id: BlockId,
    /// Position of the air block just before hitting the solid block (for light lookup)
    air_pos: Option<Vec3>,
}

/// Simple raycast to find block player is looking at
fn raycast_block<F: bevy::ecs::query::QueryFilter>(
    camera_transform: &Transform,
    chunk_query: &Query<(&Chunk, &Transform), F>,
    max_distance: f32,
) -> RaycastResult {
    let start = camera_transform.translation;
    let direction = camera_transform.forward();

    // Step along ray
    let step_size = 0.1;
    let steps = (max_distance / step_size) as i32;

    let mut last_air_pos: Option<Vec3> = None;

    for i in 1..steps {
        let pos = start + direction * (i as f32 * step_size);
        let block = get_block_at_world_pos(pos, chunk_query);

        if !block.is_air() {
            return RaycastResult {
                block_id: block,
                air_pos: last_air_pos,
            };
        }

        last_air_pos = Some(pos);
    }

    RaycastResult {
        block_id: BlockId::AIR,
        air_pos: None,
    }
}

/// Get human-readable block name
fn get_block_name(block_id: BlockId, registry: &BlockRegistry) -> String {
    if block_id.is_air() {
        return "Air".to_string();
    }

    if let Some(block_type) = registry.get_block(block_id) {
        block_type.properties.id.clone()
    } else {
        format!("Unknown (ID: {})", block_id.0)
    }
}
