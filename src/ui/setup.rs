use bevy::prelude::*;
use crate::components::{PauseMenu, ResumeButton, ExitButton, Hotbar, HotbarSlot, HotbarSlotIcon, HotbarSlotText, SurvivalBars, HeartBackground, HeartIcon, HungerBackground, HungerIcon};
use crate::assets::{AssetManager, IconsTextureHandle};

pub fn setup_pause_menu(mut commands: Commands) {
    // Create pause menu container (initially hidden)
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden,
            PauseMenu,
        ))
        .with_children(|parent| {
            // Menu panel
            parent
                .spawn(Node {
                    width: Val::Px(400.0),
                    height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|parent| {
                    // Title
                    parent.spawn((
                        Text::new("PAUSED"),
                        TextFont {
                            font_size: 60.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ));

                    // Resume button
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(250.0),
                                height: Val::Px(65.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            ResumeButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Resume"),
                                TextFont {
                                    font_size: 32.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });

                    // Exit button
                    parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(250.0),
                                height: Val::Px(65.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            ExitButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Exit Game"),
                                TextFont {
                                    font_size: 32.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                            ));
                        });
                });
        });
}

pub fn setup_hotbar(mut commands: Commands) {
    // Minecraft-style hotbar sizing
    // At 1080p, Minecraft's hotbar is roughly 364 pixels wide (9 slots * ~40px each)
    // We'll use slightly larger slots for better visibility
    const SLOT_SIZE: f32 = 64.0;
    const ICON_SIZE: f32 = 48.0; // Match the isometric icon size
    const SLOT_PADDING: f32 = 2.0;
    const HOTBAR_BOTTOM: f32 = 10.0;

    // Hotbar container - full width row at bottom to enable centering
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                position_type: PositionType::Absolute,
                bottom: Val::Px(HOTBAR_BOTTOM),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Hotbar,
        ))
        .with_children(|parent| {
            // Inner container for slots with proper spacing
            parent.spawn(Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(SLOT_PADDING),
                ..default()
            })
            .with_children(|slots_parent| {
                // Create 9 hotbar slots
                for i in 0..9 {
                    slots_parent.spawn((
                        Node {
                            width: Val::Px(SLOT_SIZE),
                            height: Val::Px(SLOT_SIZE),
                            border: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BorderColor(Color::srgb(0.3, 0.3, 0.3)),
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        HotbarSlot { slot_index: i },
                    ))
                    .with_children(|slot_parent| {
                        // Add ImageNode for block texture (initially hidden)
                        // The texture and visibility will be set by the update_hotbar_icons system
                        slot_parent.spawn((
                            ImageNode::default(),
                            Node {
                                width: Val::Px(ICON_SIZE),
                                height: Val::Px(ICON_SIZE),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            Visibility::Hidden,
                            HotbarSlotIcon { slot_index: i },
                        ));

                        // Add text label for items without textures (initially hidden)
                        slot_parent.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            Visibility::Hidden,
                            HotbarSlotText { slot_index: i },
                        ));
                    });
                }
            });
        });
}

