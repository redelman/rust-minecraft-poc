use bevy::prelude::*;
use bevy::pbr::StandardMaterial;
use crate::components::CameraController;
use crate::components::{Skybox, NightSkybox, Sun, Moon};
use crate::world::{ChunkManager, ChunkCoord, MAX_LIGHT_LEVEL};
use crate::systems::NeedsRemesh;

/// Maximum chunk distance (in chunks) to remesh during sky light updates
/// Chunks beyond this distance will be remeshed lazily when they come into view
const SKY_LIGHT_REMESH_DISTANCE: i32 = 4;

/// Time of day resource (0.0 = midnight, 0.5 = noon, 1.0 = midnight again)
/// A full day cycle takes DAY_LENGTH_SECONDS real-world seconds
#[derive(Resource)]
pub struct TimeOfDay {
    /// Current time as a fraction of the day (0.0 to 1.0)
    pub time: f32,
    /// Speed multiplier (1.0 = normal, 2.0 = double speed, etc.)
    pub speed: f32,
    /// Whether time is currently paused
    pub paused: bool,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self {
            time: 0.25, // Start at sunrise (6:00 AM)
            speed: 1.0,
            paused: false,
        }
    }
}

impl TimeOfDay {
    /// Length of a full day in real-world seconds (at speed 1.0)
    /// Minecraft's day is 20 minutes (1200 seconds)
    pub const DAY_LENGTH_SECONDS: f32 = 1200.0;

    /// Get the current hour (0-24)
    pub fn hour(&self) -> f32 {
        self.time * 24.0
    }

    /// Check if it's daytime (sun above horizon)
    /// Sun is above horizon when time is between 0.0 and 0.5 (midnight to noon to midnight)
    /// Actually: sun_y = sin(time * 2π), so sun is above (y > 0) when time is 0.0-0.5
    /// But we want sunrise at 6:00 (0.25) and sunset at 18:00 (0.75)
    /// So daytime is when 0.25 <= time < 0.75
    pub fn is_day(&self) -> bool {
        // Sun Y position = sin(time * 2π)
        // At time=0.25: sin(π/2) = 1 (rising, at horizon going up... wait)
        // Actually with angle = time * 2π:
        // time=0: angle=0, sin=0, cos=1 -> sun at +X, Y=0 (horizon east)
        // time=0.25: angle=π/2, sin=1, cos=0 -> sun at Y=+1, X=0 (overhead)
        // Hmm, that's noon at 6:00 AM... let me recalculate
        // We want: time=0.25 (6AM) -> sunrise (east horizon), time=0.5 (noon) -> overhead
        // So we need to offset: angle = (time - 0.25) * 2π
        // time=0.25: angle=0, cos=1, sin=0 -> +X (east), Y=0 ✓
        // time=0.5: angle=π/2, cos=0, sin=1 -> X=0, +Y ✓
        // time=0.75: angle=π, cos=-1, sin=0 -> -X (west), Y=0 ✓
        // time=0.0/1.0: angle=-π/2 or 3π/2, cos=0, sin=-1 -> X=0, -Y (below) ✓
        //
        // Sun is above horizon (Y > 0) when sin(angle) > 0
        // sin((time - 0.25) * 2π) > 0 when 0 < (time - 0.25) * 2π < π
        // i.e., when 0 < time - 0.25 < 0.5
        // i.e., when 0.25 < time < 0.75
        self.time > 0.25 && self.time < 0.75
    }

    /// Get sun altitude as a normalized value (-1 to 1)
    /// -1 = directly below, 0 = horizon, 1 = directly overhead
    #[allow(dead_code)]
    pub fn sun_altitude(&self) -> f32 {
        let angle = (self.time - 0.25) * 2.0 * std::f32::consts::PI;
        angle.sin()
    }

