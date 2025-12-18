use bevy::prelude::*;
use bevy::window::{PrimaryWindow, CursorGrabMode};
use crate::components::DebugOverlay;
use crate::resources::{GameState, ChunkBorderState, ChunkBorderMode};

pub fn toggle_pause_menu(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        game_state.paused = !game_state.paused;

        // Toggle cursor grab mode
        if let Ok(mut window) = windows.get_single_mut() {
            if game_state.paused {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            } else {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

pub fn toggle_debug_overlay(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut DebugOverlay>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        for mut debug_overlay in query.iter_mut() {
            debug_overlay.visible = !debug_overlay.visible;
        }
    }
}

pub fn toggle_chunk_borders(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut border_state: ResMut<ChunkBorderState>,
) {
    if keyboard_input.just_pressed(KeyCode::F9) {
        // Cycle through modes: Off -> Mode1 -> Mode2 -> Off
        border_state.mode = match border_state.mode {
            ChunkBorderMode::Off => ChunkBorderMode::Mode1,
            ChunkBorderMode::Mode1 => ChunkBorderMode::Mode2,
            ChunkBorderMode::Mode2 => ChunkBorderMode::Off,
        };
    }
}

pub fn toggle_ui_visibility(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F1) {
        game_state.ui_visible = !game_state.ui_visible;
    }
}

pub fn take_screenshot(
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Shift+F2 reminder to use OS screenshot tool
    if keyboard_input.just_pressed(KeyCode::F2) &&
       (keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight)) {
        info!("Screenshot: Use your OS screenshot tool");
        info!("Windows: Win+Shift+S | Mac: Cmd+Shift+4 | Linux: varies by DE");
    }
}

pub fn toggle_creative_mode(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<GameState>,
) {
    if keyboard_input.just_pressed(KeyCode::F4) {
        game_state.creative_mode = !game_state.creative_mode;
        let mode = if game_state.creative_mode { "Creative" } else { "Survival" };
        info!("Game mode changed to: {}", mode);
    }
}
