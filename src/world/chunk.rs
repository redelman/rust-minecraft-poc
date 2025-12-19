use bevy::prelude::*;
use crate::blocks::BlockId;

// Chunk configuration - 16x16x16 cubic chunks
pub const CHUNK_SIZE: usize = 16;
pub const VIEW_DISTANCE: i32 = 10; // Render distance in chunks (horizontal) - 160 blocks
pub const VIEW_DISTANCE_VERTICAL: i32 = 5; // Render distance in chunks (vertical)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world_pos(pos: Vec3) -> Self {
        Self {
            x: (pos.x / CHUNK_SIZE as f32).floor() as i32,
            y: (pos.y / CHUNK_SIZE as f32).floor() as i32,
            z: (pos.z / CHUNK_SIZE as f32).floor() as i32,
        }
    }

    pub fn to_world_pos(&self) -> Vec3 {
        Vec3::new(
            self.x as f32 * CHUNK_SIZE as f32,
            self.y as f32 * CHUNK_SIZE as f32,
            self.z as f32 * CHUNK_SIZE as f32,
        )
    }

    pub fn distance_squared(&self, other: &ChunkCoord) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Manhattan distance (useful for chunk loading priority)
    #[allow(dead_code)]
    pub fn manhattan_distance(&self, other: &ChunkCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// Maximum light level (full sunlight)
pub const MAX_LIGHT_LEVEL: u8 = 15;

/// Boundary light values for a chunk (used to detect if neighbors need updating)
#[allow(dead_code)]
pub struct BoundaryLight {
    pub neg_x: [[u8; CHUNK_SIZE]; CHUNK_SIZE], // indexed by [y][z]
    pub pos_x: [[u8; CHUNK_SIZE]; CHUNK_SIZE],
    pub neg_y: [[u8; CHUNK_SIZE]; CHUNK_SIZE], // indexed by [x][z]
    pub pos_y: [[u8; CHUNK_SIZE]; CHUNK_SIZE],
    pub neg_z: [[u8; CHUNK_SIZE]; CHUNK_SIZE], // indexed by [x][y]
    pub pos_z: [[u8; CHUNK_SIZE]; CHUNK_SIZE],
}

#[allow(dead_code)]
impl BoundaryLight {
    /// Check if any boundary light value could propagate to the neighbor
    /// (i.e., our boundary value - 1 > neighbor's boundary value)
    pub fn would_update_neighbor(&self, face: ChunkFace, neighbor_boundary: &BoundaryLight) -> bool {
        match face {
            ChunkFace::NegX => {
                // Our neg_x face touches neighbor's pos_x face
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let our_light = self.neg_x[y][z];
                        let their_light = neighbor_boundary.pos_x[y][z];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
            ChunkFace::PosX => {
                for y in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let our_light = self.pos_x[y][z];
                        let their_light = neighbor_boundary.neg_x[y][z];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
            ChunkFace::NegY => {
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let our_light = self.neg_y[x][z];
                        let their_light = neighbor_boundary.pos_y[x][z];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
            ChunkFace::PosY => {
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let our_light = self.pos_y[x][z];
                        let their_light = neighbor_boundary.neg_y[x][z];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
            ChunkFace::NegZ => {
                for x in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        let our_light = self.neg_z[x][y];
                        let their_light = neighbor_boundary.pos_z[x][y];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
            ChunkFace::PosZ => {
                for x in 0..CHUNK_SIZE {
                    for y in 0..CHUNK_SIZE {
                        let our_light = self.pos_z[x][y];
                        let their_light = neighbor_boundary.neg_z[x][y];
                        if our_light > 1 && our_light.saturating_sub(1) > their_light {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Which face of a chunk
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ChunkFace {
    NegX, PosX, NegY, PosY, NegZ, PosZ,
}

/// Neighbor chunk light data for cross-chunk light propagation
pub struct NeighborLightData<'a> {
    pub neg_x: Option<&'a Chunk>,  // West
    pub pos_x: Option<&'a Chunk>,  // East
    pub neg_y: Option<&'a Chunk>,  // Down
    pub pos_y: Option<&'a Chunk>,  // Up
    pub neg_z: Option<&'a Chunk>,  // North
    pub pos_z: Option<&'a Chunk>,  // South
}

impl<'a> NeighborLightData<'a> {
    pub fn none() -> Self {
        Self {
            neg_x: None,
            pos_x: None,
            neg_y: None,
            pos_y: None,
            neg_z: None,
            pos_z: None,
        }
    }
}

#[derive(Component)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub blocks: Vec<BlockId>, // Block IDs for each position (16x16x16 = 4096 blocks)
    pub light_levels: Vec<u8>, // Light level (0-15) for each position
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            blocks: vec![BlockId::AIR; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
            light_levels: vec![MAX_LIGHT_LEVEL; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
        }
    }

    /// Convert 3D coordinates to 1D index
    /// Layout: x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
    #[inline]
    fn index(x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_SIZE + y * CHUNK_SIZE * CHUNK_SIZE
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockId {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return BlockId::AIR;
        }
        self.blocks[Self::index(x, y, z)]
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block_id: BlockId) {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            self.blocks[Self::index(x, y, z)] = block_id;
        }
    }

    pub fn get_light(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= CHUNK_SIZE || y >= CHUNK_SIZE || z >= CHUNK_SIZE {
            return MAX_LIGHT_LEVEL; // Assume full light outside chunk
        }
        self.light_levels[Self::index(x, y, z)]
    }

    pub fn set_light(&mut self, x: usize, y: usize, z: usize, level: u8) {
        if x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE {
            self.light_levels[Self::index(x, y, z)] = level.min(MAX_LIGHT_LEVEL);
        }
    }

    /// Calculate skylight for this chunk using flood-fill propagation
    /// Light propagates from sky downward, then spreads in all directions
    /// This version doesn't use neighbor chunks (for initial generation)
    pub fn calculate_skylight(&mut self) {
        self.calculate_skylight_with_neighbors(&NeighborLightData::none());
    }

    /// Calculate skylight with neighbor chunk data for cross-chunk propagation
    /// This allows light to flow from one chunk into another through tunnels/caves
    pub fn calculate_skylight_with_neighbors(&mut self, neighbors: &NeighborLightData) {
        // First pass: propagate direct skylight from top down
        // Only blocks with unobstructed sky access get full light
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let mut in_shadow = false;

                // Check if there's a block above in the neighbor chunk blocking skylight
                // We need to trace up through the entire column of the chunk above
                if let Some(above_chunk) = neighbors.pos_y {
                    // Check the light level at the bottom of the chunk above
                    // If it's not full brightness, this column is in shadow
                    let above_light = above_chunk.get_light(x, 0, z);
                    if above_light < MAX_LIGHT_LEVEL {
                        in_shadow = true;
                    }
                    // Also check if bottom block of above chunk is solid
                    if !above_chunk.get_block(x, 0, z).is_air() {
                        in_shadow = true;
                    }
                }

                // Propagate from top to bottom
                for y in (0..CHUNK_SIZE).rev() {
                    let block = self.get_block(x, y, z);

                    if block.is_air() {
                        if !in_shadow {
                            // Direct skylight - full brightness
                            self.set_light(x, y, z, MAX_LIGHT_LEVEL);
                        } else {
                            // In shadow - start at 0, will be filled by flood fill
                            self.set_light(x, y, z, 0);
                        }
                    } else {
                        // Solid block - blocks light, and is in shadow itself
                        self.set_light(x, y, z, 0);
                        in_shadow = true;
                    }
                }
            }
        }

        // Second pass: flood-fill light propagation using a proper BFS-like approach
        // We iterate until no changes occur, light spreads from bright to dark areas
        // Use more iterations to ensure light can propagate across the entire chunk
        for _iteration in 0..(CHUNK_SIZE * 2) {
            let mut any_change = false;

            // Forward pass (increasing coordinates)
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        if !self.get_block(x, y, z).is_air() {
                            continue;
                        }
                        if self.propagate_light_from_neighbors_ext(x, y, z, neighbors) {
                            any_change = true;
                        }
                    }
                }
            }

            // Backward pass (decreasing coordinates) for faster convergence
            for y in (0..CHUNK_SIZE).rev() {
                for z in (0..CHUNK_SIZE).rev() {
                    for x in (0..CHUNK_SIZE).rev() {
                        if !self.get_block(x, y, z).is_air() {
                            continue;
                        }
                        if self.propagate_light_from_neighbors_ext(x, y, z, neighbors) {
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

    /// Get the light values at the boundary faces of this chunk
    /// Returns (neg_x, pos_x, neg_y, pos_y, neg_z, pos_z) arrays
    #[allow(dead_code)]
    pub fn get_boundary_light(&self) -> BoundaryLight {
        let mut neg_x = [[0u8; CHUNK_SIZE]; CHUNK_SIZE]; // y, z
        let mut pos_x = [[0u8; CHUNK_SIZE]; CHUNK_SIZE];
        let mut neg_y = [[0u8; CHUNK_SIZE]; CHUNK_SIZE]; // x, z
        let mut pos_y = [[0u8; CHUNK_SIZE]; CHUNK_SIZE];
        let mut neg_z = [[0u8; CHUNK_SIZE]; CHUNK_SIZE]; // x, y
        let mut pos_z = [[0u8; CHUNK_SIZE]; CHUNK_SIZE];

        for a in 0..CHUNK_SIZE {
            for b in 0..CHUNK_SIZE {
                neg_x[a][b] = self.get_light(0, a, b);
                pos_x[a][b] = self.get_light(CHUNK_SIZE - 1, a, b);
                neg_y[a][b] = self.get_light(a, 0, b);
                pos_y[a][b] = self.get_light(a, CHUNK_SIZE - 1, b);
                neg_z[a][b] = self.get_light(a, b, 0);
                pos_z[a][b] = self.get_light(a, b, CHUNK_SIZE - 1);
            }
        }

        BoundaryLight { neg_x, pos_x, neg_y, pos_y, neg_z, pos_z }
    }

    /// Helper function: get light from this chunk or neighbor chunk if at boundary
    fn get_light_or_neighbor(&self, x: i32, y: i32, z: i32, neighbors: &NeighborLightData) -> u8 {
        // Within this chunk
        if x >= 0 && x < CHUNK_SIZE as i32 &&
           y >= 0 && y < CHUNK_SIZE as i32 &&
           z >= 0 && z < CHUNK_SIZE as i32 {
            return self.get_light(x as usize, y as usize, z as usize);
        }

        // Check neighbor chunks
        if x < 0 {
            if let Some(neighbor) = neighbors.neg_x {
                return neighbor.get_light(CHUNK_SIZE - 1, y as usize, z as usize);
            }
        } else if x >= CHUNK_SIZE as i32 {
            if let Some(neighbor) = neighbors.pos_x {
                return neighbor.get_light(0, y as usize, z as usize);
            }
        }

        if y < 0 {
            if let Some(neighbor) = neighbors.neg_y {
                return neighbor.get_light(x as usize, CHUNK_SIZE - 1, z as usize);
            }
        } else if y >= CHUNK_SIZE as i32 {
            if let Some(neighbor) = neighbors.pos_y {
                return neighbor.get_light(x as usize, 0, z as usize);
            }
        }

        if z < 0 {
            if let Some(neighbor) = neighbors.neg_z {
                return neighbor.get_light(x as usize, y as usize, CHUNK_SIZE - 1);
            }
        } else if z >= CHUNK_SIZE as i32 {
            if let Some(neighbor) = neighbors.pos_z {
                return neighbor.get_light(x as usize, y as usize, 0);
            }
        }

        // No neighbor available - assume no light from that direction
        0
    }

    /// Helper function: check neighbors (including cross-chunk) and propagate light
    /// Returns true if light was updated
    fn propagate_light_from_neighbors_ext(&mut self, x: usize, y: usize, z: usize, neighbors: &NeighborLightData) -> bool {
        let current_light = self.get_light(x, y, z);
        let mut max_neighbor_light: u8 = 0;

        let xi = x as i32;
        let yi = y as i32;
        let zi = z as i32;

        // Check all 6 neighbors (including cross-chunk)
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi - 1, yi, zi, neighbors));
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi + 1, yi, zi, neighbors));
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi, yi - 1, zi, neighbors));
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi, yi + 1, zi, neighbors));
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi, yi, zi - 1, neighbors));
        max_neighbor_light = max_neighbor_light.max(self.get_light_or_neighbor(xi, yi, zi + 1, neighbors));

        // Light propagates with -1 falloff
        let propagated = max_neighbor_light.saturating_sub(1);
        if propagated > current_light {
            self.set_light(x, y, z, propagated);
            return true;
        }
        false
    }

    /// Check if a block face should be rendered (neighbor is transparent/air)
    #[allow(dead_code)]
    pub fn should_render_face(&self, x: usize, y: usize, z: usize, dx: i32, dy: i32, dz: i32) -> bool {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        let nz = z as i32 + dz;

        // If neighbor is outside chunk, assume it should be rendered (will be handled by adjacent chunk)
        if nx < 0 || nx >= CHUNK_SIZE as i32 ||
           ny < 0 || ny >= CHUNK_SIZE as i32 ||
           nz < 0 || nz >= CHUNK_SIZE as i32 {
            return true;
        }

        // Render if neighbor is air
        self.get_block(nx as usize, ny as usize, nz as usize).is_air()
    }

    /// Get the height of the top non-air block at this x/z position (for camera collision)
    #[allow(dead_code)]
    pub fn get_height_at(&self, x: usize, z: usize) -> f32 {
        // Search from top to bottom
        for y in (0..CHUNK_SIZE).rev() {
            if !self.get_block(x, y, z).is_air() {
                return y as f32 + 1.0; // Return top of block
            }
        }
        0.0 // No blocks found, return ground level
    }
}