    /// Get sky light level (0-15) based on time of day
    /// Uses smooth transitions for gradual lighting changes
    pub fn sky_light_level(&self) -> u8 {
        let hour = self.hour();

        // Smooth sine-based transition
        // Day (7:00-17:00): full light
        // Night (19:00-5:00): moonlight (level 4)
        // Transitions: 1-hour ramps at sunrise/sunset

        if hour >= 7.0 && hour < 17.0 {
            // Full daylight
            MAX_LIGHT_LEVEL
        } else if hour >= 19.0 || hour < 5.0 {
            // Full night - moonlight
            4
        } else if hour >= 5.0 && hour < 7.0 {
            // Sunrise transition (5:00-7:00)
            let t = (hour - 5.0) / 2.0; // 0.0 to 1.0
            let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
            (4.0 + smooth_t * 11.0) as u8
        } else {
            // Sunset transition (17:00-19:00)
            let t = (hour - 17.0) / 2.0; // 0.0 to 1.0
            let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
            (15.0 - smooth_t * 11.0) as u8
        }
    }
}

/// Current global sky light level resource
/// This is updated based on TimeOfDay and used by mesh generation
#[derive(Resource)]
pub struct SkyLightLevel {
    pub level: u8,
}

impl Default for SkyLightLevel {
    fn default() -> Self {
        Self {
            level: MAX_LIGHT_LEVEL, // Start at full daylight
        }
    }
}

/// Component to track what sky light level a chunk was last rendered with
/// Used to detect stale chunks that need remeshing when player approaches
#[derive(Component)]
pub struct ChunkSkyLight {
    pub level: u8,
}

/// Update time of day based on real time
pub fn update_time_of_day(
    time: Res<Time>,
    mut time_of_day: ResMut<TimeOfDay>,
) {
    if time_of_day.paused {
        return;
    }

    let delta = time.delta_secs() * time_of_day.speed / TimeOfDay::DAY_LENGTH_SECONDS;
    time_of_day.time = (time_of_day.time + delta) % 1.0;
}

/// Update sky light level based on time of day
/// When it changes, mark nearby chunks for remeshing (distant chunks update lazily)
pub fn update_sky_light_level(
    time_of_day: Res<TimeOfDay>,
    mut sky_light: ResMut<SkyLightLevel>,
    chunk_manager: Res<ChunkManager>,
    camera_query: Query<&Transform, With<CameraController>>,
    mut commands: Commands,
) {
    let new_level = time_of_day.sky_light_level();

    if new_level != sky_light.level {
        sky_light.level = new_level;

        // Get player position to prioritize nearby chunks
        let player_chunk = if let Ok(camera_transform) = camera_query.get_single() {
            ChunkCoord::from_world_pos(camera_transform.translation)
        } else {
            // If no camera, just mark all chunks (fallback)
            for &entity in chunk_manager.loaded_chunks.values() {
                commands.entity(entity).insert(NeedsRemesh);
            }
            return;
        };

        // Only mark chunks within SKY_LIGHT_REMESH_DISTANCE for immediate remeshing
        // This prevents FPS drops during day/night transitions
        // Distant chunks will get their updated lighting when they're regenerated
        // or when the player moves closer
        for (coord, &entity) in chunk_manager.loaded_chunks.iter() {
            let dx = (coord.x - player_chunk.x).abs();
            let dy = (coord.y - player_chunk.y).abs();
            let dz = (coord.z - player_chunk.z).abs();

            // Use Chebyshev distance (max of all axes)
            let distance = dx.max(dy).max(dz);

            if distance <= SKY_LIGHT_REMESH_DISTANCE {
                commands.entity(entity).insert(NeedsRemesh);
            }
        }
    }
}

