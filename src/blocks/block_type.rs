/// Represents which face of a block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockFace {
    Top,
    Bottom,
    North,
    South,
    East,
    West,
}

/// Properties of a block type
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BlockProperties {
    /// Unique identifier for this block type (e.g., "core:stone", "mymod:custom_block")
    pub id: String,
    /// Display name for the block
    pub name: String,
    /// Whether the block is solid (affects collision, rendering, etc.)
    pub is_solid: bool,
    /// Whether the block is transparent (affects rendering optimization)
    pub is_transparent: bool,
    /// Whether the block emits light
    pub light_emission: u8,
    /// Texture paths for each face (can use the same texture for all faces)
    pub textures: BlockTextures,
    /// Per-face tint colors for biome-specific coloring (e.g., grass, leaves)
    /// None = no tint (white), Some = tint color
    pub tint_colors: FaceTints,
}

/// Per-face tint colors
#[derive(Debug, Clone, Copy)]
pub struct FaceTints {
    pub top: Option<(f32, f32, f32)>,
    pub bottom: Option<(f32, f32, f32)>,
    pub north: Option<(f32, f32, f32)>,
    pub south: Option<(f32, f32, f32)>,
    pub east: Option<(f32, f32, f32)>,
    pub west: Option<(f32, f32, f32)>,
}

#[allow(dead_code)]
impl FaceTints {
    /// No tint on any face
    pub fn none() -> Self {
        Self {
            top: None,
            bottom: None,
            north: None,
            south: None,
            east: None,
            west: None,
        }
    }

    /// Same tint on all faces
    pub fn uniform(color: (f32, f32, f32)) -> Self {
        Some(color).into()
    }

    /// Tint on top and sides, no tint on bottom
    pub fn top_and_sides(color: (f32, f32, f32)) -> Self {
        Self {
            top: Some(color),
            bottom: None,
            north: Some(color),
            south: Some(color),
            east: Some(color),
            west: Some(color),
        }
    }
}

impl From<Option<(f32, f32, f32)>> for FaceTints {
    fn from(color: Option<(f32, f32, f32)>) -> Self {
        match color {
            Some(c) => Self {
                top: Some(c),
                bottom: Some(c),
                north: Some(c),
                south: Some(c),
                east: Some(c),
                west: Some(c),
            },
            None => Self::none(),
        }
    }
}

/// Texture atlas coordinates for a block face
/// Stored as (x, y) grid position in a 16x16 atlas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasCoord {
    pub x: u8,
    pub y: u8,
}

impl AtlasCoord {
    pub const fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// Get UV coordinates for this atlas position
    /// Returns (u_min, v_min, u_max, v_max) for a 16x16 grid
    /// Applies a small padding to prevent texture bleeding between atlas cells
    pub fn uv_coords(&self) -> (f32, f32, f32, f32) {
        // For a 16x16 atlas with 256x256 pixels, each cell is 16 pixels
        // We apply a 0.5 pixel padding on each edge to prevent bleeding
        // 0.5 pixels / 256 pixels = 0.001953125 in UV space
        const PADDING: f32 = 0.001953125;

        let u_min = (self.x as f32 / 16.0) + PADDING;
        let v_min = (self.y as f32 / 16.0) + PADDING;
        let u_max = ((self.x + 1) as f32 / 16.0) - PADDING;
        let v_max = ((self.y + 1) as f32 / 16.0) - PADDING;
        (u_min, v_min, u_max, v_max)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockTextures {
    pub top: AtlasCoord,
    pub bottom: AtlasCoord,
    pub north: AtlasCoord,
    pub south: AtlasCoord,
    pub east: AtlasCoord,
    pub west: AtlasCoord,
    /// Optional overlay texture for side faces (e.g., grass overlay on dirt)
    /// This greyscale texture gets tinted and rendered on top of side faces
    pub side_overlay: Option<AtlasCoord>,
}

#[allow(dead_code)]
impl BlockTextures {
    /// Create a texture set where all faces use the same texture
    pub fn uniform(coord: AtlasCoord) -> Self {
        Self {
            top: coord,
            bottom: coord,
            north: coord,
            south: coord,
            east: coord,
            west: coord,
            side_overlay: None,
        }
    }

    /// Create a texture set with different top/bottom and uniform sides
    pub fn top_bottom_sides(top: AtlasCoord, bottom: AtlasCoord, sides: AtlasCoord) -> Self {
        Self {
            top,
            bottom,
            north: sides,
            south: sides,
            east: sides,
            west: sides,
            side_overlay: None,
        }
    }

    /// Create a texture set with overlay on sides (like grass blocks)
    /// The sides show the base texture with a tinted overlay on top
    pub fn with_side_overlay(top: AtlasCoord, bottom: AtlasCoord, sides: AtlasCoord, overlay: AtlasCoord) -> Self {
        Self {
            top,
            bottom,
            north: sides,
            south: sides,
            east: sides,
            west: sides,
            side_overlay: Some(overlay),
        }
    }

    /// Get the texture coordinates for a specific face
    pub fn get_face(&self, face: BlockFace) -> AtlasCoord {
        match face {
            BlockFace::Top => self.top,
            BlockFace::Bottom => self.bottom,
            BlockFace::North => self.north,
            BlockFace::South => self.south,
            BlockFace::East => self.east,
            BlockFace::West => self.west,
        }
    }
}

/// Represents a specific type of block
#[derive(Debug, Clone)]
pub struct BlockType {
    pub properties: BlockProperties,
}

#[allow(dead_code)]
impl BlockType {
    pub fn new(properties: BlockProperties) -> Self {
        Self { properties }
    }

    /// Builder pattern for creating block types
    pub fn builder(id: &str, name: &str) -> BlockTypeBuilder {
        BlockTypeBuilder {
            id: id.to_string(),
            name: name.to_string(),
            is_solid: true,
            is_transparent: false,
            light_emission: 0,
            textures: None,
            tint_colors: FaceTints::none(),
        }
    }
}

pub struct BlockTypeBuilder {
    id: String,
    name: String,
    is_solid: bool,
    is_transparent: bool,
    light_emission: u8,
    textures: Option<BlockTextures>,
    tint_colors: FaceTints,
}

#[allow(dead_code)]
impl BlockTypeBuilder {
    pub fn solid(mut self, is_solid: bool) -> Self {
        self.is_solid = is_solid;
        self
    }

    pub fn transparent(mut self, is_transparent: bool) -> Self {
        self.is_transparent = is_transparent;
        self
    }

    pub fn light_emission(mut self, level: u8) -> Self {
        self.light_emission = level;
        self
    }

    pub fn textures(mut self, textures: BlockTextures) -> Self {
        self.textures = Some(textures);
        self
    }

    pub fn tint_colors(mut self, tints: FaceTints) -> Self {
        self.tint_colors = tints;
        self
    }

    pub fn build(self) -> BlockType {
        BlockType {
            properties: BlockProperties {
                id: self.id,
                name: self.name,
                is_solid: self.is_solid,
                is_transparent: self.is_transparent,
                light_emission: self.light_emission,
                textures: self.textures.unwrap_or_else(|| BlockTextures::uniform(AtlasCoord::new(0, 0))),
                tint_colors: self.tint_colors,
            },
        }
    }
}
