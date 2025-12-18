use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use noise::{NoiseFn, Simplex};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::chunk::{Chunk, ChunkCoord, CHUNK_SIZE, VIEW_DISTANCE, VIEW_DISTANCE_VERTICAL};
use super::mesh_gen::create_chunk_mesh;
use crate::assets::AssetManager;
use crate::blocks::{BlockRegistry, BlockId};
use crate::components::CameraController;
use crate::rendering::terrain_material::TerrainMaterial;

#[derive(Resource)]
pub struct ChunkManager {
    pub loaded_chunks: HashMap<ChunkCoord, Entity>,
    pub loading_chunks: HashSet<ChunkCoord>,
    pub world_seed: u32,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            loading_chunks: HashSet::new(),
            world_seed: 42,
        }
    }
}

#[derive(Component)]
pub struct ChunkTask(Task<(ChunkCoord, Chunk, Option<Mesh>)>);

/// Marker component for terrain chunk meshes
#[derive(Component)]
pub struct TerrainChunk;

pub fn setup_terrain(mut commands: Commands) {
    commands.init_resource::<ChunkManager>();
}

pub fn spawn_chunks_around_player(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    block_registry: Res<BlockRegistry>,
    camera_query: Query<&Transform, With<CameraController>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let player_chunk = ChunkCoord::from_world_pos(camera_transform.translation);
    let task_pool = AsyncComputeTaskPool::get();

    // Get all chunks in view distance (3D sphere)
    for dx in -VIEW_DISTANCE..=VIEW_DISTANCE {
        for dy in -VIEW_DISTANCE_VERTICAL..=VIEW_DISTANCE_VERTICAL {
            for dz in -VIEW_DISTANCE..=VIEW_DISTANCE {
                let chunk_coord = ChunkCoord::new(
                    player_chunk.x + dx,
                    player_chunk.y + dy,
                    player_chunk.z + dz
                );

                // Check if within spherical view distance
                if chunk_coord.distance_squared(&player_chunk) > VIEW_DISTANCE * VIEW_DISTANCE {
                    continue;
                }

                // Skip if already loaded or loading
                if chunk_manager.loaded_chunks.contains_key(&chunk_coord)
                    || chunk_manager.loading_chunks.contains(&chunk_coord)
                {
                    continue;
                }

                // Mark as loading
                chunk_manager.loading_chunks.insert(chunk_coord);

                // Spawn async task to generate chunk
                let seed = chunk_manager.world_seed;

                // Get block IDs we need for terrain generation
                let grass_id = block_registry.get_id("core:grass").unwrap_or(BlockId::AIR);
                let dirt_id = block_registry.get_id("core:dirt").unwrap_or(BlockId::AIR);
                let stone_id = block_registry.get_id("core:stone").unwrap_or(BlockId::AIR);
                let bedrock_id = block_registry.get_id("core:bedrock").unwrap_or(BlockId::AIR);

                // Share the registry with the async task by wrapping in Arc
                // This eliminates duplicate registrations
                let registry_arc = Arc::new(block_registry.clone());

                let task = task_pool.spawn(async move {
                    generate_chunk(chunk_coord, seed, grass_id, dirt_id, stone_id, bedrock_id, &registry_arc)
                });

                commands.spawn(ChunkTask(task));
            }
        }
    }
}

