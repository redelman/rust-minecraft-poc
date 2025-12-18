use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use super::chunk::{Chunk, CHUNK_SIZE, MAX_LIGHT_LEVEL};
use crate::blocks::{BlockRegistry, BlockFace, BlockId};

/// Neighbor chunks for face culling (6 directions: -X, +X, -Y, +Y, -Z, +Z)
pub struct NeighborChunks<'a> {
    pub neg_x: Option<&'a Chunk>,  // West
    pub pos_x: Option<&'a Chunk>,  // East
    pub neg_y: Option<&'a Chunk>,  // Down
    pub pos_y: Option<&'a Chunk>,  // Up
    pub neg_z: Option<&'a Chunk>,  // North
    pub pos_z: Option<&'a Chunk>,  // South
}

impl<'a> NeighborChunks<'a> {
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

/// Check if a block face should be rendered with neighbor chunk support
fn should_render_face_with_neighbors(
    chunk: &Chunk,
    neighbors: &NeighborChunks,
    x: usize,
    y: usize,
    z: usize,
    dx: i32,
    dy: i32,
    dz: i32,
) -> bool {
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    let nz = z as i32 + dz;

    // Check within current chunk
    if nx >= 0 && nx < CHUNK_SIZE as i32 &&
       ny >= 0 && ny < CHUNK_SIZE as i32 &&
       nz >= 0 && nz < CHUNK_SIZE as i32 {
        return chunk.get_block(nx as usize, ny as usize, nz as usize).is_air();
    }

    // Check neighbor chunks
    // Only check one neighbor at a time - faces can only cross one chunk boundary
    if nx < 0 {
        // West neighbor (-X)
        if let Some(neighbor) = neighbors.neg_x {
            return neighbor.get_block(CHUNK_SIZE - 1, y, z).is_air();
        }
    } else if nx >= CHUNK_SIZE as i32 {
        // East neighbor (+X)
        if let Some(neighbor) = neighbors.pos_x {
            return neighbor.get_block(0, y, z).is_air();
        }
    } else if ny < 0 {
        // Down neighbor (-Y)
        if let Some(neighbor) = neighbors.neg_y {
            return neighbor.get_block(x, CHUNK_SIZE - 1, z).is_air();
        }
    } else if ny >= CHUNK_SIZE as i32 {
        // Up neighbor (+Y)
        if let Some(neighbor) = neighbors.pos_y {
            return neighbor.get_block(x, 0, z).is_air();
        }
    } else if nz < 0 {
        // North neighbor (-Z)
        if let Some(neighbor) = neighbors.neg_z {
            return neighbor.get_block(x, y, CHUNK_SIZE - 1).is_air();
        }
    } else if nz >= CHUNK_SIZE as i32 {
        // South neighbor (+Z)
        if let Some(neighbor) = neighbors.pos_z {
            return neighbor.get_block(x, y, 0).is_air();
        }
    }

    // No neighbor loaded - render the face (chunk boundary)
    true
}

pub fn create_chunk_mesh(chunk: &Chunk, block_registry: &BlockRegistry) -> Option<Mesh> {
    create_chunk_mesh_with_neighbors(chunk, block_registry, &NeighborChunks::none())
}

/// Face brightness values for baked lighting (Minecraft-style)
/// These match MC's hardcoded directional shading values
const BRIGHTNESS_UP: f32 = 1.0;        // Top faces get full light
const BRIGHTNESS_DOWN: f32 = 0.5;      // Bottom faces are darkest
const BRIGHTNESS_NORTH: f32 = 0.8;     // North (-Z)
const BRIGHTNESS_SOUTH: f32 = 0.8;     // South (+Z)
const BRIGHTNESS_EAST: f32 = 0.6;      // East (+X)
const BRIGHTNESS_WEST: f32 = 0.6;      // West (-X)

/// Small offset for overlay quads to prevent z-fighting
const OVERLAY_OFFSET: f32 = 0.001;

/// Get the light level at a position, using neighbor chunks for boundary lookups
/// Returns light level 0-15
/// When neighbors are missing, we assume full sky light for horizontal boundaries
/// (since the face is exposed to the outside world)
fn get_light_at(
    chunk: &Chunk,
    neighbors: &NeighborChunks,
    x: i32,
    y: i32,
    z: i32,
) -> u8 {
    // Within current chunk - use chunk's own light data
    if x >= 0 && x < CHUNK_SIZE as i32 &&
       y >= 0 && y < CHUNK_SIZE as i32 &&
       z >= 0 && z < CHUNK_SIZE as i32 {
        return chunk.get_light(x as usize, y as usize, z as usize);
    }

    // Looking above the chunk - check neighbor or assume sky light
    if y >= CHUNK_SIZE as i32 {
        if let Some(neighbor) = neighbors.pos_y {
            if x >= 0 && x < CHUNK_SIZE as i32 && z >= 0 && z < CHUNK_SIZE as i32 {
                return neighbor.get_light(x as usize, 0, z as usize);
            }
        }
        // No chunk above - assume full sky light (open sky)
        return MAX_LIGHT_LEVEL;
    }

    // Below the chunk
    if y < 0 {
        if let Some(neighbor) = neighbors.neg_y {
            if x >= 0 && x < CHUNK_SIZE as i32 && z >= 0 && z < CHUNK_SIZE as i32 {
                return neighbor.get_light(x as usize, CHUNK_SIZE - 1, z as usize);
            }
        }
        // Below ground with no neighbor - assume dark (underground)
        return 0;
    }

    // For horizontal boundaries, try to get from neighbor
    // If no neighbor available, assume full sky light since the face is exposed
    // to the outside world (chunk boundary = world edge or unloaded area)
    if x < 0 {
        if let Some(neighbor) = neighbors.neg_x {
            if y >= 0 && y < CHUNK_SIZE as i32 && z >= 0 && z < CHUNK_SIZE as i32 {
                return neighbor.get_light(CHUNK_SIZE - 1, y as usize, z as usize);
            }
        }
        // No neighbor - assume sky light for exposed face
        return MAX_LIGHT_LEVEL;
    }

    if x >= CHUNK_SIZE as i32 {
        if let Some(neighbor) = neighbors.pos_x {
            if y >= 0 && y < CHUNK_SIZE as i32 && z >= 0 && z < CHUNK_SIZE as i32 {
                return neighbor.get_light(0, y as usize, z as usize);
            }
        }
        return MAX_LIGHT_LEVEL;
    }

    if z < 0 {
        if let Some(neighbor) = neighbors.neg_z {
            if x >= 0 && x < CHUNK_SIZE as i32 && y >= 0 && y < CHUNK_SIZE as i32 {
                return neighbor.get_light(x as usize, y as usize, CHUNK_SIZE - 1);
            }
        }
        return MAX_LIGHT_LEVEL;
    }

    if z >= CHUNK_SIZE as i32 {
        if let Some(neighbor) = neighbors.pos_z {
            if x >= 0 && x < CHUNK_SIZE as i32 && y >= 0 && y < CHUNK_SIZE as i32 {
                return neighbor.get_light(x as usize, y as usize, 0);
            }
        }
        return MAX_LIGHT_LEVEL;
    }

    // Shouldn't reach here, but fallback to full light
    MAX_LIGHT_LEVEL
}

/// Minimum brightness - even in complete darkness, there's some ambient light
/// This prevents the world from being pitch black but keeps caves very dark
const MIN_BRIGHTNESS: f32 = 0.05;

/// Convert light level (0-15) to brightness multiplier
/// Uses a curve that provides good contrast at high levels but smooth falloff at low levels
fn light_to_brightness(light: u8, sky_light_level: u8) -> f32 {
    // Clamp stored light by current sky light level
    // Blocks can't be brighter than the current global sky light
    let effective_light = light.min(sky_light_level);

    // Use a quadratic-ish curve that's smoother at low light levels
    // This avoids the harsh jump between light 1 and 0
    // Formula: brightness = ((light + 1) / 16)^1.5
    // Light 15 = 1.0, Light 7 ≈ 0.35, Light 3 ≈ 0.18, Light 1 ≈ 0.09, Light 0 ≈ 0.05
    let normalized = (effective_light as f32 + 1.0) / 16.0;
    let base_brightness = normalized.powf(1.5);

    // Ensure we don't go below minimum brightness
    base_brightness.max(MIN_BRIGHTNESS)
}

pub fn create_chunk_mesh_with_neighbors(chunk: &Chunk, block_registry: &BlockRegistry, neighbors: &NeighborChunks) -> Option<Mesh> {
    // Use full daylight for backward compatibility
    create_chunk_mesh_with_sky_light(chunk, block_registry, neighbors, MAX_LIGHT_LEVEL)
}

/// Get the maximum light level from all 6 adjacent blocks
/// This gives a block its "ambient" light level for all faces
fn get_block_light(chunk: &Chunk, neighbors: &NeighborChunks, x: usize, y: usize, z: usize) -> u8 {
    let xi = x as i32;
    let yi = y as i32;
    let zi = z as i32;

    // Get light from all 6 adjacent positions and take the maximum
    let light_up = get_light_at(chunk, neighbors, xi, yi + 1, zi);
    let light_down = get_light_at(chunk, neighbors, xi, yi - 1, zi);
    let light_north = get_light_at(chunk, neighbors, xi, yi, zi - 1);
    let light_south = get_light_at(chunk, neighbors, xi, yi, zi + 1);
    let light_east = get_light_at(chunk, neighbors, xi + 1, yi, zi);
    let light_west = get_light_at(chunk, neighbors, xi - 1, yi, zi);

    light_up.max(light_down).max(light_north).max(light_south).max(light_east).max(light_west)
}

/// Create chunk mesh with a specific sky light level (for day/night cycle)
/// Uses UV2 attribute for overlay textures (grass sides) instead of separate quads
pub fn create_chunk_mesh_with_sky_light(chunk: &Chunk, block_registry: &BlockRegistry, neighbors: &NeighborChunks, sky_light_level: u8) -> Option<Mesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut uv2s: Vec<[f32; 2]> = Vec::new(); // Overlay texture UVs (0,0 = no overlay)
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // No overlay marker
    let no_overlay: [f32; 2] = [0.0, 0.0];

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block_id = chunk.get_block(x, y, z);
                if block_id.is_air() {
                    continue;
                }

                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;

