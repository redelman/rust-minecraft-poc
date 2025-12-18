use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use super::block_type::BlockType;

/// Numeric ID for a block type (0 is always air)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct BlockId(pub u16);

impl BlockId {
    /// The air block (always ID 0)
    pub const AIR: BlockId = BlockId(0);

    pub fn is_air(self) -> bool {
        self == Self::AIR
    }
}

/// Global registry for all block types
/// This is a Bevy resource that mods can access to register new blocks
#[derive(Resource, Clone)]
pub struct BlockRegistry {
    /// Map from block ID to block type
    blocks: Vec<Option<BlockType>>,
    /// Map from string ID to numeric ID for lookups
    id_map: HashMap<String, BlockId>,
    /// Next available ID
    next_id: u16,
}

impl Default for BlockRegistry {
    fn default() -> Self {
        let mut registry = Self {
            blocks: vec![None; 65536], // u16 max
            id_map: HashMap::new(),
            next_id: 0,
        };

        // Register air block as ID 0
        registry.register_block(
            BlockType::builder("core:air", "Air")
                .solid(false)
                .transparent(true)
                .build()
        ).expect("Failed to register air block");

        registry
    }
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new block type and return its ID
    /// Returns an error if a block with this string ID already exists
    pub fn register_block(&mut self, block_type: BlockType) -> Result<BlockId, String> {
        let string_id = block_type.properties.id.clone();

        // Check if already registered
        if self.id_map.contains_key(&string_id) {
            return Err(format!("Block '{}' is already registered", string_id));
        }

        // Check if we've run out of IDs
        if self.next_id == u16::MAX {
            return Err("Block registry is full (65536 block types)".to_string());
        }

        let block_id = BlockId(self.next_id);
        self.next_id += 1;

        // Store the block type
        self.blocks[block_id.0 as usize] = Some(block_type);
        self.id_map.insert(string_id.clone(), block_id);

        info!("Registered block '{}' with ID {}", string_id, block_id.0);

        Ok(block_id)
    }

    /// Get a block type by its numeric ID
    pub fn get_block(&self, id: BlockId) -> Option<&BlockType> {
        self.blocks.get(id.0 as usize)?.as_ref()
    }

    /// Get a block type by its string ID
    pub fn get_block_by_string_id(&self, id: &str) -> Option<&BlockType> {
        let block_id = self.id_map.get(id)?;
        self.get_block(*block_id)
    }

    /// Get the numeric ID for a block by its string ID
    pub fn get_id(&self, string_id: &str) -> Option<BlockId> {
        self.id_map.get(string_id).copied()
    }

    /// Get the string ID for a block by its numeric ID (for world save/load)
    pub fn get_string_id(&self, id: BlockId) -> Option<&str> {
        self.get_block(id).map(|block| block.properties.id.as_str())
    }

    /// Get all registered block IDs
    pub fn get_all_ids(&self) -> Vec<BlockId> {
        (0..self.next_id).map(BlockId).collect()
    }

    /// Get the number of registered blocks
    pub fn block_count(&self) -> usize {
        self.next_id as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_block_registration() {
        let registry = BlockRegistry::new();
        assert_eq!(registry.block_count(), 1);
        assert!(registry.get_block(BlockId::AIR).is_some());
        assert_eq!(
            registry.get_block(BlockId::AIR).unwrap().properties.id,
            "core:air"
        );
    }

    #[test]
    fn test_register_and_get_block() {
        let mut registry = BlockRegistry::new();

        let stone = BlockType::builder("core:stone", "Stone").build();
        let id = registry.register_block(stone).unwrap();

        assert_eq!(registry.block_count(), 2);
        assert!(registry.get_block(id).is_some());
        assert_eq!(registry.get_id("core:stone"), Some(id));
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = BlockRegistry::new();

        let stone1 = BlockType::builder("core:stone", "Stone").build();
        let stone2 = BlockType::builder("core:stone", "Stone").build();

        assert!(registry.register_block(stone1).is_ok());
        assert!(registry.register_block(stone2).is_err());
    }
}
