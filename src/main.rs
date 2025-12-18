mod assets;
mod blocks;
mod components;
mod mods;
mod rendering;
mod resources;
mod systems;
mod ui;
mod world;

use bevy::prelude::*;
use bevy::render::settings::{Backends, RenderCreation, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::pbr::{ExtendedMaterial, MaterialPlugin, StandardMaterial};
use bevy::text::{TextColor, TextFont};

use components::*;
use mods::ModPlugin;
use rendering::*;
use resources::{GameState, ChunkBorderState, PlayerInventory, PlayerStats};
use assets::IconsTextureHandle;
use systems::*;
use ui::{setup_pause_menu, setup_hotbar, setup_survival_bars};
use blocks::BlockRegistry;
use world::{setup_terrain, spawn_chunks_around_player, process_chunk_tasks, get_spawn_height};

// Import Crosshair component
use components::Crosshair;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "VoxelCraft".to_string(),
                        resolution: (1280.0, 720.0).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        // Force Vulkan backend for multi-platform compatibility
                        // Vulkan works on Windows, Linux, macOS (via MoltenVK), and Android
                        backends: Some(Backends::VULKAN),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(MaterialPlugin::<VoxelExtendedMaterial>::default())
        .add_plugins(MaterialPlugin::<rendering::terrain_material::TerrainMaterial>::default())
        .add_plugins(ModPlugin)
        .init_resource::<GameState>()
        .init_resource::<ChunkBorderState>()
        .init_resource::<rendering::IsometricIconCache>()
        .init_resource::<TimeOfDay>()
        .init_resource::<SkyLightLevel>()
        .init_resource::<systems::LightingOverlayState>()
        .init_resource::<PlayerStats>()
        .init_resource::<IconsTextureHandle>()
        .add_systems(Startup, (setup, setup_cursor_grab, setup_pause_menu, setup_hotbar, setup_survival_bars, setup_terrain, init_inventory).chain())
        // Input systems
        .add_systems(Update, (
            handle_window_focus,
            toggle_pause_menu,
            toggle_chunk_borders,
            toggle_debug_overlay,
            toggle_ui_visibility,
            toggle_creative_mode,
            take_screenshot,
            handle_time_controls,
            systems::toggle_lighting_overlay,
        ))
        // Gameplay systems
        .add_systems(Update, (
            spawn_chunks_around_player.run_if(|state: Res<GameState>| !state.paused),
            process_chunk_tasks,
            camera_movement_controls.run_if(|state: Res<GameState>| !state.paused),
            camera_look_controls.run_if(|state: Res<GameState>| !state.paused),
            hotbar_mouse_wheel_selection.run_if(|state: Res<GameState>| !state.paused),
            hotbar_number_key_selection.run_if(|state: Res<GameState>| !state.paused),
            block_interaction.run_if(|state: Res<GameState>| !state.paused),
            voxel_click_detection.run_if(|state: Res<GameState>| !state.paused),
            update_torch_light,
            follow_player_with_torch_light,
        ))
        // Remeshing must run after block interaction to see the updated chunk data
        .add_systems(PostUpdate, remesh_modified_chunks)
        // UI update systems
        .add_systems(Update, (
            update_hotbar_selection,
            update_hotbar_icons,
            update_hotbar_visibility,
            update_crosshair_visibility,
            update_debug_visibility,
            update_debug_visibility_on_ui_toggle,
            update_survival_bars_visibility,
            update_health_display,
            update_hunger_display,
            update_debug_overlay,
            update_click_text_timer,
            update_pause_menu_visibility,
            handle_pause_menu_buttons,
        ))
        // Day/night cycle systems
        .add_systems(Update, (
            update_time_of_day,
            update_sky_light_level,
            systems::update_stale_chunk_lighting,
            update_sun_transform,
            systems::update_night_skybox_alpha,
        ))
        // Rendering systems
        .add_systems(Update, (
            update_sky_position,
            update_voxel_material,
            update_chunk_borders,
            update_block_highlight,
            systems::detect_chunk_changes,
            systems::update_lighting_overlay,
        ))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VoxelExtendedMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Create daytime skybox (blue gradient)
    let skybox_texture = create_skybox_texture(&mut images);
    let skybox_material = standard_materials.add(StandardMaterial {
        base_color_texture: Some(skybox_texture),
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Create night skybox with stars
    let night_sky_texture = create_night_sky_texture(&mut images);
    let night_skybox_material = standard_materials.add(StandardMaterial {
        base_color_texture: Some(night_sky_texture),
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.0), // Start invisible
        unlit: true,
        cull_mode: None,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Create large spheres for skyboxes (inverted normals)
    let skybox_mesh = meshes.add(Sphere::new(500.0).mesh().uv(32, 18));
    let night_skybox_mesh = meshes.add(Sphere::new(499.0).mesh().uv(64, 32)); // Slightly smaller to render inside

    // Daytime skybox
    commands.spawn((
        Mesh3d(skybox_mesh),
        MeshMaterial3d(skybox_material),
        Transform::from_xyz(0.0, 40.0, 5.0).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
        Skybox,
    ));

    // Night skybox (stars)
    commands.spawn((
        Mesh3d(night_skybox_mesh),
        MeshMaterial3d(night_skybox_material),
        Transform::from_xyz(0.0, 40.0, 5.0).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
        NightSkybox,
    ));

    // Create Minecraft-sized sun and moon
    // In Minecraft, sun/moon appear quite large - about 7-8 degrees apparent diameter
    // At 400 unit distance: radius = 400 * tan(4 deg) ≈ 28 units
    let celestial_radius = 28.0;
    let orbit_distance = 400.0;
    let sun_position = Vec3::new(0.0, 40.0 + orbit_distance, 5.0);
    let moon_position = Vec3::new(0.0, 40.0 - orbit_distance, 5.0);

    // Create sun mesh (use more segments for smoother appearance at larger size)
    let sun_mesh = meshes.add(Sphere::new(celestial_radius).mesh().uv(48, 24));

    // Create bright, emissive sun material
    let sun_material = standard_materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.98, 0.9),
        emissive: LinearRgba::new(10.0, 9.5, 8.0, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(sun_mesh),
        MeshMaterial3d(sun_material),
        Transform::from_translation(sun_position),
        Sun,
    ));

    // Create moon mesh (same apparent size as sun)
    let moon_mesh = meshes.add(Sphere::new(celestial_radius).mesh().uv(48, 24));

    // Create moon material - pale gray/white, softly emissive
    let moon_material = standard_materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 1.0),
        emissive: LinearRgba::new(1.0, 1.0, 1.2, 1.0),
        unlit: true,
        ..default()
    });

    commands.spawn((
        Mesh3d(moon_mesh),
        MeshMaterial3d(moon_material),
        Transform::from_translation(moon_position),
        Moon,
    ));

    // Add directional light for the sun (at noon position, directly overhead)
    // Light direction points downward from the sun
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.99, 0.95), // Bright white sunlight
            illuminance: 80000.0, // Bright noon sun
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
    ));

    // Add ambient light for general scene illumination (bright noon sun)
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.95, 1.0),
        brightness: 2000.0,
    });

    // Set clear color for the window background
    commands.insert_resource(ClearColor(Color::srgb(0.5, 0.7, 0.95)));

    // Spawn camera with first-person controller and bloom effect
    // Calculate spawn height based on actual terrain at spawn position
    let spawn_x = 0;
    let spawn_z = 0;
    let spawn_y = get_spawn_height(spawn_x, spawn_z, 42); // seed 42 matches ChunkManager default

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true, // Enable HDR for bloom
            ..default()
        },
        Transform::from_xyz(spawn_x as f32, spawn_y, spawn_z as f32),
        CameraController::default(),
        Tonemapping::TonyMcMapface, // Good tonemapping for outdoor scenes
        // Bloom disabled - causes hazy/foggy appearance
        // Bloom {
        //     intensity: 0.3,
        //     low_frequency_boost: 0.7,
        //     low_frequency_boost_curvature: 0.95,
        //     high_pass_frequency: 1.0,
        //     composite_mode: bevy::core_pipeline::bloom::BloomCompositeMode::Additive,
        //     ..default()
        // },
    ));

    // UI text for click feedback (initially hidden)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 40.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        ClickText {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        },
    ));

    // Crosshair cursor (centered on screen)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(20.0),
            height: Val::Px(2.0),
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-10.0),
                top: Val::Px(-1.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Crosshair,
    ));

    // Crosshair vertical line
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(2.0),
            height: Val::Px(20.0),
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            margin: UiRect {
                left: Val::Px(-1.0),
                top: Val::Px(-10.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Crosshair,
    ));

    // Debug overlay (visible by default)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugOverlay { visible: true },
    ));
}

/// Initialize player inventory with hardcoded blocks
/// Must run after BlockRegistry is initialized (via ModPlugin)
fn init_inventory(
    mut commands: Commands,
    block_registry: Res<BlockRegistry>,
) {
    // Get block IDs for the hotbar
    // Position 1 (index 0): Bedrock
    // Position 2 (index 1): Stone
    // Position 3 (index 2): Dirt
    // Position 4 (index 3): Grass
    let bedrock = block_registry.get_id("core:bedrock")
        .expect("core:bedrock not found in registry");
    let stone = block_registry.get_id("core:stone")
        .expect("core:stone not found in registry");
    let dirt = block_registry.get_id("core:dirt")
        .expect("core:dirt not found in registry");
    let grass = block_registry.get_id("core:grass")
        .expect("core:grass not found in registry");

    let inventory = PlayerInventory::new_with_blocks(bedrock, stone, dirt, grass);
    commands.insert_resource(inventory);
}
