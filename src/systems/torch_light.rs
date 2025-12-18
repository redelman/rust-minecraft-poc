use bevy::prelude::*;
use crate::components::CameraController;
use crate::resources::PlayerInventory;

/// Marker component for the torch's point light
#[derive(Component)]
pub struct TorchLight;

/// Spawn or despawn the torch light based on inventory selection
pub fn update_torch_light(
    mut commands: Commands,
    inventory: Res<PlayerInventory>,
    camera_query: Query<&Transform, With<CameraController>>,
    torch_light_query: Query<Entity, With<TorchLight>>,
) {
    let torch_selected = inventory.is_torch_selected();
    let torch_light_exists = !torch_light_query.is_empty();

    if torch_selected && !torch_light_exists {
        // Spawn torch light when torch is selected
        if let Ok(camera_transform) = camera_query.get_single() {
            commands.spawn((
                PointLight {
                    color: Color::srgb(1.0, 0.85, 0.5), // Warm orange/yellow torch color
                    intensity: 800_000.0,               // Bright enough to illuminate surroundings
                    range: 20.0,                        // 20 block range
                    shadows_enabled: true,
                    ..default()
                },
                Transform::from_translation(camera_transform.translation),
                TorchLight,
            ));
        }
    } else if !torch_selected && torch_light_exists {
        // Despawn torch light when torch is deselected
        for entity in torch_light_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

/// Update torch light position to follow the player
pub fn follow_player_with_torch_light(
    inventory: Res<PlayerInventory>,
    camera_query: Query<&Transform, With<CameraController>>,
    mut torch_light_query: Query<&mut Transform, (With<TorchLight>, Without<CameraController>)>,
) {
    // Only update if torch is selected
    if !inventory.is_torch_selected() {
        return;
    }

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    for mut light_transform in torch_light_query.iter_mut() {
        // Position the light slightly in front and below the camera (like holding a torch)
        let forward = camera_transform.forward();
        let offset = forward * 0.5 + Vec3::new(0.0, -0.3, 0.0);
        light_transform.translation = camera_transform.translation + offset;
    }
}
