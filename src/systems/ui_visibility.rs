use bevy::prelude::*;
use crate::components::{Hotbar, DebugOverlay, Crosshair, SurvivalBars, HeartIcon, HungerIcon};
use crate::resources::{GameState, PlayerStats};

// Icon atlas constants (icons.png is 256x256, icons are 9x9 pixels)
const ICON_PIXELS: f32 = 9.0;

// Heart UV coordinates (row v=0)
const HEART_FULL_U: f32 = 52.0;    // Full red heart
const HEART_HALF_U: f32 = 61.0;    // Half heart
const HEART_V: f32 = 0.0;

// Hunger UV coordinates (row v=27)
const HUNGER_FULL_U: f32 = 52.0;   // Full haunch
const HUNGER_HALF_U: f32 = 61.0;   // Half haunch
const HUNGER_V: f32 = 27.0;

// Create rect from pixel coordinates (Bevy's ImageNode.rect uses pixel coords)
fn make_icon_rect(u: f32, v: f32) -> Rect {
    Rect::new(u, v, u + ICON_PIXELS, v + ICON_PIXELS)
}

/// Update hotbar visibility based on UI state
pub fn update_hotbar_visibility(
    game_state: Res<GameState>,
    mut hotbar_query: Query<&mut Visibility, With<Hotbar>>,
) {
    if !game_state.is_changed() {
        return;
    }

    for mut visibility in hotbar_query.iter_mut() {
        *visibility = if game_state.ui_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update crosshair visibility based on UI state
pub fn update_crosshair_visibility(
    game_state: Res<GameState>,
    mut crosshair_query: Query<&mut Visibility, With<Crosshair>>,
) {
    if !game_state.is_changed() {
        return;
    }

    for mut visibility in crosshair_query.iter_mut() {
        *visibility = if game_state.ui_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Hide debug overlay when UI is hidden (F3 still toggles it when UI is visible)
pub fn update_debug_visibility(
    game_state: Res<GameState>,
    mut debug_query: Query<(&DebugOverlay, &mut Visibility), Changed<DebugOverlay>>,
) {
    for (debug_overlay, mut visibility) in debug_query.iter_mut() {
        *visibility = if game_state.ui_visible && debug_overlay.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update debug overlay visibility when UI state changes
pub fn update_debug_visibility_on_ui_toggle(
    game_state: Res<GameState>,
    mut debug_query: Query<(&DebugOverlay, &mut Visibility)>,
) {
    if !game_state.is_changed() {
        return;
    }

    for (debug_overlay, mut visibility) in debug_query.iter_mut() {
        *visibility = if game_state.ui_visible && debug_overlay.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update survival bars visibility based on game mode
/// Visible in survival mode, hidden in creative mode
pub fn update_survival_bars_visibility(
    game_state: Res<GameState>,
    mut bars_query: Query<&mut Visibility, With<SurvivalBars>>,
) {
    if !game_state.is_changed() {
        return;
    }

    for mut visibility in bars_query.iter_mut() {
        // Show in survival mode when UI is visible
        *visibility = if game_state.ui_visible && !game_state.creative_mode {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update heart icons based on player health
/// Changes the UV rect to show full, half, or hidden hearts
pub fn update_health_display(
    stats: Res<PlayerStats>,
    mut heart_query: Query<(&HeartIcon, &mut ImageNode, &mut Visibility)>,
) {
    if !stats.is_changed() {
        return;
    }

    // Each heart represents 2 health points
    // Full heart: index * 2 + 2 <= health
    // Half heart: index * 2 + 1 <= health < index * 2 + 2
    // Empty: health < index * 2 + 1 (hide the foreground, background still shows)
    for (heart, mut image, mut visibility) in heart_query.iter_mut() {
        let threshold = (heart.index as u8) * 2 + 2;
        let half_threshold = (heart.index as u8) * 2 + 1;

        if stats.health >= threshold {
            // Full heart
            image.rect = Some(make_icon_rect(HEART_FULL_U, HEART_V));
            *visibility = Visibility::Inherited;
        } else if stats.health >= half_threshold {
            // Half heart
            image.rect = Some(make_icon_rect(HEART_HALF_U, HEART_V));
            *visibility = Visibility::Inherited;
        } else {
            // Empty - hide foreground (background outline still visible)
            *visibility = Visibility::Hidden;
        }
    }
}

/// Update hunger icons based on player hunger level
/// Changes the UV rect to show full, half, or hidden haunches
pub fn update_hunger_display(
    stats: Res<PlayerStats>,
    mut hunger_query: Query<(&HungerIcon, &mut ImageNode, &mut Visibility)>,
) {
    if !stats.is_changed() {
        return;
    }

    // Each drumstick represents 2 hunger points
    for (hunger, mut image, mut visibility) in hunger_query.iter_mut() {
        let threshold = (hunger.index as u8) * 2 + 2;
        let half_threshold = (hunger.index as u8) * 2 + 1;

        if stats.hunger >= threshold {
            // Full haunch
            image.rect = Some(make_icon_rect(HUNGER_FULL_U, HUNGER_V));
            *visibility = Visibility::Inherited;
        } else if stats.hunger >= half_threshold {
            // Half haunch
            image.rect = Some(make_icon_rect(HUNGER_HALF_U, HUNGER_V));
            *visibility = Visibility::Inherited;
        } else {
            // Empty - hide foreground (background outline still visible)
            *visibility = Visibility::Hidden;
        }
    }
}
