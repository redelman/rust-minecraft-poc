use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::pbr::Material;

/// Custom material for terrain rendering with support for:
/// - Texture atlas sampling
/// - Overlay texture blending (for grass sides)
/// - Per-vertex light levels
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct TerrainMaterial {
    /// Settings: x = minimum ambient light level
    #[uniform(0)]
    pub settings: Vec4,

    /// Base texture atlas
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Handle<Image>,
}

#[allow(dead_code)]
impl TerrainMaterial {
    pub fn new(base_texture: Handle<Image>) -> Self {
        Self {
            settings: Vec4::new(0.1, 0.0, 0.0, 0.0), // 0.1 = 10% minimum ambient light
            base_texture,
        }
    }

    /// Set the minimum ambient light level (0.0 - 1.0)
    pub fn with_min_light(mut self, min_light: f32) -> Self {
        self.settings.x = min_light;
        self
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/terrain_shader.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_shader.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Mask(0.5)
    }
}