                // Get block properties
                let block_type = block_registry.get_block(block_id);
                let tint_colors = block_type.map(|b| &b.properties.tint_colors);
                let textures = block_type.map(|b| &b.properties.textures);
                let side_overlay = textures.and_then(|t| t.side_overlay);

                // Helper to get brightness for a specific face direction
                // Each face uses the light level of the air block it's facing
                let xi = x as i32;
                let yi = y as i32;
                let zi = z as i32;
                let get_face_brightness = |dx: i32, dy: i32, dz: i32, face_shading: f32| -> f32 {
                    let face_light = get_light_at(chunk, neighbors, xi + dx, yi + dy, zi + dz);
                    let brightness = light_to_brightness(face_light, sky_light_level);
                    face_shading * brightness
                };

                // Top face (+Y) - no overlay on top faces
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, 0, 1, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);

                    // Get texture coordinates from block registry
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::Top).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0) // Default to (0,0) if block not found
                    };

                    // No V-flip needed for horizontal faces
                    uvs.extend_from_slice(&[
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max],
                        [u_min, v_max]
                    ]);
                    // No overlay on top face
                    uv2s.extend_from_slice(&[no_overlay; 4]);

                    // Vertex colors: RGB = tint, A = brightness
                    // The shader applies tint to texture, then multiplies by alpha for lighting
                    let tint = tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0));
                    let brightness = get_face_brightness(0, 1, 0, BRIGHTNESS_UP);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);

                    // Reverse winding order so top face is visible from above (counter-clockwise when viewed from above)
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // Bottom face (-Y) - no overlay on bottom faces
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, 0, -1, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx, fy, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                        [fx + 1.0, fy, fz],
                    ]);
                    normals.extend_from_slice(&[[0.0, -1.0, 0.0]; 4]);

                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::Bottom).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0)
                    };

                    uvs.extend_from_slice(&[
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max],
                        [u_min, v_max]
                    ]);
                    // No overlay on bottom face
                    uv2s.extend_from_slice(&[no_overlay; 4]);

                    let tint = tint_colors.and_then(|t| t.bottom).unwrap_or((1.0, 1.0, 1.0));
                    let brightness = get_face_brightness(0, -1, 0, BRIGHTNESS_DOWN);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    // Winding order for bottom face - counter-clockwise when viewed from below
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // Front face (+Z) - South
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, 0, 0, 1) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);

                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::South).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0)
                    };

                    // Swap v_min and v_max because PNG has Y=0 at top, but we need it at bottom for vertical faces
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max]
                    ]);

                    // Add overlay UV2 if present
                    let has_overlay = if let Some(overlay_coord) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay_coord.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_min, ov_min],
                            [ou_max, ov_min],
                            [ou_max, ov_max]
                        ]);
                        true
                    } else {
                        uv2s.extend_from_slice(&[no_overlay; 4]);
                        false
                    };

                    // For faces with overlay, use top tint (grass color) for the overlay
                    // For faces without overlay, use normal side tint
                    let tint = if has_overlay {
                        tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0))
                    } else {
                        tint_colors.and_then(|t| t.south).unwrap_or((1.0, 1.0, 1.0))
                    };
                    let brightness = get_face_brightness(0, 0, 1, BRIGHTNESS_SOUTH);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    // Reverse winding order for proper front-facing visibility
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // Back face (-Z) - North
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, 0, 0, -1) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [fx, fy + 1.0, fz],
                    ]);
                    normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);

                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::North).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0)
                    };

                    // Swap v_min and v_max for vertical faces
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_max, v_max],
                        [u_max, v_min],
                        [u_min, v_min]
                    ]);

                    // Add overlay UV2 if present
                    let has_overlay = if let Some(overlay_coord) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay_coord.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_max, ov_max],
                            [ou_max, ov_min],
                            [ou_min, ov_min]
                        ]);
                        true
                    } else {
                        uv2s.extend_from_slice(&[no_overlay; 4]);
                        false
                    };

                    let tint = if has_overlay {
                        tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0))
                    } else {
                        tint_colors.and_then(|t| t.north).unwrap_or((1.0, 1.0, 1.0))
                    };
                    let brightness = get_face_brightness(0, 0, -1, BRIGHTNESS_NORTH);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    // Reverse winding order for proper front-facing visibility
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // Right face (+X) - East
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, 1, 0, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz],
                    ]);
                    normals.extend_from_slice(&[[1.0, 0.0, 0.0]; 4]);

                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::East).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0)
                    };

                    // Swap v_min and v_max for vertical faces
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_max, v_max],
                        [u_max, v_min],
                        [u_min, v_min]
                    ]);

                    // Add overlay UV2 if present
                    let has_overlay = if let Some(overlay_coord) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay_coord.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_max, ov_max],
                            [ou_max, ov_min],
                            [ou_min, ov_min]
                        ]);
                        true
                    } else {
                        uv2s.extend_from_slice(&[no_overlay; 4]);
                        false
                    };

                    let tint = if has_overlay {
                        tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0))
                    } else {
                        tint_colors.and_then(|t| t.east).unwrap_or((1.0, 1.0, 1.0))
                    };
                    let brightness = get_face_brightness(1, 0, 0, BRIGHTNESS_EAST);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    // Reverse winding order for proper front-facing visibility
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // Left face (-X) - West
                if should_render_face_with_neighbors(chunk, neighbors, x, y, z, -1, 0, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx, fy + 1.0, fz],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx, fy, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[-1.0, 0.0, 0.0]; 4]);

                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::West).uv_coords()
                    } else {
                        (0.0, 0.0, 1.0/16.0, 1.0/16.0)
                    };

                    // Swap v_min and v_max for vertical faces
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max]
                    ]);

                    // Add overlay UV2 if present
                    let has_overlay = if let Some(overlay_coord) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay_coord.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_min, ov_min],
                            [ou_max, ov_min],
                            [ou_max, ov_max]
                        ]);
                        true
                    } else {
                        uv2s.extend_from_slice(&[no_overlay; 4]);
                        false
                    };

                    let tint = if has_overlay {
                        tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0))
                    } else {
                        tint_colors.and_then(|t| t.west).unwrap_or((1.0, 1.0, 1.0))
                    };
                    let brightness = get_face_brightness(-1, 0, 0, BRIGHTNESS_WEST);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    // Reverse winding order for proper front-facing visibility
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }
            }
        }
    }

    if positions.is_empty() {
        // Return None if no visible faces (empty chunk or all faces culled)
        return None;
    }

    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uv2s)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
        .with_inserted_indices(Indices::U32(indices))
    )
}