pub fn process_chunk_tasks(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    mut chunk_tasks: Query<(Entity, &mut ChunkTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_manager: Res<AssetManager>,
    mut texture_handle: Local<Option<Handle<Image>>>,
) {
    // Get texture atlas from AssetManager for the core mod
    if texture_handle.is_none() {
        *texture_handle = asset_manager.get_mod_texture_atlas("core");
    }

    for (entity, mut task) in chunk_tasks.iter_mut() {
        if let Some((coord, chunk, mesh_opt)) = future::block_on(future::poll_once(&mut task.0)) {
            // Remove from loading set
            chunk_manager.loading_chunks.remove(&coord);

            // Spawn chunk entity (with or without mesh)
            let mut chunk_entity_builder = commands.spawn((
                Transform::from_translation(coord.to_world_pos()),
                chunk,
            ));

            // Only add mesh components if we have a mesh
            if let Some(mesh) = mesh_opt {
                let mesh_handle = meshes.add(mesh);
                // Use custom TerrainMaterial with overlay support
                let material_handle = if let Some(ref tex) = *texture_handle {
                    materials.add(TerrainMaterial::new(tex.clone()))
                } else {
                    // Fallback - shouldn't happen
                    materials.add(TerrainMaterial::new(Handle::default()))
                };

                chunk_entity_builder.insert((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(material_handle),
                    TerrainChunk,
                ));
            }

            let chunk_entity = chunk_entity_builder.id();

            // Register in chunk manager
            chunk_manager.loaded_chunks.insert(coord, chunk_entity);

            // Mark this chunk for remeshing to incorporate neighbor data
            commands.entity(chunk_entity).insert(crate::systems::NeedsRemesh);

            // Mark neighboring chunks for remeshing too, since they now have a new neighbor
            // This ensures faces at chunk boundaries are rendered correctly
            let neighbor_coords = [
                ChunkCoord::new(coord.x - 1, coord.y, coord.z),
                ChunkCoord::new(coord.x + 1, coord.y, coord.z),
                ChunkCoord::new(coord.x, coord.y - 1, coord.z),
                ChunkCoord::new(coord.x, coord.y + 1, coord.z),
                ChunkCoord::new(coord.x, coord.y, coord.z - 1),
                ChunkCoord::new(coord.x, coord.y, coord.z + 1),
            ];

            for neighbor_coord in neighbor_coords.iter() {
                if let Some(&neighbor_entity) = chunk_manager.loaded_chunks.get(neighbor_coord) {
                    commands.entity(neighbor_entity).insert(crate::systems::NeedsRemesh);
                }
            }

            // Despawn the task
            commands.entity(entity).despawn();
        }
    }
}

/// Calculate a safe spawn height for the given world position
/// Returns the camera Y coordinate (player eye level) for spawning above terrain
pub fn get_spawn_height(world_x: i32, world_z: i32, seed: u32) -> f32 {
    let simplex = Simplex::new(seed);
    let terrain_height = get_terrain_height(&simplex, world_x, world_z);
    // terrain_height is the Y level of the top solid block (e.g., grass at Y=36)
    // Player feet at terrain_height + 1.5 (standing on top of surface block with clearance)
    // Camera at eye level = feet + 1.6
    // So camera Y = terrain_height + 1.5 + 1.6 = terrain_height + 3.1
    (terrain_height as f32) + 3.1
}

/// Generate terrain height using multi-octave Simplex noise (fractal Brownian motion)
/// This creates smooth, natural-looking terrain with both large-scale hills and small details
pub fn get_terrain_height(simplex: &Simplex, world_x: i32, world_z: i32) -> i32 {
    let x = world_x as f64;
    let z = world_z as f64;

    // Continental scale - very large, smooth features (mountains vs plains)
    // Scale: 256 blocks, amplitude: 20 blocks
    let continental = simplex.get([x / 256.0, z / 256.0]) * 20.0;

    // Regional scale - hills and valleys
    // Scale: 64 blocks, amplitude: 12 blocks
    let regional = simplex.get([x / 64.0, z / 64.0]) * 12.0;

    // Local detail - small bumps and dips
    // Scale: 32 blocks, amplitude: 6 blocks
    let local = simplex.get([x / 32.0, z / 32.0]) * 6.0;

    // Fine detail - very small variations
    // Scale: 16 blocks, amplitude: 2 blocks
    let detail = simplex.get([x / 16.0, z / 16.0]) * 2.0;

    // Micro detail - tiny surface variations
    // Scale: 8 blocks, amplitude: 1 block
    let micro = simplex.get([x / 8.0, z / 8.0]) * 1.0;

    // Combine all octaves
    // Base height is 32, total variation range is about ±41 blocks
    // This gives terrain heights roughly between -9 and 73
    let height = 32.0 + continental + regional + local + detail + micro;

    height as i32
}

fn generate_chunk(
    coord: ChunkCoord,
    seed: u32,
    grass_id: BlockId,
    dirt_id: BlockId,
    stone_id: BlockId,
    bedrock_id: BlockId,
    block_registry: &Arc<BlockRegistry>,
) -> (ChunkCoord, Chunk, Option<Mesh>) {
    let mut chunk = Chunk::new(coord);

    // Use Simplex noise for terrain generation
    // Simplex is better than Perlin: smoother gradients, no directional artifacts, faster
    let simplex = Simplex::new(seed);

    // Calculate world position of this chunk
    let chunk_world_x = coord.x * CHUNK_SIZE as i32;
    let chunk_world_y = coord.y * CHUNK_SIZE as i32;
    let chunk_world_z = coord.z * CHUNK_SIZE as i32;

    // Generate terrain for this chunk
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = chunk_world_x + x as i32;
            let world_z = chunk_world_z + z as i32;

            // Get terrain height at this x,z coordinate using continuous noise
            let terrain_height = get_terrain_height(&simplex, world_x, world_z);

            // Generate blocks for this column
            for y in 0..CHUNK_SIZE {
                let world_y = chunk_world_y + y as i32;

                // Determine block type based on depth from surface
                let depth_from_surface = terrain_height - world_y;

                let block_id = if world_y < 0 {
                    // Void below y=0
                    BlockId::AIR
                } else if world_y == 0 {
                    // Bedrock at y=0
                    bedrock_id
                } else if world_y > terrain_height {
                    // Air above terrain
                    BlockId::AIR
                } else if depth_from_surface == 0 {
                    // Surface block - grass
                    grass_id
                } else if depth_from_surface <= 3 {
                    // Top 3 blocks below surface - dirt
                    dirt_id
                } else {
                    // Everything else underground - stone
                    stone_id
                };

                chunk.set_block(x, y, z, block_id);
            }
        }
    }

    // Calculate skylight propagation for this chunk
    chunk.calculate_skylight();

    // Generate mesh for this chunk
    let mesh = create_chunk_mesh(&chunk, block_registry);

    (coord, chunk, mesh)
}
