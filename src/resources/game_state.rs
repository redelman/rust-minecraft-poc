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
#[allow(dead_code)]
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

/// FPS tracking for debug overlay
#[derive(Resource)]
pub struct FpsStats {
    /// Minimum FPS recorded since reset
    pub min_fps: f64,
    /// Maximum FPS recorded since reset
    pub max_fps: f64,
    /// Sum of all FPS readings (for average calculation)
    fps_sum: f64,
    /// Number of FPS readings taken
    sample_count: u64,
}

impl Default for FpsStats {
    fn default() -> Self {
        Self {
            min_fps: f64::MAX,
            max_fps: 0.0,
            fps_sum: 0.0,
            sample_count: 0,
        }
    }
}

#[allow(dead_code)]
impl FpsStats {
    /// Update with new FPS reading
    /// Ignores values below 1.0 FPS (startup noise)
    pub fn update(&mut self, fps: f64) {
        if fps >= 1.0 {
            self.min_fps = self.min_fps.min(fps);
            self.max_fps = self.max_fps.max(fps);
            self.fps_sum += fps;
            self.sample_count += 1;
        }
    }

    /// Get average FPS, or None if no samples yet
    pub fn average(&self) -> Option<f64> {
        if self.sample_count > 0 {
            Some(self.fps_sum / self.sample_count as f64)
        } else {
            None
        }
    }

    /// Check if we have any valid samples
    pub fn initialized(&self) -> bool {
        self.sample_count > 0
    }

    /// Reset all tracking
    pub fn reset(&mut self) {
        self.min_fps = f64::MAX;
        self.max_fps = 0.0;
        self.fps_sum = 0.0;
        self.sample_count = 0;
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
