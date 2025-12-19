use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageFilterMode, ImageAddressMode};

use crate::blocks::{BlockRegistry, BlockFace, BlockId};

/// Size of each isometric cube icon in pixels
pub const ISOMETRIC_ICON_SIZE: u32 = 48;

/// Renders an isometric cube image for a block using standard 2:1 isometric projection
/// Shows top, south (front-left), and east (front-right) faces
pub fn render_isometric_cube(
    block_id: BlockId,
    block_registry: &BlockRegistry,
    atlas_data: &[u8],
    atlas_width: u32,
    atlas_height: u32,
) -> Option<Image> {
    let block_type = block_registry.get_block(block_id)?;

    let textures = &block_type.properties.textures;
    let tints = &block_type.properties.tint_colors;

    // Get atlas coordinates for each visible face
    let top_coord = textures.get_face(BlockFace::Top);
    let south_coord = textures.get_face(BlockFace::South);
    let east_coord = textures.get_face(BlockFace::East);

    // Get tints for each face
    let top_tint = tints.top;
    let south_tint = tints.south;
    let east_tint = tints.east;

    // Create the isometric image
    let size = ISOMETRIC_ICON_SIZE;
    let mut data = vec![0u8; (size * size * 4) as usize];

    // Standard 2:1 isometric projection
    // The cube is centered in the image with vertices at specific positions
    //
    // Isometric cube layout (looking at it):
    //           top_point
    //            /    \
    //           /      \
    //     left_point    right_point
    //          |\      /|
    //          | \    / |
    //          |  \  /  |
    //          |   \/   |
    //          |  front |
    //          |   ||   |
    //     bl_point  br_point
    //           \      /
    //            \    /
    //          bottom_point

    let cx = size as f32 / 2.0;
    let cy = size as f32 / 2.0;

    // Cube edge length in pixels (scaled to nearly fill the icon)
    let edge = size as f32 * 0.48;

    // Isometric angles: for 2:1 ratio, horizontal component is 2x vertical
    // Going right-down: dx = edge * cos(30°) ≈ edge * 0.866, dy = edge * sin(30°) = edge * 0.5
    // For pixel-perfect look, we use 2:1 ratio exactly
    let dx = edge;  // horizontal distance from center to left/right corners
    let dy = edge * 0.5;  // vertical distance for the slant
    let h = edge * 1.15;  // height of side faces (taller for proper cube proportions)

    // Key vertices of the isometric cube
    let top = (cx, cy - dy - h * 0.5);  // Topmost point
    let left = (cx - dx, cy - h * 0.5);  // Left corner of top face
    let right = (cx + dx, cy - h * 0.5);  // Right corner of top face
    let front = (cx, cy + dy - h * 0.5);  // Front corner of top face (bottom of top rhombus)
    let bottom_left = (cx - dx, cy + h * 0.5);  // Bottom of left face
    let bottom_right = (cx + dx, cy + h * 0.5);  // Bottom of right face
    let bottom = (cx, cy + dy + h * 0.5);  // Bottommost point

    // For each pixel, determine which face it belongs to and sample the texture
    for py in 0..size {
        for px in 0..size {
            let x = px as f32 + 0.5;
            let y = py as f32 + 0.5;

            let idx = ((py * size + px) * 4) as usize;

            // Check if pixel is in top face (rhombus: top, left, front, right)
            if let Some((u, v)) = point_in_quad(x, y, top, left, front, right) {
                sample_texture_to_buffer(&mut data, idx, atlas_data, atlas_width, atlas_height,
                    top_coord.x as u32, top_coord.y as u32, u, v, top_tint, 1.0);
            }
            // Check if pixel is in left/south face (parallelogram: left, bottom_left, bottom, front)
            else if let Some((u, v)) = point_in_quad(x, y, left, bottom_left, bottom, front) {
                // Left face is slightly darker for depth
                sample_texture_to_buffer(&mut data, idx, atlas_data, atlas_width, atlas_height,
                    south_coord.x as u32, south_coord.y as u32, u, v, south_tint, 0.8);
            }
            // Check if pixel is in right/east face (parallelogram: front, bottom, bottom_right, right)
            else if let Some((u, v)) = point_in_quad(x, y, front, bottom, bottom_right, right) {
                // Right face is even darker for depth
                sample_texture_to_buffer(&mut data, idx, atlas_data, atlas_width, atlas_height,
                    east_coord.x as u32, east_coord.y as u32, u, v, east_tint, 0.6);
            }
            // Pixel is outside the cube - leave transparent
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // Use nearest-neighbor filtering for pixel-art textures
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        label: None,
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..Default::default()
    });

    Some(image)
}