/// Update chunks with stale sky lighting as the player approaches them
/// This ensures distant chunks eventually get correct lighting without causing FPS drops
pub fn update_stale_chunk_lighting(
    sky_light: Res<SkyLightLevel>,
    chunk_manager: Res<ChunkManager>,
    camera_query: Query<&Transform, With<CameraController>>,
    chunk_light_query: Query<&ChunkSkyLight>,
    mut commands: Commands,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let player_chunk = ChunkCoord::from_world_pos(camera_transform.translation);
    let current_level = sky_light.level;

    // Check chunks within remesh distance + 1 for stale lighting
    // This creates a "buffer zone" where chunks get updated before the player reaches them
    let check_distance = SKY_LIGHT_REMESH_DISTANCE + 1;

    // Limit how many stale chunks we mark per frame to avoid stuttering
    let mut marked_count = 0;
    const MAX_STALE_PER_FRAME: usize = 4;

    for (coord, &entity) in chunk_manager.loaded_chunks.iter() {
        if marked_count >= MAX_STALE_PER_FRAME {
            break;
        }

        let dx = (coord.x - player_chunk.x).abs();
        let dy = (coord.y - player_chunk.y).abs();
        let dz = (coord.z - player_chunk.z).abs();
        let distance = dx.max(dy).max(dz);

        // Only check chunks within check_distance
        if distance > check_distance {
            continue;
        }

        // Check if this chunk has stale lighting
        if let Ok(chunk_light) = chunk_light_query.get(entity) {
            if chunk_light.level != current_level {
                commands.entity(entity).insert(NeedsRemesh);
                marked_count += 1;
            }
        } else {
            // No ChunkSkyLight component - this is a new chunk, mark it
            commands.entity(entity).insert(NeedsRemesh);
            marked_count += 1;
        }
    }
}

