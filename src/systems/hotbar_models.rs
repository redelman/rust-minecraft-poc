use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::view::RenderLayers;
use crate::components::HotbarSlot;
use crate::resources::PlayerInventory;
use crate::blocks::{BlockRegistry, BlockId, BlockFace};
use crate::rendering::VoxelExtendedMaterial;

/// Marker component for hotbar 3D block models
#[derive(Component)]
pub struct HotbarBlockModel {
    pub slot_index: usize,
}

/// Marker for the hotbar render camera
#[derive(Component)]
pub struct HotbarRenderCamera;

const HOTBAR_RENDER_LAYER: u8 = 1;

/// Create small 3D block mesh for hotbar display (isometric view like Minecraft)
fn create_hotbar_block_mesh(block_id: BlockId, block_registry: &BlockRegistry) -> Option<Mesh> {
    let block_type = block_registry.get_block(block_id)?;

    // Create a small cube showing 3 visible faces (top, front-right, front-left)
    // Similar to Minecraft's hotbar block rendering
    let size = 0.5; // Small block size for hotbar

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let textures = &block_type.properties.textures;
    let tints = &block_type.properties.tint_colors;

    // Top face (visible)
    let top_coord = textures.get_coords(BlockFace::Top);
    let top_tint = tints.top.unwrap_or((1.0, 1.0, 1.0));
    let u_min = top_coord.u as f32 / 16.0;
    let v_min = top_coord.v as f32 / 16.0;
    let u_max = (top_coord.u + 1) as f32 / 16.0;
    let v_max = (top_coord.v + 1) as f32 / 16.0;

    let base_idx = positions.len() as u32;
    positions.extend_from_slice(&[
        [-size, size, -size],
        [size, size, -size],
        [size, size, size],
        [-size, size, size],
    ]);
    normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [u_min, v_max],
        [u_max, v_max],
        [u_max, v_min],
        [u_min, v_min],
    ]);
    colors.extend_from_slice(&[[top_tint.0, top_tint.1, top_tint.2, 1.0]; 4]);
    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);

    // Front-right face (South)
    let south_coord = textures.get_coords(BlockFace::South);
    let south_tint = tints.south.unwrap_or((1.0, 1.0, 1.0));
    let u_min = south_coord.u as f32 / 16.0;
    let v_min = south_coord.v as f32 / 16.0;
    let u_max = (south_coord.u + 1) as f32 / 16.0;
    let v_max = (south_coord.v + 1) as f32 / 16.0;

    let base_idx = positions.len() as u32;
    positions.extend_from_slice(&[
        [size, -size, size],
        [size, size, size],
        [size, size, -size],
        [size, -size, -size],
    ]);
    normals.extend_from_slice(&[[1.0, 0.0, 0.0]; 4]);
    uvs.extend_from_slice(&[
        [u_min, v_max],
        [u_min, v_min],
        [u_max, v_min],
        [u_max, v_max],
    ]);
    colors.extend_from_slice(&[[south_tint.0, south_tint.1, south_tint.2, 1.0]; 4]);
    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);

    // Front-left face (West)
    let west_coord = textures.get_coords(BlockFace::West);
    let west_tint = tints.west.unwrap_or((1.0, 1.0, 1.0));
    let u_min = west_coord.u as f32 / 16.0;
    let v_min = west_coord.v as f32 / 16.0;
    let u_max = (west_coord.u + 1) as f32 / 16.0;
    let v_max = (west_coord.v + 1) as f32 / 16.0;

    let base_idx = positions.len() as u32;
    positions.extend_from_slice(&[
        [-size, -size, -size],
        [-size, size, -size],
        [size, size, -size],
        [size, -size, -size],
    ]);
    normals.extend_from_slice(&[[0.0, 0.0, -1.0]; 4]);
    uvs.extend_from_slice(&[
        [u_max, v_max],
        [u_max, v_min],
        [u_min, v_min],
        [u_min, v_max],
    ]);
    colors.extend_from_slice(&[[west_tint.0, west_tint.1, west_tint.2, 1.0]; 4]);
    indices.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2, base_idx, base_idx + 2, base_idx + 3]);

    Some(
        Mesh::new(bevy::render::mesh::PrimitiveTopology::TriangleList, bevy::render::render_asset::RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
            .with_inserted_indices(bevy::render::mesh::Indices::U32(indices))
    )
}

/// Update hotbar block models when inventory changes
pub fn update_hotbar_block_models(
    mut commands: Commands,
    inventory: Res<PlayerInventory>,
    block_registry: Res<BlockRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<VoxelExtendedMaterial>>,
    asset_server: Res<AssetServer>,
    existing_models: Query<(Entity, &HotbarBlockModel)>,
    slot_query: Query<&HotbarSlot>,
) {
    if !inventory.is_changed() {
        return;
    }

    // Remove old models
    for (entity, _) in existing_models.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Create new models for each slot with a block
    for slot in slot_query.iter() {
        if let Some(block_id) = inventory.hotbar[slot.slot_index] {
            if let Some(mesh) = create_hotbar_block_mesh(block_id, &block_registry) {
                // Position based on slot index (centered in each hotbar slot)
                let slot_x = (slot.slot_index as f32 - 4.0) * 54.0; // 50px slot + 4px padding

                // Spawn 3D block model
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(VoxelExtendedMaterial {
                        base: StandardMaterial {
                            base_color_texture: Some(asset_server.load("mods/core/textures/atlas.png")),
                            ..default()
                        },
                        extension: (),
                    })),
                    Transform::from_xyz(slot_x, -400.0, 0.0) // Position in hotbar space
                        .with_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.785, 0.0)), // Isometric rotation
                    RenderLayers::layer(HOTBAR_RENDER_LAYER),
                    HotbarBlockModel { slot_index: slot.slot_index },
                ));
            }
        }
    }
}