/// Check if a point is inside a quadrilateral defined by 4 vertices (in order)
/// Returns UV coordinates if inside, None if outside
/// Vertices should be in order: v0 (top), v1 (left), v2 (bottom), v3 (right) for rhombus
/// Or for parallelogram: v0 (top-left), v1 (bottom-left), v2 (bottom-right), v3 (top-right)
fn point_in_quad(
    x: f32, y: f32,
    v0: (f32, f32), v1: (f32, f32), v2: (f32, f32), v3: (f32, f32),
) -> Option<(f32, f32)> {
    // Check if point is inside the quad using cross product method
    // All cross products should have the same sign if point is inside

    let c0 = cross_2d_points((x, y), v0, v1);
    let c1 = cross_2d_points((x, y), v1, v2);
    let c2 = cross_2d_points((x, y), v2, v3);
    let c3 = cross_2d_points((x, y), v3, v0);

    let all_positive = c0 >= 0.0 && c1 >= 0.0 && c2 >= 0.0 && c3 >= 0.0;
    let all_negative = c0 <= 0.0 && c1 <= 0.0 && c2 <= 0.0 && c3 <= 0.0;

    if all_positive || all_negative {
        // Calculate UV using bilinear interpolation in the quad
        // For a quad with vertices v0, v1, v2, v3:
        // We need to find (u, v) such that P = lerp(lerp(v0, v3, u), lerp(v1, v2, u), v)

        // Simplified approach: use the position relative to the bounding parallelogram
        // For the top face (rhombus), u goes from left to right, v from top to bottom
        // For side faces (parallelograms), similar mapping

        let (u, v) = compute_quad_uv(x, y, v0, v1, v2, v3);
        Some((u.clamp(0.0, 1.0), v.clamp(0.0, 1.0)))
    } else {
        None
    }
}

/// Compute UV coordinates for a point inside a quadrilateral
fn compute_quad_uv(
    x: f32, y: f32,
    v0: (f32, f32), v1: (f32, f32), _v2: (f32, f32), v3: (f32, f32),
) -> (f32, f32) {
    // Use inverse bilinear interpolation
    // For a quad defined by v0, v1, v2, v3 (in order), find u,v such that:
    // P = (1-v)*((1-u)*v0 + u*v3) + v*((1-u)*v1 + u*v2)

    // Solve for u and v using iterative approach or direct formula
    // For parallelograms and rhombi, we can use a simpler linear approach

    // Vector from v0 to v3 (top edge direction)
    let top_dx = v3.0 - v0.0;
    let top_dy = v3.1 - v0.1;

    // Vector from v0 to v1 (left edge direction)
    let left_dx = v1.0 - v0.0;
    let left_dy = v1.1 - v0.1;

    // Point relative to v0
    let px = x - v0.0;
    let py = y - v0.1;

    // Solve the 2x2 system:
    // px = u * top_dx + v * left_dx
    // py = u * top_dy + v * left_dy

    let det = top_dx * left_dy - top_dy * left_dx;
    if det.abs() < 0.0001 {
        return (0.5, 0.5);
    }

    let u = (px * left_dy - py * left_dx) / det;
    let v = (top_dx * py - top_dy * px) / det;

    (u, v)
}

/// 2D cross product using three points (determines which side of line p0-p1 point p is on)
fn cross_2d_points(p: (f32, f32), p0: (f32, f32), p1: (f32, f32)) -> f32 {
    (p1.0 - p0.0) * (p.1 - p0.1) - (p1.1 - p0.1) * (p.0 - p0.0)
}

/// Sample a texture from the atlas and write to the output buffer
fn sample_texture_to_buffer(
    buffer: &mut [u8],
    idx: usize,
    atlas_data: &[u8],
    atlas_width: u32,
    atlas_height: u32,
    tile_x: u32,
    tile_y: u32,
    u: f32,
    v: f32,
    tint: Option<(f32, f32, f32)>,
    brightness: f32,
) {
    // Each tile is 16x16 pixels in a 256x256 atlas (16x16 grid)
    let tile_size = 16;
    let pixel_x = tile_x * tile_size + (u * (tile_size - 1) as f32) as u32;
    let pixel_y = tile_y * tile_size + (v * (tile_size - 1) as f32) as u32;

    // Clamp to atlas bounds
    let pixel_x = pixel_x.min(atlas_width - 1);
    let pixel_y = pixel_y.min(atlas_height - 1);

    // Sample the atlas (RGBA format)
    let atlas_idx = ((pixel_y * atlas_width + pixel_x) * 4) as usize;

    if atlas_idx + 3 < atlas_data.len() {
        let mut r = atlas_data[atlas_idx] as f32 / 255.0;
        let mut g = atlas_data[atlas_idx + 1] as f32 / 255.0;
        let mut b = atlas_data[atlas_idx + 2] as f32 / 255.0;
        let a = atlas_data[atlas_idx + 3];

        // Apply tint
        if let Some((tr, tg, tb)) = tint {
            r *= tr;
            g *= tg;
            b *= tb;
        }

        // Apply brightness for face shading
        r *= brightness;
        g *= brightness;
        b *= brightness;

        buffer[idx] = (r * 255.0).min(255.0) as u8;
        buffer[idx + 1] = (g * 255.0).min(255.0) as u8;
        buffer[idx + 2] = (b * 255.0).min(255.0) as u8;
        buffer[idx + 3] = a;
    }
}

/// Cache for pre-rendered isometric cube icons
#[derive(Resource, Default)]
pub struct IsometricIconCache {
    pub icons: bevy::utils::HashMap<BlockId, Handle<Image>>,
}

impl IsometricIconCache {
    pub fn get(&self, block_id: BlockId) -> Option<&Handle<Image>> {
        self.icons.get(&block_id)
    }

    pub fn insert(&mut self, block_id: BlockId, handle: Handle<Image>) {
        self.icons.insert(block_id, handle);
    }
}