/// Update sun and moon positions and lighting based on time of day
pub fn update_sun_transform(
    time_of_day: Res<TimeOfDay>,
    camera_query: Query<&Transform, With<CameraController>>,
    mut sun_query: Query<&mut Transform, (With<Sun>, Without<CameraController>, Without<Moon>)>,
    mut moon_query: Query<&mut Transform, (With<Moon>, Without<CameraController>, Without<Sun>)>,
    mut directional_light_query: Query<(&mut DirectionalLight, &mut Transform), (Without<Sun>, Without<CameraController>, Without<Moon>)>,
    mut ambient_light: ResMut<AmbientLight>,
    mut clear_color: ResMut<ClearColor>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    // Celestial body distance from camera (far enough to look distant)
    let orbit_distance = 400.0;

    // Calculate sun angle based on time, offset so sunrise is at 6:00 AM (time=0.25)
    // angle = (time - 0.25) * 2π
    // time=0.25 (6:00 sunrise): angle=0, cos=1, sin=0 -> sun at +X (east), Y=0
    // time=0.5 (noon): angle=π/2, cos=0, sin=1 -> sun at X=0, +Y (overhead)
    // time=0.75 (18:00 sunset): angle=π, cos=-1, sin=0 -> sun at -X (west), Y=0
    // time=0.0/1.0 (midnight): angle=-π/2, cos=0, sin=-1 -> sun at X=0, -Y (below)
    let angle = (time_of_day.time - 0.25) * 2.0 * std::f32::consts::PI;

    // Sun position: rises in east (+X), peaks overhead (+Y), sets in west (-X)
    let sun_x = angle.cos() * orbit_distance;  // East at sunrise, west at sunset
    let sun_y = angle.sin() * orbit_distance;  // Below at midnight, above at noon

    let sun_pos = Vec3::new(
        camera_transform.translation.x + sun_x,
        camera_transform.translation.y + sun_y,
        camera_transform.translation.z,
    );

    // Moon is opposite the sun (180 degrees offset)
    let moon_pos = Vec3::new(
        camera_transform.translation.x - sun_x,
        camera_transform.translation.y - sun_y,
        camera_transform.translation.z,
    );

    // Update sun mesh position
    for mut sun_transform in sun_query.iter_mut() {
        sun_transform.translation = sun_pos;
    }

    // Update moon mesh position
    for mut moon_transform in moon_query.iter_mut() {
        moon_transform.translation = moon_pos;
    }

    // Update directional light
    let hour = time_of_day.hour();
    let is_day = time_of_day.is_day();

    for (mut light, mut light_transform) in directional_light_query.iter_mut() {
        if is_day {
            // Daytime: sun provides directional light
            let light_dir = (camera_transform.translation - sun_pos).normalize();
            *light_transform = Transform::from_translation(sun_pos).looking_to(light_dir, Vec3::Y);

            // Color and intensity vary through the day
            if hour >= 5.0 && hour <= 8.0 {
                // Sunrise: warm orange tones
                let t = ((hour - 5.0) / 3.0).clamp(0.0, 1.0);
                light.color = Color::srgb(1.0, 0.6 + t * 0.39, 0.3 + t * 0.65);
                light.illuminance = 5000.0 + t * 75000.0;
            } else if hour >= 16.0 && hour <= 19.0 {
                // Sunset: warm orange tones
                let t = ((hour - 16.0) / 3.0).clamp(0.0, 1.0);
                light.color = Color::srgb(1.0, 0.99 - t * 0.39, 0.95 - t * 0.65);
                light.illuminance = 80000.0 - t * 75000.0;
            } else {
                // Midday: bright white
                light.color = Color::srgb(1.0, 0.99, 0.95);
                light.illuminance = 80000.0;
            }
        } else {
            // Nighttime: moonlight - very dim
            light.color = Color::srgb(0.7, 0.8, 1.0);
            light.illuminance = 200.0; // Much dimmer at night

            let light_dir = (camera_transform.translation - moon_pos).normalize();
            *light_transform = Transform::from_translation(moon_pos).looking_to(light_dir, Vec3::Y);
        }
    }

    // Update ambient light
    if is_day {
        if hour >= 5.0 && hour <= 8.0 {
            let t = ((hour - 5.0) / 3.0).clamp(0.0, 1.0);
            ambient_light.color = Color::srgb(0.7 + t * 0.2, 0.5 + t * 0.45, 0.4 + t * 0.6);
            ambient_light.brightness = 50.0 + t * 1950.0;
        } else if hour >= 16.0 && hour <= 19.0 {
            let t = ((hour - 16.0) / 3.0).clamp(0.0, 1.0);
            ambient_light.color = Color::srgb(0.9 - t * 0.2, 0.95 - t * 0.45, 1.0 - t * 0.6);
            ambient_light.brightness = 2000.0 - t * 1950.0;
        } else {
            ambient_light.color = Color::srgb(0.9, 0.95, 1.0);
            ambient_light.brightness = 2000.0;
        }
    } else {
        // Night ambient - very dark with slight blue moonlight tint
        ambient_light.color = Color::srgb(0.3, 0.4, 0.6);
        ambient_light.brightness = 50.0;
    }

    // Update sky color (clear color) with proper sunrise/sunset colors
    // Using multi-stage transitions for realistic sky
    if hour >= 5.0 && hour < 6.0 {
        // Early dawn: dark blue -> purple/pink horizon glow
        let t = hour - 5.0; // 0.0 to 1.0
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.02 + smooth_t * 0.25,  // Dark -> rose
            0.03 + smooth_t * 0.08,  // Stay dark
            0.08 + smooth_t * 0.12,  // Dark blue -> purple
        );
    } else if hour >= 6.0 && hour < 7.0 {
        // Sunrise: pink/orange horizon colors
        let t = hour - 6.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.27 + smooth_t * 0.53,  // Rose -> warm orange
            0.11 + smooth_t * 0.34,  // Dark -> orange-ish
            0.20 - smooth_t * 0.05,  // Purple fades slightly
        );
    } else if hour >= 7.0 && hour < 8.0 {
        // Morning transition: orange -> blue sky
        let t = hour - 7.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.80 - smooth_t * 0.30,  // Orange -> light blue
            0.45 + smooth_t * 0.25,  // Warm -> cooler
            0.15 + smooth_t * 0.80,  // Orange -> blue
        );
    } else if hour >= 8.0 && hour < 17.0 {
        // Full daytime: bright blue sky
        clear_color.0 = Color::srgb(0.5, 0.7, 0.95);
    } else if hour >= 17.0 && hour < 18.0 {
        // Late afternoon: blue -> golden
        let t = hour - 17.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.5 + smooth_t * 0.35,   // Blue -> golden orange
            0.7 - smooth_t * 0.25,   // Sky dims
            0.95 - smooth_t * 0.55,  // Blue -> warm
        );
    } else if hour >= 18.0 && hour < 19.0 {
        // Sunset: deep orange/red
        let t = hour - 18.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.85 - smooth_t * 0.35,  // Orange -> red/purple
            0.45 - smooth_t * 0.30,  // Fading
            0.40 - smooth_t * 0.15,  // Warm -> purple
        );
    } else if hour >= 19.0 && hour < 20.0 {
        // Dusk: red/purple -> deep blue
        let t = hour - 19.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.50 - smooth_t * 0.40,  // Red fading
            0.15 - smooth_t * 0.10,  // Getting dark
            0.25 - smooth_t * 0.10,  // Purple -> dark blue
        );
    } else if hour >= 20.0 && hour < 21.0 {
        // Late dusk: twilight blue -> night
        let t = hour - 20.0;
        let smooth_t = t * t * (3.0 - 2.0 * t);
        clear_color.0 = Color::srgb(
            0.10 - smooth_t * 0.08,
            0.05 - smooth_t * 0.02,
            0.15 - smooth_t * 0.07,
        );
    } else {
        // Night sky - very dark blue/black
        clear_color.0 = Color::srgb(0.02, 0.03, 0.08);
    }
}

