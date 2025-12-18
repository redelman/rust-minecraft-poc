use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{PrimaryWindow, CursorGrabMode, WindowFocused};
use crate::components::CameraController;
use crate::world::{Chunk, ChunkCoord, CHUNK_SIZE};
use crate::resources::ChunkManager;
use crate::blocks::BlockRegistry;
use crate::resources::GameState;

pub fn setup_cursor_grab(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

/// Re-grab cursor when window regains focus (e.g., after alt-tabbing)
pub fn handle_window_focus(
    mut focus_events: EventReader<WindowFocused>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    game_state: Res<GameState>,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    // Handle focus events
    for event in focus_events.read() {
        if event.focused {
            // Window gained focus - re-grab cursor if game is not paused
            if !game_state.paused {
                if let Ok(mut window) = windows.get_single_mut() {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
            }
        }
    }

    // Also re-grab on any mouse click when not paused (fallback for alt-tab issues)
    if !game_state.paused && (mouse_button.just_pressed(MouseButton::Left) || mouse_button.just_pressed(MouseButton::Right)) {
        if let Ok(mut window) = windows.get_single_mut() {
            if window.cursor_options.grab_mode != CursorGrabMode::Locked {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

pub fn camera_movement_controls(
    mut query: Query<(&mut Transform, &mut CameraController)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    chunk_manager: Res<ChunkManager>,
    chunks: Query<&Chunk>,
    block_registry: Res<BlockRegistry>,
) {
    let dt = time.delta_secs();
    let current_time = time.elapsed_secs();

    for (mut transform, mut controller) in query.iter_mut() {
        // Check for double-tap space to toggle fly mode
        if keyboard_input.just_pressed(KeyCode::Space) {
            let time_since_last_press = current_time - controller.last_space_press;
            if time_since_last_press < 0.3 {
                // Double-tap detected - toggle fly mode
                controller.is_flying = !controller.is_flying;
                controller.velocity_y = 0.0;
            } else if !controller.is_flying && controller.is_grounded {
                // Single tap - jump (only if grounded and not flying)
                controller.velocity_y = controller.jump_force;
            }
            controller.last_space_press = current_time;
        }

        // Calculate movement speed (with sprint)
        let mut speed = controller.move_speed;
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            speed *= controller.sprint_multiplier;
        }

        // Calculate horizontal velocity
        let mut velocity = Vec3::ZERO;
        let forward = Vec3::new(transform.forward().x, 0.0, transform.forward().z).normalize_or_zero();
        let right = Vec3::new(transform.right().x, 0.0, transform.right().z).normalize_or_zero();

        // WASD movement
        if keyboard_input.pressed(KeyCode::KeyW) {
            velocity += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            velocity -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            velocity -= right;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            velocity += right;
        }

        // Normalize horizontal movement
        if velocity.length() > 0.0 {
            velocity = velocity.normalize();
        }

        // Apply horizontal movement
        let horizontal_delta = velocity * speed * dt;

        // Player AABB dimensions
        const PLAYER_WIDTH: f32 = 0.6;
        const PLAYER_HALF_WIDTH: f32 = PLAYER_WIDTH / 2.0;
        const PLAYER_HEIGHT: f32 = 1.8;
        const PLAYER_EYE_HEIGHT: f32 = 1.6; // Camera is 1.6 blocks above feet

        if controller.is_flying {
            // Flying mode - free vertical movement with collision detection
            const EPSILON: f32 = 0.01; // Small value to prevent collision with block we're standing on

            // Try vertical movement with collision check
            let mut new_y = transform.translation.y;
            if keyboard_input.pressed(KeyCode::Space) {
                new_y += speed * dt;
            }
            if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
                new_y -= speed * dt;
            }

            // Check vertical collision using AABB (add epsilon to feet)
            let feet_y = new_y - PLAYER_EYE_HEIGHT;
            let head_y = feet_y + PLAYER_HEIGHT;
            let min = Vec3::new(
                transform.translation.x - PLAYER_HALF_WIDTH,
                feet_y + EPSILON,
                transform.translation.z - PLAYER_HALF_WIDTH
            );
            let max = Vec3::new(
                transform.translation.x + PLAYER_HALF_WIDTH,
                head_y,
                transform.translation.z + PLAYER_HALF_WIDTH
            );

            if !check_aabb_collision(min, max, &chunk_manager, &chunks, &block_registry) {
                transform.translation.y = new_y;
            }

            // Try horizontal movement with collision check (add epsilon to feet)
            let new_x = transform.translation.x + horizontal_delta.x;
            let new_z = transform.translation.z + horizontal_delta.z;
            let feet_y = transform.translation.y - PLAYER_EYE_HEIGHT;
            let head_y = feet_y + PLAYER_HEIGHT;
            let min = Vec3::new(
                new_x - PLAYER_HALF_WIDTH,
                feet_y + EPSILON,
                new_z - PLAYER_HALF_WIDTH
            );
            let max = Vec3::new(
                new_x + PLAYER_HALF_WIDTH,
                head_y,
                new_z + PLAYER_HALF_WIDTH
            );

            if !check_aabb_collision(min, max, &chunk_manager, &chunks, &block_registry) {
                transform.translation.x = new_x;
                transform.translation.z = new_z;
            }

            controller.is_grounded = false;
        } else {
            // Walking mode - apply gravity and collision

            // First, check if we're currently stuck inside a block and push up if so
            let current_feet_y = transform.translation.y - PLAYER_EYE_HEIGHT;
            let current_head_y = current_feet_y + PLAYER_HEIGHT;
            let current_min = Vec3::new(
                transform.translation.x - PLAYER_HALF_WIDTH,
                current_feet_y + 0.01, // Small offset to ignore ground
                transform.translation.z - PLAYER_HALF_WIDTH
            );
            let current_max = Vec3::new(
                transform.translation.x + PLAYER_HALF_WIDTH,
                current_head_y,
                transform.translation.z + PLAYER_HALF_WIDTH
            );

            if check_aabb_collision(current_min, current_max, &chunk_manager, &chunks, &block_registry) {
                // We're stuck inside a block - push up until we're free
                transform.translation.y += 0.5;
                controller.velocity_y = 0.0;
                return; // Skip normal movement this frame
            }

            // Apply gravity
            controller.velocity_y -= controller.gravity * dt;

            // Calculate new Y position
            let new_y = transform.translation.y + controller.velocity_y * dt;
            let new_feet_y = new_y - PLAYER_EYE_HEIGHT;
            let new_head_y = new_feet_y + PLAYER_HEIGHT;

            // Check vertical collision with AABB
            let min = Vec3::new(
                transform.translation.x - PLAYER_HALF_WIDTH,
                new_feet_y,
                transform.translation.z - PLAYER_HALF_WIDTH
            );
            let max = Vec3::new(
                transform.translation.x + PLAYER_HALF_WIDTH,
                new_head_y,
                transform.translation.z + PLAYER_HALF_WIDTH
            );

            if check_aabb_collision(min, max, &chunk_manager, &chunks, &block_registry) {
                if controller.velocity_y > 0.0 {
                    // Hit ceiling while rising - stop vertical movement
                    controller.velocity_y = 0.0;
                } else {
                    // Hit ground while falling - stop and mark as grounded
                    controller.is_grounded = true;
                    controller.velocity_y = 0.0;
                    // Don't change Y position - we stay where we are
                }
            } else {
                // No collision - apply vertical movement
                controller.is_grounded = false;
                transform.translation.y = new_y;
            }

            // Try horizontal movement with AABB collision check
            if horizontal_delta.length() > 0.0001 {
                let new_x = transform.translation.x + horizontal_delta.x;
                let new_z = transform.translation.z + horizontal_delta.z;
                let feet_y = transform.translation.y - PLAYER_EYE_HEIGHT;
                let head_y = feet_y + PLAYER_HEIGHT;

                // Check collision at the new horizontal position
                // Use small offset above feet to not collide with ground we're standing on
                let min = Vec3::new(
                    new_x - PLAYER_HALF_WIDTH,
                    feet_y + 0.01,
                    new_z - PLAYER_HALF_WIDTH
                );
                let max = Vec3::new(
                    new_x + PLAYER_HALF_WIDTH,
                    head_y,
                    new_z + PLAYER_HALF_WIDTH
                );

                if !check_aabb_collision(min, max, &chunk_manager, &chunks, &block_registry) {
                    // Move horizontally
                    transform.translation.x = new_x;
                    transform.translation.z = new_z;
                }
            }
        }
    }
}

/// Check if an AABB (axis-aligned bounding box) collides with any solid blocks
/// Returns true if there is a collision
fn check_aabb_collision(
    min: Vec3,
    max: Vec3,
    chunk_manager: &ChunkManager,
    chunks: &Query<&Chunk>,
    block_registry: &BlockRegistry,
) -> bool {
    // Get the range of blocks the AABB overlaps
    let min_block = IVec3::new(min.x.floor() as i32, min.y.floor() as i32, min.z.floor() as i32);
    let max_block = IVec3::new(max.x.floor() as i32, max.y.floor() as i32, max.z.floor() as i32);

    // Check all blocks that the AABB overlaps
    for by in min_block.y..=max_block.y {
        for bz in min_block.z..=max_block.z {
            for bx in min_block.x..=max_block.x {
                let block_pos = Vec3::new(bx as f32 + 0.5, by as f32 + 0.5, bz as f32 + 0.5);
                let chunk_coord = ChunkCoord::from_world_pos(block_pos);

                if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&chunk_coord) {
                    if let Ok(chunk) = chunks.get(chunk_entity) {
                        // Convert world position to local chunk position
                        let local_x = (bx - chunk_coord.x * CHUNK_SIZE as i32) as usize;
                        let local_y = (by - chunk_coord.y * CHUNK_SIZE as i32) as usize;
                        let local_z = (bz - chunk_coord.z * CHUNK_SIZE as i32) as usize;

                        if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE && local_z < CHUNK_SIZE {
                            let block_id = chunk.get_block(local_x, local_y, local_z);
                            if !block_id.is_air() {
                                if let Some(block_type) = block_registry.get_block(block_id) {
                                    if block_type.properties.is_solid {
                                        return true; // Collision detected
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    false // No collision
}

fn get_terrain_height_at(
    pos: Vec3,
    chunk_manager: &ChunkManager,
    chunks: &Query<&Chunk>,
    block_registry: &BlockRegistry,
) -> f32 {
    // Get the chunk X and Z coordinates
    let chunk_x = (pos.x / CHUNK_SIZE as f32).floor() as i32;
    let chunk_z = (pos.z / CHUNK_SIZE as f32).floor() as i32;

    // Calculate local position within chunk
    let local_x = ((pos.x - chunk_x as f32 * CHUNK_SIZE as f32).floor() as i32).max(0).min(CHUNK_SIZE as i32 - 1) as usize;
    let local_z = ((pos.z - chunk_z as f32 * CHUNK_SIZE as f32).floor() as i32).max(0).min(CHUNK_SIZE as i32 - 1) as usize;

    // Search from top to bottom across ALL vertically stacked chunks
    // Start from a reasonable max height and work down
    for world_y in (0..256).rev() {
        let chunk_y = world_y / CHUNK_SIZE;
        let local_y = world_y % CHUNK_SIZE;

        let chunk_coord = ChunkCoord::new(chunk_x, chunk_y as i32, chunk_z);

        if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&chunk_coord) {
            if let Ok(chunk) = chunks.get(chunk_entity) {
                let block_id = chunk.get_block(local_x, local_y, local_z);
                if !block_id.is_air() {
                    if let Some(block_type) = block_registry.get_block(block_id) {
                        if block_type.properties.is_solid {
                            // Return the top of this block (world_y + 1)
                            return world_y as f32 + 1.0;
                        }
                    }
                }
            }
        }
    }

    1.0 // Default to Y=1 if no terrain found
}

pub fn camera_look_controls(
    mut query: Query<(&mut Transform, &mut CameraController)>,
    mut mouse_motion_events: EventReader<MouseMotion>,
) {
    for (mut transform, mut controller) in query.iter_mut() {
        // FPS-style camera: always rotate with mouse movement (cursor is locked)
        for event in mouse_motion_events.read() {
            // Update yaw (horizontal rotation)
            controller.yaw -= event.delta.x * controller.look_sensitivity;

            // Update pitch (vertical rotation)
            controller.pitch -= event.delta.y * controller.look_sensitivity;

            // Clamp pitch to prevent camera flipping
            controller.pitch = controller.pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
        }

        // Apply rotation to transform
        transform.rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
    }
}
