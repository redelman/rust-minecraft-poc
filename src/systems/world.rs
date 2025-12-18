use bevy::prelude::*;
use crate::rendering::VoxelExtendedMaterial;

pub fn update_voxel_material(
    time: Res<Time>,
    mut materials: ResMut<Assets<VoxelExtendedMaterial>>,
) {
    for (_, material) in materials.iter_mut() {
        material.extension.set_time(time.elapsed_secs());
    }
}
