use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use crate::components::{HotbarSlot, HotbarSlotIcon, HotbarSlotText};
use crate::resources::{PlayerInventory, HotbarItem};
use crate::blocks::BlockRegistry;
use crate::assets::AssetManager;
use crate::rendering::{render_isometric_cube, IsometricIconCache};

/// Update hotbar slot visuals based on selected slot
pub fn update_hotbar_selection(
    inventory: Res<PlayerInventory>,
    mut slot_query: Query<(&HotbarSlot, &mut BorderColor, &mut BackgroundColor)>,
) {
    if !inventory.is_changed() {
        return;
    }

    for (slot, mut border_color, mut bg_color) in slot_query.iter_mut() {
        if slot.slot_index == inventory.selected_slot {
            // Selected slot - white border, lighter background
            *border_color = BorderColor(Color::srgb(1.0, 1.0, 1.0));
            *bg_color = BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.7));
        } else {
            // Unselected slot - gray border, darker background
            *border_color = BorderColor(Color::srgb(0.3, 0.3, 0.3));
            *bg_color = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5));
        }
    }
}

/// Handle mouse wheel to select hotbar slots
pub fn hotbar_mouse_wheel_selection(
    mut scroll_events: EventReader<MouseWheel>,
    mut inventory: ResMut<PlayerInventory>,
) {
    for event in scroll_events.read() {
        // Scroll up = negative delta = move left (decrease slot)
        // Scroll down = positive delta = move right (increase slot)
        if event.y > 0.1 {
            inventory.scroll_selection(-1);
        } else if event.y < -0.1 {
            inventory.scroll_selection(1);
        }
    }
}

/// Handle number keys 1-9 to select hotbar slots
pub fn hotbar_number_key_selection(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<PlayerInventory>,
) {
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if keyboard_input.just_pressed(key) {
            inventory.select_slot(slot);
            break;
        }
    }
}

/// Update hotbar slot icons to show isometric block cubes
pub fn update_hotbar_icons(
    inventory: Res<PlayerInventory>,
    block_registry: Res<BlockRegistry>,
    asset_manager: Res<AssetManager>,
    mut icon_cache: ResMut<IsometricIconCache>,
    mut images: ResMut<Assets<Image>>,
    mut icon_query: Query<(&HotbarSlotIcon, &mut ImageNode, &mut Visibility)>,
    mut text_query: Query<(&HotbarSlotText, &mut Text, &mut Visibility), Without<HotbarSlotIcon>>,
) {
    // Get the texture atlas for the core mod
    let Some(texture_handle) = asset_manager.get_mod_texture_atlas("core") else {
        return;
    };

    // Only update when inventory changes
    if !inventory.is_changed() {
        return;
    }

    // First pass: collect block IDs that need isometric icons generated
    let mut blocks_to_render: Vec<crate::blocks::BlockId> = Vec::new();
    for (icon, _, _) in icon_query.iter() {
        if let Some(HotbarItem::Block(block_id)) = inventory.hotbar[icon.slot_index] {
            if icon_cache.get(block_id).is_none() {
                if !blocks_to_render.contains(&block_id) {
                    blocks_to_render.push(block_id);
                }
            }
        }
    }

    // Generate isometric icons for blocks that don't have them cached
    if !blocks_to_render.is_empty() {
        // Get the atlas image data (needed for rendering isometric cubes)
        if let Some(atlas_image) = images.get(&texture_handle) {
            let atlas_data = atlas_image.data.clone();
            let width = atlas_image.width();
            let height = atlas_image.height();

            // Now we can drop the immutable borrow and add new images
            for block_id in blocks_to_render {
                if let Some(isometric_image) = render_isometric_cube(
                    block_id,
                    &block_registry,
                    &atlas_data,
                    width,
                    height,
                ) {
                    let handle = images.add(isometric_image);
                    icon_cache.insert(block_id, handle);
                }
            }
        }
    }

    // Second pass: update the UI icons (for blocks)
    for (icon, mut image_node, mut visibility) in icon_query.iter_mut() {
        match inventory.hotbar[icon.slot_index] {
            Some(HotbarItem::Block(block_id)) => {
                if let Some(cached_handle) = icon_cache.get(block_id) {
                    image_node.image = cached_handle.clone();
                    image_node.texture_atlas = None;
                    image_node.color = Color::WHITE;
                    *visibility = Visibility::Inherited;
                }
            }
            Some(HotbarItem::Item(_)) => {
                // Items use text labels, hide the image icon
                *visibility = Visibility::Hidden;
            }
            None => {
                // Empty slot - hide the icon
                *visibility = Visibility::Hidden;
            }
        }
    }

    // Third pass: update the text labels (for items without textures)
    for (text_slot, mut text, mut visibility) in text_query.iter_mut() {
        match inventory.hotbar[text_slot.slot_index] {
            Some(HotbarItem::Item(item_id)) => {
                // Show item name as placeholder text
                **text = item_id.name().to_string();
                *visibility = Visibility::Inherited;
            }
            _ => {
                // Blocks and empty slots don't need text labels
                *visibility = Visibility::Hidden;
            }
        }
    }
}
