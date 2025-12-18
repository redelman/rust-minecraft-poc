use bevy::prelude::*;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial};
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

// Custom material extension for voxel shading
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct VoxelMaterial {
    #[uniform(100)]
    pub data: Vec4,
}

impl VoxelMaterial {
    pub fn new(time: f32, brightness: f32) -> Self {
        Self {
            data: Vec4::new(time, brightness, 0.0, 0.0),
        }
    }

    pub fn set_time(&mut self, time: f32) {
        self.data.x = time;
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.data.y = brightness;
    }
}

impl MaterialExtension for VoxelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/voxel_shader.wgsl".into()
    }
}

pub type VoxelExtendedMaterial = ExtendedMaterial<StandardMaterial, VoxelMaterial>;
