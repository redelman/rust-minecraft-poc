use bevy::prelude::*;

#[derive(Component)]
pub struct ClickText {
    pub timer: Timer,
}

#[derive(Component)]
pub struct FpsCounter {
    pub visible: bool,
}

#[derive(Component)]
pub struct DebugOverlay {
    pub visible: bool,
}

#[derive(Component)]
pub struct PauseMenu;

#[derive(Component)]
pub struct ResumeButton;

#[derive(Component)]
pub struct ExitButton;

#[derive(Component)]
pub struct ChunkBorder;

#[derive(Component)]
pub struct BlockHighlight;

#[derive(Component)]
pub struct Hotbar;

#[derive(Component)]
pub struct HotbarSlot {
    pub slot_index: usize,
}

/// Icon displaying the block texture in a hotbar slot
#[derive(Component)]
pub struct HotbarSlotIcon {
    pub slot_index: usize,
}

/// Text label for items in a hotbar slot (used for items without textures)
#[derive(Component)]
pub struct HotbarSlotText {
    pub slot_index: usize,
}

/// Container for health and hunger bars (survival mode only)
#[derive(Component)]
pub struct SurvivalBars;

/// Background/outline for heart icons
#[derive(Component)]
pub struct HeartBackground {
    /// Heart index (0-9 for 10 hearts)
    pub index: usize,
}

/// Individual heart icon in the health bar (foreground showing fill state)
#[derive(Component)]
pub struct HeartIcon {
    /// Heart index (0-9 for 10 hearts)
    pub index: usize,
}

/// Background/outline for hunger icons
#[derive(Component)]
pub struct HungerBackground {
    /// Hunger icon index (0-9 for 10 drumsticks)
    pub index: usize,
}

/// Individual hunger/drumstick icon in the hunger bar (foreground showing fill state)
#[derive(Component)]
pub struct HungerIcon {
    /// Hunger icon index (0-9 for 10 drumsticks)
    pub index: usize,
}
