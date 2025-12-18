use bevy::prelude::*;

#[derive(Component)]
pub struct CameraController {
    pub yaw: f32,   // Horizontal rotation (left/right)
    pub pitch: f32, // Vertical rotation (up/down)
    pub move_speed: f32,
    pub sprint_multiplier: f32,
    pub look_sensitivity: f32,
    pub is_flying: bool,
    pub velocity_y: f32,
    pub gravity: f32,
    pub jump_force: f32,
    pub is_grounded: bool,
    pub last_space_press: f32, // For double-tap detection
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            move_speed: 5.0,
            sprint_multiplier: 2.0,
            look_sensitivity: 0.003,
            is_flying: false,
            velocity_y: 0.0,
            gravity: 20.0,
            jump_force: 8.0,
            is_grounded: false,
            last_space_press: -1.0,
        }
    }
}
