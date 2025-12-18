use super::mod_trait::GameMod;
use crate::blocks::{BlockRegistry, BlockType, BlockTextures, AtlasCoord, FaceTints};

/// The core/vanilla mod that provides basic Minecraft-like blocks
pub struct VanillaMod;

impl GameMod for VanillaMod {
    fn id(&self) -> &str {
        "core"
    }

    fn name(&self) -> &str {
        "VoxelCraft Core"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn get_embedded_texture_atlas(&self) -> Option<&'static [u8]> {
        // Provide the embedded texture atlas from mod-specific location
        Some(include_bytes!("../../assets/mods/core/textures/atlas.png"))
    }

    fn register_blocks(&self, registry: &mut BlockRegistry) {
        // Note: Air is already registered in the BlockRegistry default constructor

        // Minecraft 1.5 terrain.png atlas coordinates:
        // (0, 0) = Grass top
        // (1, 0) = Stone
        // (2, 0) = Dirt
        // (3, 0) = Grass side
        // (1, 1) = Bedrock

        // Stone - uniform gray texture at (1, 0)
        let stone = BlockType::builder("core:stone", "Stone")
            .solid(true)
            .transparent(false)
            .textures(BlockTextures::uniform(AtlasCoord::new(1, 0)))
            .build();
        registry.register_block(stone)
            .expect("Failed to register stone block");

        // Dirt - uniform brown texture at (2, 0)
        let dirt = BlockType::builder("core:dirt", "Dirt")
            .solid(true)
            .transparent(false)
            .textures(BlockTextures::uniform(AtlasCoord::new(2, 0)))
            .build();
        registry.register_block(dirt)
            .expect("Failed to register dirt block");

        // Grass - grass top (0,0), dirt bottom (2,0), dirt sides (2,0) with grass overlay (6,2)
        // Top is grayscale and gets tinted with Minecraft's default grass color #7cbd6b
        // Sides use dirt texture with grayscale grass overlay that also gets tinted
        let grass = BlockType::builder("core:grass", "Grass Block")
            .solid(true)
            .transparent(false)
            .textures(BlockTextures::with_side_overlay(
                AtlasCoord::new(0, 0),  // Grass top (grayscale, gets tinted)
                AtlasCoord::new(2, 0),  // Dirt bottom (no tint)
                AtlasCoord::new(2, 0),  // Dirt sides (base texture)
                AtlasCoord::new(6, 2),  // Grass side overlay (grayscale, gets tinted)
            ))
            .tint_colors(FaceTints {
                top: Some((0.486, 0.741, 0.420)),  // Minecraft grass color #7cbd6b
                bottom: None,
                north: None,
                south: None,
                east: None,
                west: None,
            })
            .build();
        registry.register_block(grass)
            .expect("Failed to register grass block");

        // Bedrock - unbreakable base layer at (1, 1)
        let bedrock = BlockType::builder("core:bedrock", "Bedrock")
            .solid(true)
            .transparent(false)
            .textures(BlockTextures::uniform(AtlasCoord::new(1, 1)))
            .build();
        registry.register_block(bedrock)
            .expect("Failed to register bedrock block");
    }
}
