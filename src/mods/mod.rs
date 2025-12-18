mod mod_trait;
mod vanilla;

pub use mod_trait::GameMod;
pub use vanilla::VanillaMod;

use bevy::prelude::*;
use crate::assets::AssetManager;
use crate::blocks::BlockRegistry;

/// Resource that holds all registered mods
#[derive(Resource, Default)]
pub struct ModRegistry {
    mods: Vec<Box<dyn GameMod>>,
}

impl ModRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mod
    pub fn register_mod(&mut self, game_mod: Box<dyn GameMod>) {
        info!("Registering mod: {}", game_mod.id());
        self.mods.push(game_mod);
    }

    /// Get all registered mods
    pub fn mods(&self) -> &[Box<dyn GameMod>] {
        &self.mods
    }
}

/// System to initialize all mods during startup
pub fn initialize_mods(
    mod_registry: Res<ModRegistry>,
    mut block_registry: ResMut<BlockRegistry>,
    mut asset_manager: ResMut<AssetManager>,
    mut images: ResMut<Assets<Image>>,
) {
    info!("Initializing {} mods", mod_registry.mods.len());

    for game_mod in mod_registry.mods() {
        info!("Initializing mod: {} v{}", game_mod.id(), game_mod.version());

        // Load mod's texture atlas
        asset_manager.load_mod_texture_atlas(
            game_mod.id(),
            &mut images,
            game_mod.get_embedded_texture_atlas(),
        );

        // Register mod's blocks
        game_mod.register_blocks(&mut block_registry);
    }

    info!("All mods initialized. Total blocks: {}", block_registry.block_count());
}

/// Plugin for the mod system
pub struct ModPlugin;

impl Plugin for ModPlugin {
    fn build(&self, app: &mut App) {
        // Initialize registries
        app.init_resource::<BlockRegistry>();
        app.init_resource::<AssetManager>();

        // Create mod registry and register core mods
        let mut mod_registry = ModRegistry::new();
        mod_registry.register_mod(Box::new(VanillaMod));

        app.insert_resource(mod_registry);

        // Run mod initialization at startup
        app.add_systems(Startup, initialize_mods);
    }
}
