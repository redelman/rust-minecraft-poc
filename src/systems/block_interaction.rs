use bevy::prelude::*;
use bevy::ecs::system::ParamSet;
use bevy::input::mouse::MouseButton;
use bevy::render::primitives::Aabb;
use std::collections::HashMap;
use crate::components::CameraController;
use crate::resources::{ChunkManager, PlayerInventory, GameState};
use crate::world::{Chunk, ChunkCoord, CHUNK_SIZE, MAX_LIGHT_LEVEL};
use crate::systems::{SkyLightLevel, ChunkSkyLight};
use crate::blocks::{BlockId, BlockRegistry};
use crate::rendering::terrain_material::TerrainMaterial;

/// Marker component for chunks that need to be remeshed
#[derive(Component)]
pub struct NeedsRemesh;

/// Mark neighboring chunks for remeshing if the block is at a chunk boundary
fn mark_neighbor_chunks_for_remesh(
    commands: &mut Commands,
    chunk_coord: ChunkCoord,
    local_pos: UVec3,
    chunk_manager: &ChunkManager,
) {
    // Check if block is at chunk boundaries and mark neighbors for remesh
    if local_pos.x == 0 {
        // At west edge - mark west neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x - 1, chunk_coord.y, chunk_coord.z)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    } else if local_pos.x == (CHUNK_SIZE - 1) as u32 {
        // At east edge - mark east neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x + 1, chunk_coord.y, chunk_coord.z)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    }

    if local_pos.y == 0 {
        // At bottom edge - mark bottom neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x, chunk_coord.y - 1, chunk_coord.z)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    } else if local_pos.y == (CHUNK_SIZE - 1) as u32 {
        // At top edge - mark top neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x, chunk_coord.y + 1, chunk_coord.z)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    }

    if local_pos.z == 0 {
        // At north edge - mark north neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x, chunk_coord.y, chunk_coord.z - 1)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    } else if local_pos.z == (CHUNK_SIZE - 1) as u32 {
        // At south edge - mark south neighbor
        if let Some(&neighbor) = chunk_manager.loaded_chunks.get(&ChunkCoord::new(chunk_coord.x, chunk_coord.y, chunk_coord.z + 1)) {
            commands.entity(neighbor).insert(NeedsRemesh);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RaycastHit {
    pub block_pos: IVec3,     // Position of the hit block
    pub face_normal: IVec3,   // Normal of the face that was hit
    pub chunk_coord: ChunkCoord,
    pub local_pos: UVec3,     // Position within the chunk (0-15)
}

/// Raycast from camera to find which block is being looked at
/// Returns the hit block and which face was hit
/// This version works with a generic query that can be either &Chunk or &mut Chunk
fn raycast_block_impl<T>(
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    chunk_manager: &ChunkManager,
    get_chunk: impl Fn(Entity) -> Option<T>,
    read_block: impl Fn(&T, usize, usize, usize) -> BlockId,
) -> Option<RaycastHit>
where
    T: std::ops::Deref<Target = Chunk>,
{
    let step = 0.05; // Smaller steps for better accuracy
    let mut current = origin;
    let direction = direction.normalize();

    for _ in 0..((max_distance / step) as i32) {
        current += direction * step;

        let block_pos = IVec3::new(
            current.x.floor() as i32,
            current.y.floor() as i32,
            current.z.floor() as i32,
        );

        let chunk_coord = ChunkCoord::from_world_pos(current);

        if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&chunk_coord) {
            if let Some(chunk) = get_chunk(chunk_entity) {
                // Convert world position to local chunk position
                let local_x = (block_pos.x - chunk_coord.x * CHUNK_SIZE as i32) as usize;
                let local_y = (block_pos.y - chunk_coord.y * CHUNK_SIZE as i32) as usize;
                let local_z = (block_pos.z - chunk_coord.z * CHUNK_SIZE as i32) as usize;

                if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE && local_z < CHUNK_SIZE {
                    if !read_block(&chunk, local_x, local_y, local_z).is_air() {
                        // Hit a block! Calculate which face was hit based on the fractional part
                        // of the current position within the block (0.0 to 1.0)
                        let local_hit = Vec3::new(
                            current.x - block_pos.x as f32,
                            current.y - block_pos.y as f32,
                            current.z - block_pos.z as f32,
                        );

                        // Determine which face by finding which axis is closest to 0 or 1
                        let mut face_normal = IVec3::ZERO;
                        let mut min_dist = f32::MAX;

                        // Check all 6 faces
                        let faces = [
                            (IVec3::NEG_X, local_hit.x),           // Left face (x=0)
                            (IVec3::X, 1.0 - local_hit.x),         // Right face (x=1)
                            (IVec3::NEG_Y, local_hit.y),           // Bottom face (y=0)
                            (IVec3::Y, 1.0 - local_hit.y),         // Top face (y=1)
                            (IVec3::NEG_Z, local_hit.z),           // Back face (z=0)
                            (IVec3::Z, 1.0 - local_hit.z),         // Front face (z=1)
                        ];

                        for (normal, dist) in faces {
                            if dist < min_dist {
                                min_dist = dist;
                                face_normal = normal;
                            }
                        }

                        return Some(RaycastHit {
                            block_pos,
                            face_normal,
                            chunk_coord,
                            local_pos: UVec3::new(local_x as u32, local_y as u32, local_z as u32),
                        });
                    }
                }
            }
        }
    }

    None
}

/// System to handle block placement (right-click) and destruction (left-click)
pub fn block_interaction(
    mut commands: Commands,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query: Query<&Transform, With<CameraController>>,
    chunk_manager: Res<ChunkManager>,
    mut chunks_query: Query<&mut Chunk>,
    inventory: Res<PlayerInventory>,
    game_state: Res<GameState>,
    block_registry: Res<BlockRegistry>,
) {
    // Don't allow interaction when paused
    if game_state.paused {
        return;
    }

    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let ray_origin = camera_transform.translation;
    let ray_direction = camera_transform.forward();
    let max_distance = 8.0;

    // Perform raycast using the mutable query (but only reading)
    let hit = raycast_block_impl(
        ray_origin,
        *ray_direction,
        max_distance,
        &chunk_manager,
        |entity| chunks_query.get(entity).ok(),
        |chunk, x, y, z| chunk.get_block(x, y, z),
    );

    // Handle block destruction (left-click)
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(ref hit) = hit {
            // Get the chunk entity and modify the block
            if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&hit.chunk_coord) {
                if let Ok(mut chunk) = chunks_query.get_mut(chunk_entity) {
                    let current_block = chunk.get_block(
                        hit.local_pos.x as usize,
                        hit.local_pos.y as usize,
                        hit.local_pos.z as usize
                    );

                    // Check if block is destructible
                    let bedrock_id = block_registry.get_id("core:bedrock");
                    let is_bedrock = bedrock_id.map_or(false, |id| id == current_block);
                    let can_destroy = game_state.creative_mode || !is_bedrock;

                    if can_destroy {
                        chunk.set_block(
                            hit.local_pos.x as usize,
                            hit.local_pos.y as usize,
                            hit.local_pos.z as usize,
                            BlockId::AIR
                        );
                        // Recalculate skylight for this chunk
                        chunk.calculate_skylight();
                        // Mark chunk for remeshing
                        commands.entity(chunk_entity).insert(NeedsRemesh);

                        // Mark neighbor chunks if block is at boundary
                        mark_neighbor_chunks_for_remesh(&mut commands, hit.chunk_coord, hit.local_pos, &chunk_manager);

                        if let Some(block_type) = block_registry.get_block(current_block) {
                            info!("Destroyed {} at {:?}", block_type.properties.id, hit.block_pos);
                        }
                    } else {
                        info!("Cannot destroy bedrock in survival mode!");
                    }
                }
            }
        }
    }

    // Handle block placement (right-click)
    if mouse_button.just_pressed(MouseButton::Right) {
        if let Some(selected_block) = inventory.get_selected_block() {
            if let Some(ref hit) = hit {
                // Calculate placement position (adjacent to the hit face)
                let placement_pos = hit.block_pos + hit.face_normal;

                // Check if player would be placing block inside themselves
                // Player's feet are at camera Y - 1.6, head at camera Y
                // Player occupies roughly a 0.6x1.8x0.6 box
                let player_feet = ray_origin.y - 1.6;
                let player_head = ray_origin.y;
                let player_x = ray_origin.x;
                let player_z = ray_origin.z;

                // Check if placement would intersect player (simple box check)
                let would_intersect_player =
                    placement_pos.y as f32 <= player_head &&
                    (placement_pos.y as f32 + 1.0) >= player_feet &&
                    (placement_pos.x as f32 - player_x).abs() < 0.8 &&
                    (placement_pos.z as f32 - player_z).abs() < 0.8;

                if would_intersect_player {
                    info!("Cannot place block inside player!");
                    return;
                }

                // Find which chunk the new block should go in
                let placement_chunk_coord = ChunkCoord::from_world_pos(Vec3::new(
                    placement_pos.x as f32 + 0.5,
                    placement_pos.y as f32 + 0.5,
                    placement_pos.z as f32 + 0.5,
                ));

                if let Some(&chunk_entity) = chunk_manager.loaded_chunks.get(&placement_chunk_coord) {
                    if let Ok(mut chunk) = chunks_query.get_mut(chunk_entity) {
                        // Convert world position to local chunk position
                        let local_x = (placement_pos.x - placement_chunk_coord.x * CHUNK_SIZE as i32) as usize;
                        let local_y = (placement_pos.y - placement_chunk_coord.y * CHUNK_SIZE as i32) as usize;
                        let local_z = (placement_pos.z - placement_chunk_coord.z * CHUNK_SIZE as i32) as usize;

                        if local_x < CHUNK_SIZE && local_y < CHUNK_SIZE && local_z < CHUNK_SIZE {
                            // Only place if the target position is air
                            if chunk.get_block(local_x, local_y, local_z).is_air() {
                                chunk.set_block(local_x, local_y, local_z, selected_block);
                                // Recalculate skylight for this chunk
                                chunk.calculate_skylight();
                                // Mark chunk for remeshing
                                commands.entity(chunk_entity).insert(NeedsRemesh);

                                // Mark neighbor chunks if block is at boundary
                                let local_pos = UVec3::new(local_x as u32, local_y as u32, local_z as u32);
                                mark_neighbor_chunks_for_remesh(&mut commands, placement_chunk_coord, local_pos, &chunk_manager);

                                info!("Placed block {:?} at {:?}", selected_block, placement_pos);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Cached boundary light data for a chunk (6 faces worth of light values)
#[derive(Clone)]
struct CachedLightData {
    /// Light values at each face boundary
    /// Key: neighbor direction as (dx, dy, dz), Value: 2D array of light levels
    /// For X boundaries: [CHUNK_SIZE][CHUNK_SIZE] indexed by [y][z]
    /// For Y boundaries: [CHUNK_SIZE][CHUNK_SIZE] indexed by [x][z]
    /// For Z boundaries: [CHUNK_SIZE][CHUNK_SIZE] indexed by [x][y]
    light_levels: Vec<u8>,
    blocks: Vec<BlockId>,
}

impl CachedLightData {
    fn from_chunk(chunk: &Chunk) -> Self {
        Self {
            light_levels: chunk.light_levels.clone(),
            blocks: chunk.blocks.clone(),
        }
    }

    fn get_light(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return 15; // Full light outside
        }
        let idx = x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE;
        self.light_levels[idx]
    }

    fn get_block(&self, x: usize, y: usize, z: usize) -> BlockId {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return BlockId::AIR;
        }
        let idx = x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE;
        self.blocks[idx]
    }
}

/// Create a mesh for a chunk using cached neighbor data
/// This avoids query conflicts by using pre-cached block data for neighbor lookups
fn create_chunk_mesh_with_cache(
    chunk: &Chunk,
    block_registry: &BlockRegistry,
    chunk_coord: ChunkCoord,
    cache: &HashMap<ChunkCoord, CachedLightData>,
    sky_light_level: u8,
) -> Option<Mesh> {
    use crate::world::mesh_gen::create_chunk_mesh_with_cached_neighbors;

    // Build neighbor block getter from cache
    let get_neighbor_block = |dx: i32, dy: i32, dz: i32, x: usize, y: usize, z: usize| -> BlockId {
        let neighbor_coord = ChunkCoord::new(chunk_coord.x + dx, chunk_coord.y + dy, chunk_coord.z + dz);
        cache.get(&neighbor_coord)
            .map(|c| c.get_block(x, y, z))
            .unwrap_or(BlockId::AIR) // If no neighbor, assume air (render face)
    };

    // Build neighbor light getter from cache
    let get_neighbor_light = |dx: i32, dy: i32, dz: i32, x: usize, y: usize, z: usize| -> u8 {
        let neighbor_coord = ChunkCoord::new(chunk_coord.x + dx, chunk_coord.y + dy, chunk_coord.z + dz);
        cache.get(&neighbor_coord)
            .map(|c| c.get_light(x, y, z))
            .unwrap_or(0) // If no neighbor in cache, assume dark
    };

    create_chunk_mesh_with_cached_neighbors(chunk, block_registry, sky_light_level, get_neighbor_block, get_neighbor_light)
}

/// System to remesh chunks that have been modified
/// Uses a three-phase approach:
/// 1. Cache all chunk data (light and blocks) so we can read from ALL chunks
/// 2. Recalculate lighting using cached neighbor data
/// 3. Regenerate meshes using updated lighting
pub fn remesh_modified_chunks(
    mut commands: Commands,
    mut chunk_sets: ParamSet<(
        Query<(Entity, &mut Chunk, Option<&Mesh3d>), With<NeedsRemesh>>,
        Query<&Chunk>,  // All chunks for reading neighbor data
    )>,
    chunk_manager: Res<ChunkManager>,
    block_registry: Res<BlockRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_manager: Res<crate::assets::AssetManager>,
    mut texture_handle: Local<Option<Handle<Image>>>,
    sky_light_level: Res<SkyLightLevel>,
) {
    // Get texture atlas from AssetManager for the core mod
    if texture_handle.is_none() {
        *texture_handle = asset_manager.get_mod_texture_atlas("core");
    }

    // Limit chunks processed per frame to avoid stuttering during day/night transitions
    // Process more chunks when there are fewer to avoid visual lag
    const MAX_CHUNKS_PER_FRAME: usize = 32;

    // Phase 1: Collect entities and cache ALL chunk data using all_chunks query
    // This must happen before we access the mutable query
    let mut entities_to_process: Vec<(Entity, ChunkCoord)>;
    let mut chunk_cache: HashMap<ChunkCoord, CachedLightData> = HashMap::new();

    {
        // First pass: use all_chunks to read everything we need
        let all_chunks = chunk_sets.p1();

        // Collect entities to process by checking which ones have NeedsRemesh
        // We need to iterate all chunks and filter
        entities_to_process = chunk_manager.loaded_chunks.iter()
            .filter_map(|(coord, &entity)| {
                // Check if this entity needs remesh by trying to get it
                // We'll verify with the mutable query later
                all_chunks.get(entity).ok().map(|_| (entity, *coord))
            })
            .collect();
    }

    // Now check which ones actually have NeedsRemesh using the mutable query
    {
        let chunks_query = chunk_sets.p0();
        entities_to_process.retain(|(entity, _)| chunks_query.contains(*entity));
    }

    if entities_to_process.is_empty() {
        return;
    }

    // Sort by Y first (bottom to top), then by Z, then by X
    // This ensures light propagates correctly from sky down
    entities_to_process.sort_by(|a, b| {
        b.1.y.cmp(&a.1.y) // Reverse Y: top chunks first (they have skylight)
            .then(a.1.z.cmp(&b.1.z))
            .then(a.1.x.cmp(&b.1.x))
    });

    // Limit to MAX_CHUNKS_PER_FRAME to spread work across frames
    entities_to_process.truncate(MAX_CHUNKS_PER_FRAME);

    // Cache all chunk data we need using the all_chunks query
    {
        let all_chunks = chunk_sets.p1();

        // Cache chunks being processed
        for &(entity, coord) in &entities_to_process {
            if let Ok(chunk) = all_chunks.get(entity) {
                chunk_cache.insert(coord, CachedLightData::from_chunk(chunk));
            }
        }

        // Cache all neighbors (that aren't already cached)
        // Also cache chunks above for skylight column checking
        for &(_, coord) in &entities_to_process {
            let mut neighbor_coords = vec![
                ChunkCoord::new(coord.x - 1, coord.y, coord.z),
                ChunkCoord::new(coord.x + 1, coord.y, coord.z),
                ChunkCoord::new(coord.x, coord.y - 1, coord.z),
                ChunkCoord::new(coord.x, coord.y + 1, coord.z),
                ChunkCoord::new(coord.x, coord.y, coord.z - 1),
                ChunkCoord::new(coord.x, coord.y, coord.z + 1),
            ];

            // Also cache chunks above (for skylight column checking)
            for dy in 2..=8 {
                neighbor_coords.push(ChunkCoord::new(coord.x, coord.y + dy, coord.z));
            }

            for neighbor_coord in neighbor_coords {
                if chunk_cache.contains_key(&neighbor_coord) {
                    continue;
                }
                if let Some(&neighbor_entity) = chunk_manager.loaded_chunks.get(&neighbor_coord) {
                    if let Ok(neighbor_chunk) = all_chunks.get(neighbor_entity) {
                        chunk_cache.insert(neighbor_coord, CachedLightData::from_chunk(neighbor_chunk));
                    }
                }
            }
        }
    }

    // Phase 2: Recalculate lighting for all NeedsRemesh chunks
    // First pass: reset skylight columns (direct sunlight from above)
    // Subsequent passes: only propagate from neighbors (don't reset)
    const LIGHT_PROPAGATION_PASSES: usize = 4;

    for pass in 0..LIGHT_PROPAGATION_PASSES {
        // Process all chunks using current cache
        {
            let mut chunks_query = chunk_sets.p0();
            for &(entity, coord) in &entities_to_process {
                let Ok((_, mut chunk, _)) = chunks_query.get_mut(entity) else {
                    continue;
                };

                if pass == 0 {
                    // First pass: full recalculation (reset skylight columns, then flood fill)
                    calculate_skylight_with_cache(&mut chunk, coord, &chunk_cache);
                } else {
                    // Subsequent passes: only flood fill from neighbors (don't reset columns)
                    propagate_light_from_cache(&mut chunk, coord, &chunk_cache);
                }
            }
        }

        // After processing all chunks, update the cache with new light values
        // This allows the next pass to see updated light from neighboring NeedsRemesh chunks
        if pass < LIGHT_PROPAGATION_PASSES - 1 {
            let all_chunks = chunk_sets.p1();
            for &(entity, coord) in &entities_to_process {
                if let Ok(chunk) = all_chunks.get(entity) {
                    chunk_cache.insert(coord, CachedLightData::from_chunk(chunk));
                }
            }
        }
    }

    // Phase 3: Generate meshes using updated lighting
    // Use cached neighbor data for mesh generation to avoid query conflicts
    {
        let chunks_query = chunk_sets.p0();
        for &(entity, coord) in &entities_to_process {
            let Ok((_, chunk, mesh_3d_opt)) = chunks_query.get(entity) else {
                continue;
            };

            // Build mesh using cache for neighbor data
            // The cache now has all the block data we need
            let mesh_result = create_chunk_mesh_with_cache(&chunk, &block_registry, coord, &chunk_cache, sky_light_level.level);

            let has_mesh = mesh_3d_opt.is_some();
            let mesh_handle_clone = mesh_3d_opt.map(|m| m.0.clone());

            if let Some(new_mesh) = mesh_result {
                // We need to process this outside the query borrow
                // Store info and process after
                if has_mesh {
                    if let Some(ref mesh_handle) = mesh_handle_clone {
                        if let Some(mesh_asset) = meshes.get_mut(mesh_handle) {
                            *mesh_asset = new_mesh;
                        }
                    }
                    commands.entity(entity).remove::<Aabb>();
                } else {
                    let mesh_handle = meshes.add(new_mesh);
                    let material_handle = if let Some(ref tex) = *texture_handle {
                        materials.add(TerrainMaterial::new(tex.clone()))
                    } else {
                        materials.add(TerrainMaterial::new(Handle::default()))
                    };

                    commands.entity(entity).insert((
                        Mesh3d(mesh_handle),
                        MeshMaterial3d(material_handle),
                        crate::world::TerrainChunk,
                    ));
                }
            } else {
                // No visible faces - remove mesh components entirely
                // This avoids wgpu validation errors from empty meshes without proper attributes
                if has_mesh {
                    commands.entity(entity).remove::<Mesh3d>();
                    commands.entity(entity).remove::<MeshMaterial3d<TerrainMaterial>>();
                    commands.entity(entity).remove::<Aabb>();
                }
            }

            // Remove NeedsRemesh marker and record the sky light level we rendered with
            commands.entity(entity).remove::<NeedsRemesh>();
            commands.entity(entity).insert(ChunkSkyLight { level: sky_light_level.level });
        }
    }
}

/// Calculate skylight for a chunk using cached neighbor data
/// This allows reading from neighbors that might also be marked NeedsRemesh
fn calculate_skylight_with_cache(
    chunk: &mut Chunk,
    coord: ChunkCoord,
    cache: &HashMap<ChunkCoord, CachedLightData>,
) {
    // First pass: propagate direct skylight from top down
    // To determine if a column has sky access, trace UP through chunks above
    // checking for solid blocks (not light values, which may be stale)
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            // Check if this column has direct sky access by tracing upward
            let in_shadow = is_column_shadowed(x, z, coord, cache);

            // Propagate from top to bottom
            let mut column_shadow = in_shadow;
            for y in (0..CHUNK_SIZE).rev() {
                let block = chunk.get_block(x, y, z);

                if block.is_air() {
                    if !column_shadow {
                        // Direct skylight - full brightness
                        chunk.set_light(x, y, z, MAX_LIGHT_LEVEL);
                    } else {
                        // In shadow - start at 0, will be filled by flood fill
                        chunk.set_light(x, y, z, 0);
                    }
                } else {
                    // Solid block - blocks light, and everything below is in shadow
                    chunk.set_light(x, y, z, 0);
                    column_shadow = true;
                }
            }
        }
    }

    // Second pass: flood-fill light propagation
    propagate_light_from_cache(chunk, coord, cache);
}

/// Check if a column at (x, z) in the given chunk is shadowed by blocks above
/// Traces upward through cached chunks to find any solid blocks
fn is_column_shadowed(
    x: usize,
    z: usize,
    chunk_coord: ChunkCoord,
    cache: &HashMap<ChunkCoord, CachedLightData>,
) -> bool {
    // Start from the chunk above and trace upward
    let mut check_coord = ChunkCoord::new(chunk_coord.x, chunk_coord.y + 1, chunk_coord.z);

    // Check up to 8 chunks above (128 blocks) - beyond that assume open sky
    for _ in 0..8 {
        if let Some(above_cache) = cache.get(&check_coord) {
            // Check the entire column in this chunk for any solid blocks
            for y in 0..CHUNK_SIZE {
                if !above_cache.get_block(x, y, z).is_air() {
                    // Found a solid block above - we're in shadow
                    return true;
                }
            }
            // This chunk is all air in this column, check the next one up
            check_coord = ChunkCoord::new(check_coord.x, check_coord.y + 1, check_coord.z);
        } else {
            // No chunk above in cache - assume open sky
            return false;
        }
    }

    // Reached max height without finding solid block - open sky
    false
}

/// Propagate light from neighbors without resetting skylight columns
/// Used for subsequent passes after initial skylight calculation
fn propagate_light_from_cache(
    chunk: &mut Chunk,
    coord: ChunkCoord,
    cache: &HashMap<ChunkCoord, CachedLightData>,
) {
    let get_cached_light = |neighbor_coord: ChunkCoord, x: usize, y: usize, z: usize| -> u8 {
        cache.get(&neighbor_coord)
            .map(|c| c.get_light(x, y, z))
            // If neighbor not in cache, return 0 (dark)
            // This is rare since we cache all neighbors during remesh
            .unwrap_or(0)
    };

    let neg_x = ChunkCoord::new(coord.x - 1, coord.y, coord.z);
    let pos_x = ChunkCoord::new(coord.x + 1, coord.y, coord.z);
    let neg_y = ChunkCoord::new(coord.x, coord.y - 1, coord.z);
    let pos_y = ChunkCoord::new(coord.x, coord.y + 1, coord.z);
    let neg_z = ChunkCoord::new(coord.x, coord.y, coord.z - 1);
    let pos_z = ChunkCoord::new(coord.x, coord.y, coord.z + 1);

    for _iteration in 0..(CHUNK_SIZE * 2) {
        let mut any_change = false;

        // Forward pass
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if !chunk.get_block(x, y, z).is_air() {
                        continue;
                    }

                    let current_light = chunk.get_light(x, y, z);
                    let mut max_neighbor: u8 = 0;

                    // Check neighbors within this chunk
                    if x > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x - 1, y, z)); }
                    if x < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x + 1, y, z)); }
                    if y > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x, y - 1, z)); }
                    if y < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x, y + 1, z)); }
                    if z > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x, y, z - 1)); }
                    if z < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x, y, z + 1)); }

                    // Check cross-chunk neighbors using cached data
                    if x == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_x, CHUNK_SIZE - 1, y, z)); }
                    if x == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_x, 0, y, z)); }
                    if y == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_y, x, CHUNK_SIZE - 1, z)); }
                    if y == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_y, x, 0, z)); }
                    if z == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_z, x, y, CHUNK_SIZE - 1)); }
                    if z == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_z, x, y, 0)); }

                    let propagated = max_neighbor.saturating_sub(1);
                    if propagated > current_light {
                        chunk.set_light(x, y, z, propagated);
                        any_change = true;
                    }
                }
            }
        }

        // Backward pass for faster convergence
        for y in (0..CHUNK_SIZE).rev() {
            for z in (0..CHUNK_SIZE).rev() {
                for x in (0..CHUNK_SIZE).rev() {
                    if !chunk.get_block(x, y, z).is_air() {
                        continue;
                    }

                    let current_light = chunk.get_light(x, y, z);
                    let mut max_neighbor: u8 = 0;

                    if x > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x - 1, y, z)); }
                    if x < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x + 1, y, z)); }
                    if y > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x, y - 1, z)); }
                    if y < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x, y + 1, z)); }
                    if z > 0 { max_neighbor = max_neighbor.max(chunk.get_light(x, y, z - 1)); }
                    if z < CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(chunk.get_light(x, y, z + 1)); }

                    if x == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_x, CHUNK_SIZE - 1, y, z)); }
                    if x == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_x, 0, y, z)); }
                    if y == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_y, x, CHUNK_SIZE - 1, z)); }
                    if y == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_y, x, 0, z)); }
                    if z == 0 { max_neighbor = max_neighbor.max(get_cached_light(neg_z, x, y, CHUNK_SIZE - 1)); }
                    if z == CHUNK_SIZE - 1 { max_neighbor = max_neighbor.max(get_cached_light(pos_z, x, y, 0)); }

                    let propagated = max_neighbor.saturating_sub(1);
                    if propagated > current_light {
                        chunk.set_light(x, y, z, propagated);
                        any_change = true;
                    }
                }
            }
        }

        if !any_change {
            break;
        }
    }
}

