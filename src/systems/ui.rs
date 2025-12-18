use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::window::{PrimaryWindow, CursorGrabMode};
use crate::components::{ClickText, FpsCounter, PauseMenu, ResumeButton, ExitButton, CameraController};
use crate::resources::GameState;

pub fn update_click_text_timer(
    time: Res<Time>,
    mut query: Query<(&mut Text, &mut ClickText)>,
) {
    for (mut text, mut click_text) in query.iter_mut() {
        if !click_text.timer.finished() {
            click_text.timer.tick(time.delta());

            if click_text.timer.just_finished() {
                **text = "".to_string();
            }
        }
    }
}

pub fn update_fps_counter(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<(&mut Text, &FpsCounter)>,
    camera_query: Query<(&Transform, &CameraController), With<CameraController>>,
) {
    for (mut text, fps_counter) in query.iter_mut() {
        if fps_counter.visible {
            let mut debug_text = String::new();

            // Add FPS
            if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
                if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
                    debug_text.push_str(&format!("FPS: {:.0}\n", fps_smoothed));
                } else {
                    debug_text.push_str("FPS: --\n");
                }
            } else {
                debug_text.push_str("FPS: --\n");
            }

            // Add position and compass
            if let Ok((transform, controller)) = camera_query.get_single() {
                let pos = transform.translation;
                debug_text.push_str(&format!("X: {:.1}\nY: {:.1}\nZ: {:.1}\n", pos.x, pos.y, pos.z));

                // Calculate cardinal direction from yaw
                let direction = get_cardinal_direction(controller.yaw);
                debug_text.push_str(&format!("Facing: {}", direction));
            }

            **text = debug_text;
        } else {
            **text = "".to_string();
        }
    }
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
    // 0° is East in Bevy's coordinate system
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

pub fn update_pause_menu_visibility(
    game_state: Res<GameState>,
    mut menu_query: Query<&mut Visibility, With<PauseMenu>>,
) {
    for mut visibility in menu_query.iter_mut() {
        *visibility = if game_state.paused {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn handle_pause_menu_buttons(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&ResumeButton>, Option<&ExitButton>),
        Changed<Interaction>,
    >,
    mut game_state: ResMut<GameState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for (interaction, mut color, resume_button, exit_button) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if resume_button.is_some() {
                    // Resume game
                    game_state.paused = false;
                    if let Ok(mut window) = windows.get_single_mut() {
                        window.cursor_options.grab_mode = CursorGrabMode::Locked;
                        window.cursor_options.visible = false;
                    }
                } else if exit_button.is_some() {
                    // Exit game
                    app_exit_events.send(AppExit::Success);
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.25, 0.25, 0.25));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.15, 0.15, 0.15));
            }
        }
    }
}