/// Setup health and hunger bars above the hotbar (survival mode only)
pub fn setup_survival_bars(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut icons_handle: ResMut<IconsTextureHandle>,
) {
    // Load icons texture if not already loaded
    if icons_handle.handle.is_none() {
        icons_handle.handle = AssetManager::load_icons_texture(&mut images);
    }

    let Some(texture) = icons_handle.handle.clone() else {
        warn!("Icons texture not loaded, survival bars will not display");
        return;
    };

    // Position above the hotbar
    // Hearts on the left, hunger on the right (Minecraft style)
    const BAR_BOTTOM: f32 = 84.0; // Above hotbar (10 + 64 + 10 spacing)

    // Icons.png is 256x256, each icon is 9x9 pixels
    // Scale up 2x for visibility (18x18 display size)
    const ATLAS_SIZE: f32 = 256.0;
    const ICON_PIXELS: f32 = 9.0;
    const ICON_DISPLAY_SIZE: f32 = 18.0;
    const ICON_GAP: f32 = 0.0; // No gap - icons include their own spacing

    // UV coordinates for icons (in pixels, will convert to 0-1 range)
    // Hearts row: v=0
    const HEART_BG_U: f32 = 16.0;      // Black outline background
    const HEART_FULL_U: f32 = 52.0;    // Full red heart (16 + 36)
    const HEART_HALF_U: f32 = 61.0;    // Half heart (16 + 36 + 9)
    const HEART_V: f32 = 0.0;

    // Hunger row: v=27
    const HUNGER_BG_U: f32 = 16.0;     // Black outline background
    const HUNGER_FULL_U: f32 = 52.0;   // Full haunch (16 + 36)
    const HUNGER_HALF_U: f32 = 61.0;   // Half haunch (16 + 45)
    const HUNGER_V: f32 = 27.0;

    // Helper to create rect from pixel coordinates (Bevy's ImageNode.rect uses pixel coords)
    let make_rect = |u: f32, v: f32| -> Rect {
        Rect::new(u, v, u + ICON_PIXELS, v + ICON_PIXELS)
    };

    // Container for both health and hunger bars
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                position_type: PositionType::Absolute,
                bottom: Val::Px(BAR_BOTTOM),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            Visibility::Hidden, // Hidden by default (shown in survival mode)
            SurvivalBars,
        ))
        .with_children(|parent| {
            // Inner container with fixed width matching hotbar
            parent.spawn(Node {
                width: Val::Px(590.0), // 9 slots * 64px + gaps
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            })
            .with_children(|bar_parent| {
                // Health bar (hearts) - left side
                bar_parent.spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(ICON_GAP),
                    ..default()
                })
                .with_children(|hearts_parent| {
                    // 10 heart icons (each with background + foreground)
                    for i in 0..10 {
                        // Container for stacked background + foreground
                        hearts_parent.spawn(Node {
                            width: Val::Px(ICON_DISPLAY_SIZE),
                            height: Val::Px(ICON_DISPLAY_SIZE),
                            ..default()
                        })
                        .with_children(|icon_parent| {
                            // Background (black outline)
                            icon_parent.spawn((
                                ImageNode {
                                    image: texture.clone(),
                                    rect: Some(make_rect(HEART_BG_U, HEART_V)),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(ICON_DISPLAY_SIZE),
                                    height: Val::Px(ICON_DISPLAY_SIZE),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                HeartBackground { index: i },
                            ));

                            // Foreground (full heart - will be swapped for half/empty)
                            icon_parent.spawn((
                                ImageNode {
                                    image: texture.clone(),
                                    rect: Some(make_rect(HEART_FULL_U, HEART_V)),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(ICON_DISPLAY_SIZE),
                                    height: Val::Px(ICON_DISPLAY_SIZE),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                HeartIcon { index: i },
                            ));
                        });
                    }
                });

                // Hunger bar (drumsticks) - right side
                bar_parent.spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::RowReverse, // Right-to-left like Minecraft
                    column_gap: Val::Px(ICON_GAP),
                    ..default()
                })
                .with_children(|hunger_parent| {
                    // 10 hunger icons (reversed order for right-to-left fill)
                    for i in 0..10 {
                        // Container for stacked background + foreground
                        hunger_parent.spawn(Node {
                            width: Val::Px(ICON_DISPLAY_SIZE),
                            height: Val::Px(ICON_DISPLAY_SIZE),
                            ..default()
                        })
                        .with_children(|icon_parent| {
                            // Background (black outline)
                            icon_parent.spawn((
                                ImageNode {
                                    image: texture.clone(),
                                    rect: Some(make_rect(HUNGER_BG_U, HUNGER_V)),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(ICON_DISPLAY_SIZE),
                                    height: Val::Px(ICON_DISPLAY_SIZE),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                HungerBackground { index: i },
                            ));

                            // Foreground (full haunch - will be swapped for half/empty)
                            icon_parent.spawn((
                                ImageNode {
                                    image: texture.clone(),
                                    rect: Some(make_rect(HUNGER_FULL_U, HUNGER_V)),
                                    ..default()
                                },
                                Node {
                                    width: Val::Px(ICON_DISPLAY_SIZE),
                                    height: Val::Px(ICON_DISPLAY_SIZE),
                                    position_type: PositionType::Absolute,
                                    ..default()
                                },
                                HungerIcon { index: i },
                            ));
                        });
                    }
                });
            });
        });
}
