use bevy::prelude::*;

/// Daytime skybox component
#[derive(Component)]
pub struct Skybox;

/// Night skybox with stars (separate mesh that fades in at night)
#[derive(Component)]
pub struct NightSkybox;

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct Moon;

/// Marker for the sun's directional light
#[derive(Component)]
pub struct SunLight;
