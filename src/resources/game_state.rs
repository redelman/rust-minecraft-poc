use bevy::prelude::*;

#[derive(Resource)]
pub struct GameState {
    pub paused: bool,
    pub ui_visible: bool,
    pub creative_mode: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            paused: false,
            ui_visible: true,
            creative_mode: true, // Start in creative mode for now
        }
    }
}

/// Player health and hunger stats for survival mode
#[derive(Resource)]
pub struct PlayerStats {
    /// Current health (0-20, like Minecraft's 10 hearts * 2 half-hearts)
    pub health: u8,
    /// Maximum health
    pub max_health: u8,
    /// Current hunger/food level (0-20, like Minecraft's 10 drumsticks * 2 half-drumsticks)
    pub hunger: u8,
    /// Maximum hunger
    pub max_hunger: u8,
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            health: 20,
            max_health: 20,
            hunger: 20,
            max_hunger: 20,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChunkBorderMode {
    Off,
    Mode1,  // Single tall wireframe box around current chunk
    Mode2,  // Grid of boxes from ground to height for all blocks
}

#[derive(Resource)]
pub struct ChunkBorderState {
    pub mode: ChunkBorderMode,
}

impl Default for ChunkBorderState {
    fn default() -> Self {
        Self {
            mode: ChunkBorderMode::Off,
        }
    }
}
