use crate::blocks::BlockRegistry;

/// Trait that all mods must implement
pub trait GameMod: Send + Sync {
    /// Unique identifier for this mod (e.g., "core", "mymod")
    fn id(&self) -> &str;

    /// Display name for this mod
    fn name(&self) -> &str;

    /// Version string
    fn version(&self) -> &str;

    /// Called during startup to register blocks
    fn register_blocks(&self, registry: &mut BlockRegistry);

    /// Get embedded texture atlas bytes (optional)
    /// If None, the asset manager will look for atlas.png in the mod's asset folder
    fn get_embedded_texture_atlas(&self) -> Option<&'static [u8]> {
        None
    }

    /// Called during startup to register items (future implementation)
    fn register_items(&self, _registry: &mut ()) {
        // Default implementation does nothing
    }

    /// Called during startup to register entities (future implementation)
    fn register_entities(&self, _registry: &mut ()) {
        // Default implementation does nothing
    }
}