/// Update skybox position to follow the camera
pub fn update_sky_position(
    camera_query: Query<&Transform, With<CameraController>>,
    mut skybox_query: Query<&mut Transform, (With<Skybox>, Without<CameraController>, Without<Sun>, Without<Moon>, Without<NightSkybox>)>,
    mut night_skybox_query: Query<&mut Transform, (With<NightSkybox>, Without<CameraController>, Without<Sun>, Without<Moon>, Without<Skybox>)>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    // Update daytime skybox to follow camera position exactly
    for mut skybox_transform in skybox_query.iter_mut() {
        skybox_transform.translation = camera_transform.translation;
    }

    // Update night skybox to follow camera position exactly
    for mut skybox_transform in night_skybox_query.iter_mut() {
        skybox_transform.translation = camera_transform.translation;
    }
}

/// Update night skybox material alpha based on time of day
/// Stars fade in during sunset and out during sunrise
pub fn update_night_skybox_alpha(
    time_of_day: Res<TimeOfDay>,
    night_skybox_query: Query<&MeshMaterial3d<StandardMaterial>, With<NightSkybox>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let hour = time_of_day.hour();

    // Calculate star visibility alpha
    // Stars fully visible at night (20:00 - 5:00)
    // Fade in during sunset (18:30 - 20:00) - 1.5 hour transition
    // Fade out during sunrise (5:00 - 6:00) - 1 hour transition (stars disappear quickly once sun rises)
    let alpha = if hour >= 20.0 || hour < 5.0 {
        // Full night - stars fully visible
        1.0
    } else if hour >= 5.0 && hour < 6.0 {
        // Sunrise - stars fading out quickly
        let t = (hour - 5.0) / 1.0; // 1 hour transition
        let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
        1.0 - smooth_t
    } else if hour >= 18.5 && hour < 20.0 {
        // Sunset - stars fading in
        let t = (hour - 18.5) / 1.5; // 1.5 hour transition
        let smooth_t = t * t * (3.0 - 2.0 * t); // Smoothstep
        smooth_t
    } else {
        // Daytime - no stars
        0.0
    };

    // Update material alpha
    for material_handle in night_skybox_query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
        }
    }
}

/// Handle time control input (for testing)
pub fn handle_time_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut time_of_day: ResMut<TimeOfDay>,
) {
    // T key: toggle time pause
    if keyboard.just_pressed(KeyCode::KeyT) {
        time_of_day.paused = !time_of_day.paused;
    }

    // [ and ] keys: slow down / speed up time
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        time_of_day.speed = (time_of_day.speed / 2.0).max(0.125);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        time_of_day.speed = (time_of_day.speed * 2.0).min(64.0);
    }

    // , and . keys: jump time backward/forward by 1 hour
    if keyboard.just_pressed(KeyCode::Comma) {
        time_of_day.time = (time_of_day.time - 1.0 / 24.0 + 1.0) % 1.0;
    }
    if keyboard.just_pressed(KeyCode::Period) {
        time_of_day.time = (time_of_day.time + 1.0 / 24.0) % 1.0;
    }
}
