use bevy::prelude::*;
use crate::blocks::BlockId;

/// Unique identifier for non-block items
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemId {
    Torch,
}

impl ItemId {
    /// Get the display name for this item
    pub fn name(&self) -> &'static str {
        match self {
            ItemId::Torch => "Torch",
        }
    }
}

/// Represents something that can be held in the inventory
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotbarItem {
    Block(BlockId),
    Item(ItemId),
}

#[derive(Resource)]
pub struct PlayerInventory {
    pub hotbar: [Option<HotbarItem>; 9],
    pub selected_slot: usize, // 0-8
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            hotbar: [None; 9],
            selected_slot: 0,
        }
    }
}

#[allow(dead_code)]
impl PlayerInventory {
    pub fn new_with_blocks(bedrock: BlockId, stone: BlockId, dirt: BlockId, grass: BlockId) -> Self {
        let mut hotbar: [Option<HotbarItem>; 9] = [None; 9];
        hotbar[0] = Some(HotbarItem::Block(bedrock));  // Slot 1 (index 0)
        hotbar[1] = Some(HotbarItem::Block(stone));    // Slot 2 (index 1)
        hotbar[2] = Some(HotbarItem::Block(dirt));     // Slot 3 (index 2)
        hotbar[3] = Some(HotbarItem::Block(grass));    // Slot 4 (index 3)
        // Torch implementation exists but is not added to hotbar until it's fully working

        Self {
            hotbar,
            selected_slot: 0,
        }
    }

    /// Get the selected item if it's a block
    pub fn get_selected_block(&self) -> Option<BlockId> {
        match self.hotbar[self.selected_slot] {
            Some(HotbarItem::Block(id)) => Some(id),
            _ => None,
        }
    }

    /// Get the selected item if it's a non-block item
    pub fn get_selected_item(&self) -> Option<ItemId> {
        match self.hotbar[self.selected_slot] {
            Some(HotbarItem::Item(id)) => Some(id),
            _ => None,
        }
    }

    /// Check if the torch is currently selected
    pub fn is_torch_selected(&self) -> bool {
        matches!(self.hotbar[self.selected_slot], Some(HotbarItem::Item(ItemId::Torch)))
    }

    pub fn select_slot(&mut self, slot: usize) {
        if slot < 9 {
            self.selected_slot = slot;
        }
    }

    pub fn scroll_selection(&mut self, delta: i32) {
        let new_slot = (self.selected_slot as i32 + delta).rem_euclid(9);
        self.selected_slot = new_slot as usize;
    }
}
