# VoxelCraft Asset System

## Overview

VoxelCraft features a flexible mod-specific asset loading system with resource pack support, allowing you to customize textures without modifying mod code.

## Directory Structure

```
voxelcraft/
├── assets/
│   └── mods/
│       ├── core/              # Core/vanilla mod
│       │   └── textures/
│       │       └── atlas.png  # Texture atlas for core blocks
│       └── mymod/             # Example custom mod
│           └── textures/
│               └── atlas.png  # Texture atlas for custom mod
└── resource_packs/            # Optional resource pack overrides
    └── hd_textures/
        └── mods/
            └── core/
                └── textures/
                    └── atlas.png  # HD override for core textures
```

## How It Works

### Mod Asset Loading

When a mod is initialized, the AssetManager loads its texture atlas using this priority order:

1. **Resource Packs** (highest priority) - Checks all registered resource packs in order
   - `resource_packs/{pack_name}/mods/{mod_id}/textures/atlas.png`

2. **Mod Asset Folder** - The mod's dedicated asset directory
   - `assets/mods/{mod_id}/textures/atlas.png`

3. **Embedded Assets** (fallback) - Assets compiled into the executable
   - Defined via `get_embedded_texture_atlas()` in the mod trait

4. **Fallback Texture** - If none of the above are found, a magenta/black checkerboard is used

### Texture Atlas Format

- Texture atlases should be **16x16 grids** (typically 256x256 pixels for 16px textures)
- Each cell represents one block face texture
- Coordinates are specified using `AtlasCoord::new(x, y)` where x and y are grid positions (0-15)

Example from Minecraft 1.5 terrain.png:
- (0, 0) = Grass top
- (1, 0) = Stone
- (2, 0) = Dirt
- (3, 0) = Grass side
- (1, 1) = Bedrock

## Creating a Mod with Custom Textures

### Option 1: External Asset File (Recommended for Development)

1. Create your mod directory:
   ```
   assets/mods/mymod/textures/
   ```

2. Create your `atlas.png` (256x256, 16x16 grid)

3. Implement your mod:
   ```rust
   impl GameMod for MyMod {
       fn id(&self) -> &str {
           "mymod"
       }

       fn register_blocks(&self, registry: &mut BlockRegistry) {
           let my_block = BlockType::builder("mymod:cool_block", "Cool Block")
               .textures(BlockTextures::uniform(AtlasCoord::new(0, 0)))
               .build();
           registry.register_block(my_block).unwrap();
       }
   }
   ```

### Option 2: Embedded Assets (for Distribution)

1. Place your atlas in your mod's directory
2. Embed it in your mod:
   ```rust
   impl GameMod for MyMod {
       fn id(&self) -> &str {
           "mymod"
       }

       fn get_embedded_texture_atlas(&self) -> Option<&'static [u8]> {
           Some(include_bytes!("../assets/my_atlas.png"))
       }

       fn register_blocks(&self, registry: &mut BlockRegistry) {
           // ... register blocks
       }
   }
   ```

## Creating Resource Packs

Resource packs allow users to override mod textures without changing the mod itself.

1. Create a resource pack directory:
   ```
   resource_packs/my_pack/mods/core/textures/
   ```

2. Add your custom `atlas.png`

3. Register the resource pack (in your game configuration):
   ```rust
   asset_manager.add_resource_pack(PathBuf::from("resource_packs/my_pack"));
   ```

4. Resource packs are checked in reverse order of registration (last registered = highest priority)

### Important: Resource Pack Completeness

**Resource packs provide complete atlas replacement, not per-texture fallthrough.**

- If a resource pack provides an atlas for a mod, that entire atlas is used
- There is NO fallthrough to lower-priority packs for missing textures within an atlas
- Each atlas.png must be a complete 16x16 texture grid

**Example:**
- If `hd_pack/mods/core/textures/atlas.png` exists, it replaces the entire core texture atlas
- Missing textures in that atlas will show as black/transparent, not fall back to vanilla
- This is intentional: it makes resource packs predictable and easier to debug

**Best Practice:**
- Start with a copy of the original mod's atlas
- Modify only the textures you want to change
- Keep the atlas complete to avoid missing textures

## Block Texture Mapping

Blocks can have different textures on different faces:

```rust
// Uniform texture (all faces the same)
BlockTextures::uniform(AtlasCoord::new(1, 0))

// Different top/bottom and sides (like grass blocks)
BlockTextures::top_bottom_sides(
    AtlasCoord::new(0, 0),  // Top
    AtlasCoord::new(2, 0),  // Bottom
    AtlasCoord::new(3, 0),  // All sides
)

// Complete control over each face
BlockTextures {
    top: AtlasCoord::new(0, 0),
    bottom: AtlasCoord::new(2, 0),
    north: AtlasCoord::new(3, 0),
    south: AtlasCoord::new(3, 0),
    east: AtlasCoord::new(3, 0),
    west: AtlasCoord::new(3, 0),
}
```

## Benefits

- **Mod Isolation**: Each mod has its own texture atlas, preventing conflicts
- **Easy Customization**: Users can create resource packs without touching code
- **Flexible Distribution**: Mods can embed textures or load them externally
- **Development Friendly**: Quick iteration by editing external files
- **HD Texture Support**: Resource packs can provide higher resolution textures

## Future Enhancements

Planned features:
- Hot-reloading of resource packs
- Multiple atlas support per mod
- Animated textures
- Texture pack priority UI
- Mod-specific resource pack dependencies