/// Create chunk mesh using closures for neighbor lookups
/// This allows using cached data for face culling instead of live chunk references
pub fn create_chunk_mesh_with_cached_neighbors<F, G>(
    chunk: &Chunk,
    block_registry: &BlockRegistry,
    sky_light_level: u8,
    get_neighbor_block: F,
    get_neighbor_light: G,
) -> Option<Mesh>
where
    F: Fn(i32, i32, i32, usize, usize, usize) -> crate::blocks::BlockId,
    G: Fn(i32, i32, i32, usize, usize, usize) -> u8,
{
    use crate::blocks::BlockFace;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut uv2s: Vec<[f32; 2]> = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let no_overlay: [f32; 2] = [0.0, 0.0];

    // Helper to check if a face should be rendered
    // Only one axis can cross a chunk boundary at a time (faces are axis-aligned)
    let should_render_face = |x: usize, y: usize, z: usize, dx: i32, dy: i32, dz: i32| -> bool {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        let nz = z as i32 + dz;

        // Within chunk
        if nx >= 0 && nx < CHUNK_SIZE as i32 &&
           ny >= 0 && ny < CHUNK_SIZE as i32 &&
           nz >= 0 && nz < CHUNK_SIZE as i32 {
            return chunk.get_block(nx as usize, ny as usize, nz as usize).is_air();
        }

        // Cross-chunk boundary - only ONE axis crosses at a time
        if nx < 0 {
            // West neighbor (-X)
            get_neighbor_block(-1, 0, 0, CHUNK_SIZE - 1, y, z).is_air()
        } else if nx >= CHUNK_SIZE as i32 {
            // East neighbor (+X)
            get_neighbor_block(1, 0, 0, 0, y, z).is_air()
        } else if ny < 0 {
            // Down neighbor (-Y)
            get_neighbor_block(0, -1, 0, x, CHUNK_SIZE - 1, z).is_air()
        } else if ny >= CHUNK_SIZE as i32 {
            // Up neighbor (+Y)
            get_neighbor_block(0, 1, 0, x, 0, z).is_air()
        } else if nz < 0 {
            // North neighbor (-Z)
            get_neighbor_block(0, 0, -1, x, y, CHUNK_SIZE - 1).is_air()
        } else if nz >= CHUNK_SIZE as i32 {
            // South neighbor (+Z)
            get_neighbor_block(0, 0, 1, x, y, 0).is_air()
        } else {
            // Shouldn't happen, but default to rendering the face
            true
        }
    };

    // Helper to get light at a position
    let get_light = |x: i32, y: i32, z: i32| -> u8 {
        // Within chunk
        if x >= 0 && x < CHUNK_SIZE as i32 &&
           y >= 0 && y < CHUNK_SIZE as i32 &&
           z >= 0 && z < CHUNK_SIZE as i32 {
            return chunk.get_light(x as usize, y as usize, z as usize);
        }

        // Above chunk
        if y >= CHUNK_SIZE as i32 {
            let cy = 0;
            let cx = x.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            let cz = z.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            return get_neighbor_light(0, 1, 0, cx, cy, cz);
        }

        // Below chunk
        if y < 0 {
            let cy = CHUNK_SIZE - 1;
            let cx = x.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            let cz = z.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            return get_neighbor_light(0, -1, 0, cx, cy, cz);
        }

        let cy = y as usize;

        // Horizontal boundaries
        if x < 0 {
            let cx = CHUNK_SIZE - 1;
            let cz = z.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            return get_neighbor_light(-1, 0, 0, cx, cy, cz);
        }
        if x >= CHUNK_SIZE as i32 {
            let cx = 0;
            let cz = z.clamp(0, CHUNK_SIZE as i32 - 1) as usize;
            return get_neighbor_light(1, 0, 0, cx, cy, cz);
        }
        if z < 0 {
            let cx = x as usize;
            let cz = CHUNK_SIZE - 1;
            return get_neighbor_light(0, 0, -1, cx, cy, cz);
        }
        if z >= CHUNK_SIZE as i32 {
            let cx = x as usize;
            let cz = 0;
            return get_neighbor_light(0, 0, 1, cx, cy, cz);
        }

        0
    };

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let block_id = chunk.get_block(x, y, z);
                if block_id.is_air() {
                    continue;
                }

                let fx = x as f32;
                let fy = y as f32;
                let fz = z as f32;

                let block_type = block_registry.get_block(block_id);
                let tint_colors = block_type.map(|b| &b.properties.tint_colors);
                let textures = block_type.map(|b| &b.properties.textures);
                let side_overlay = textures.and_then(|t| t.side_overlay);

                let xi = x as i32;
                let yi = y as i32;
                let zi = z as i32;
                let get_face_brightness = |dx: i32, dy: i32, dz: i32, face_shading: f32| -> f32 {
                    let face_light = get_light(xi + dx, yi + dy, zi + dz);
                    let brightness = light_to_brightness(face_light, sky_light_level);
                    face_shading * brightness
                };

                // Top face (+Y)
                if should_render_face(x, y, z, 0, 1, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::Top).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[[u_min, v_min], [u_max, v_min], [u_max, v_max], [u_min, v_max]]);
                    uv2s.extend_from_slice(&[no_overlay; 4]);
                    let tint = tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0));
                    let brightness = get_face_brightness(0, 1, 0, BRIGHTNESS_UP);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[base_index, base_index + 3, base_index + 2, base_index + 2, base_index + 1, base_index]);
                }

                // Bottom face (-Y) - matches original create_chunk_mesh_with_sky_light
                if should_render_face(x, y, z, 0, -1, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx, fy, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                        [fx + 1.0, fy, fz],
                    ]);
                    normals.extend_from_slice(&[[0.0, -1.0, 0.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::Bottom).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max],
                        [u_min, v_max]
                    ]);
                    uv2s.extend_from_slice(&[no_overlay; 4]);
                    let tint = tint_colors.and_then(|t| t.bottom).unwrap_or((1.0, 1.0, 1.0));
                    let brightness = get_face_brightness(0, -1, 0, BRIGHTNESS_DOWN);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // South face (+Z) - matches original
                if should_render_face(x, y, z, 0, 0, 1) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz + 1.0],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[0.0, 0.0, 1.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::South).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max]
                    ]);
                    let has_overlay = if let Some(overlay) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_min, ov_min],
                            [ou_max, ov_min],
                            [ou_max, ov_max]
                        ]);
                        true
                    } else { uv2s.extend_from_slice(&[no_overlay; 4]); false };
                    let tint = if has_overlay { tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0)) }
                        else { tint_colors.and_then(|t| t.south).unwrap_or((1.0, 1.0, 1.0)) };
                    let brightness = get_face_brightness(0, 0, 1, BRIGHTNESS_SOUTH);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // North face (-Z) - matches original
                if should_render_face(x, y, z, 0, 0, -1) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy + 1.0, fz],
                        [fx, fy + 1.0, fz],
                    ]);
                    normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::North).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_max, v_max],
                        [u_max, v_min],
                        [u_min, v_min]
                    ]);
                    let has_overlay = if let Some(overlay) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_max, ov_max],
                            [ou_max, ov_min],
                            [ou_min, ov_min]
                        ]);
                        true
                    } else { uv2s.extend_from_slice(&[no_overlay; 4]); false };
                    let tint = if has_overlay { tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0)) }
                        else { tint_colors.and_then(|t| t.north).unwrap_or((1.0, 1.0, 1.0)) };
                    let brightness = get_face_brightness(0, 0, -1, BRIGHTNESS_NORTH);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // East face (+X) - matches original
                if should_render_face(x, y, z, 1, 0, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx + 1.0, fy, fz],
                        [fx + 1.0, fy, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz + 1.0],
                        [fx + 1.0, fy + 1.0, fz],
                    ]);
                    normals.extend_from_slice(&[[1.0, 0.0, 0.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::East).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_max, v_max],
                        [u_max, v_min],
                        [u_min, v_min]
                    ]);
                    let has_overlay = if let Some(overlay) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_max, ov_max],
                            [ou_max, ov_min],
                            [ou_min, ov_min]
                        ]);
                        true
                    } else { uv2s.extend_from_slice(&[no_overlay; 4]); false };
                    let tint = if has_overlay { tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0)) }
                        else { tint_colors.and_then(|t| t.east).unwrap_or((1.0, 1.0, 1.0)) };
                    let brightness = get_face_brightness(1, 0, 0, BRIGHTNESS_EAST);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }

                // West face (-X) - matches original
                if should_render_face(x, y, z, -1, 0, 0) {
                    let base_index = positions.len() as u32;
                    positions.extend_from_slice(&[
                        [fx, fy, fz],
                        [fx, fy + 1.0, fz],
                        [fx, fy + 1.0, fz + 1.0],
                        [fx, fy, fz + 1.0],
                    ]);
                    normals.extend_from_slice(&[[-1.0, 0.0, 0.0]; 4]);
                    let (u_min, v_min, u_max, v_max) = if let Some(bt) = block_type {
                        bt.properties.textures.get_face(BlockFace::West).uv_coords()
                    } else { (0.0, 0.0, 1.0/16.0, 1.0/16.0) };
                    uvs.extend_from_slice(&[
                        [u_min, v_max],
                        [u_min, v_min],
                        [u_max, v_min],
                        [u_max, v_max]
                    ]);
                    let has_overlay = if let Some(overlay) = side_overlay {
                        let (ou_min, ov_min, ou_max, ov_max) = overlay.uv_coords();
                        uv2s.extend_from_slice(&[
                            [ou_min, ov_max],
                            [ou_min, ov_min],
                            [ou_max, ov_min],
                            [ou_max, ov_max]
                        ]);
                        true
                    } else { uv2s.extend_from_slice(&[no_overlay; 4]); false };
                    let tint = if has_overlay { tint_colors.and_then(|t| t.top).unwrap_or((1.0, 1.0, 1.0)) }
                        else { tint_colors.and_then(|t| t.west).unwrap_or((1.0, 1.0, 1.0)) };
                    let brightness = get_face_brightness(-1, 0, 0, BRIGHTNESS_WEST);
                    colors.extend_from_slice(&[[tint.0, tint.1, tint.2, brightness]; 4]);
                    indices.extend_from_slice(&[
                        base_index, base_index + 3, base_index + 2,
                        base_index + 2, base_index + 1, base_index,
                    ]);
                }
            }
        }
    }

    if positions.is_empty() {
        return None;
    }

    Some(
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, uv2s)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
            .with_inserted_indices(Indices::U32(indices))
    )
}
