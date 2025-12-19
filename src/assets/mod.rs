use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::utils::HashMap;
use std::path::PathBuf;

/// Handle to the GUI icons texture atlas (icons.png)
#[derive(Resource, Default)]
pub struct IconsTextureHandle {
    pub handle: Option<Handle<Image>>,
}

/// Manages texture atlases for different mods and resource packs
#[allow(dead_code)]
#[derive(Resource)]
pub struct AssetManager {
    /// Texture atlases loaded for each mod (mod_id -> texture handle)
    pub texture_atlases: HashMap<String, Handle<Image>>,
    /// Resource pack paths in priority order (highest priority first)
    pub resource_pack_paths: Vec<PathBuf>,
}

impl Default for AssetManager {
    fn default() -> Self {
        Self {
            texture_atlases: HashMap::new(),
            resource_pack_paths: Vec::new(),
        }
    }
}

#[allow(dead_code)]
impl AssetManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a texture atlas for a mod, checking resource packs first, then mod assets
    ///
    /// Search order:
    /// 1. Resource packs (in priority order)
    /// 2. assets/mods/{mod_id}/textures/atlas.png
    /// 3. Embedded mod assets (if provided)
    /// 4. Fallback magenta/black checkerboard
    ///
    /// Note: Resource packs provide complete replacement, not individual texture fallthrough.
    /// If a resource pack is missing a texture in its atlas, that's a bug in the resource pack.
    pub fn load_mod_texture_atlas(
        &mut self,
        mod_id: &str,
        images: &mut ResMut<Assets<Image>>,
        embedded_bytes: Option<&[u8]>,
    ) -> Handle<Image> {
        // Check if already loaded
        if let Some(handle) = self.texture_atlases.get(mod_id) {
            info!("Texture atlas for mod '{}' already loaded", mod_id);
            return handle.clone();
        }

        info!("Loading texture atlas for mod '{}'", mod_id);

        // Try to load from resource packs first
        for resource_pack_path in &self.resource_pack_paths {
            let atlas_path = resource_pack_path
                .join("mods")
                .join(mod_id)
                .join("textures")
                .join("atlas.png");

            if let Ok(bytes) = std::fs::read(&atlas_path) {
                info!("  -> Found in resource pack: {:?}", atlas_path);
                if let Some(handle) = Self::create_texture_from_bytes(&bytes, images) {
                    self.texture_atlases.insert(mod_id.to_string(), handle.clone());
                    return handle;
                }
            }
        }

        // Try to load from mod's asset folder
        let mod_asset_path = PathBuf::from("assets")
            .join("mods")
            .join(mod_id)
            .join("textures")
            .join("atlas.png");

        if let Ok(bytes) = std::fs::read(&mod_asset_path) {
            info!("  -> Found in mod assets: {:?}", mod_asset_path);
            if let Some(handle) = Self::create_texture_from_bytes(&bytes, images) {
                self.texture_atlases.insert(mod_id.to_string(), handle.clone());
                return handle;
            }
        }

        // Fall back to embedded bytes if provided
        if let Some(bytes) = embedded_bytes {
            info!("  -> Using embedded texture atlas");
            if let Some(handle) = Self::create_texture_from_bytes(bytes, images) {
                self.texture_atlases.insert(mod_id.to_string(), handle.clone());
                return handle;
            }
        }

        warn!("  -> No texture atlas found for mod '{}', using fallback checkerboard", mod_id);

        // Create a default 16x16 magenta/black checkerboard as fallback
        let handle = Self::create_fallback_texture(images);
        self.texture_atlases.insert(mod_id.to_string(), handle.clone());
        handle
    }

    /// Get the texture atlas handle for a mod
    pub fn get_mod_texture_atlas(&self, mod_id: &str) -> Option<Handle<Image>> {
        self.texture_atlases.get(mod_id).cloned()
    }

    /// Add a resource pack path (higher priority than existing packs)
    pub fn add_resource_pack(&mut self, path: PathBuf) {
        info!("Adding resource pack: {:?}", path);
        self.resource_pack_paths.insert(0, path);
    }

    /// Remove a resource pack and reload affected textures
    pub fn remove_resource_pack(&mut self, path: &PathBuf) {
        self.resource_pack_paths.retain(|p| p != path);
        // TODO: Reload textures when resource pack is removed
    }

    /// Create a texture from PNG bytes
    fn create_texture_from_bytes(bytes: &[u8], images: &mut ResMut<Assets<Image>>) -> Option<Handle<Image>> {
        use bevy::render::render_resource::TextureFormat;
        use bevy::image::{ImageSampler, ImageSamplerDescriptor, ImageFilterMode, ImageAddressMode};

        match image::load_from_memory(bytes) {
            Ok(dyn_img) => {
                let rgba = dyn_img.to_rgba8();
                let (width, height) = rgba.dimensions();

                let mut image = Image::new(
                    bevy::render::render_resource::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    rgba.into_raw(),
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );

                // Use nearest-neighbor filtering for pixel-art textures
                // Linear filtering causes tile bleeding in texture atlases at distance
                image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    label: None,
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    address_mode_w: ImageAddressMode::Repeat,
                    mag_filter: ImageFilterMode::Nearest,
                    min_filter: ImageFilterMode::Nearest,
                    mipmap_filter: ImageFilterMode::Nearest,
                    ..Default::default()
                });

                Some(images.add(image))
            }
            Err(e) => {
                error!("Failed to load texture from bytes: {}", e);
                None
            }
        }
    }

    /// Load the GUI icons texture atlas (icons.png)
    pub fn load_icons_texture(
        images: &mut ResMut<Assets<Image>>,
    ) -> Option<Handle<Image>> {
        let icons_path = PathBuf::from("assets")
            .join("mods")
            .join("core")
            .join("textures")
            .join("icons.png");

        if let Ok(bytes) = std::fs::read(&icons_path) {
            info!("Loading icons texture from {:?}", icons_path);
            Self::create_texture_from_bytes(&bytes, images)
        } else {
            warn!("Icons texture not found at {:?}", icons_path);
            None
        }
    }

    /// Create a fallback magenta/black checkerboard texture
    fn create_fallback_texture(images: &mut ResMut<Assets<Image>>) -> Handle<Image> {
        let size = 256; // 16x16 grid for a 16x16 atlas
        let mut data = Vec::with_capacity(size * size * 4);

        for y in 0..size {
            for x in 0..size {
                let tile_x = x / 16;
                let tile_y = y / 16;
                let is_magenta = (tile_x + tile_y) % 2 == 0;

                if is_magenta {
                    data.extend_from_slice(&[255, 0, 255, 255]); // Magenta
                } else {
                    data.extend_from_slice(&[0, 0, 0, 255]); // Black
                }
            }
        }

        let image = Image::new(
            bevy::render::render_resource::Extent3d {
                width: size as u32,
                height: size as u32,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );

        images.add(image)
    }
}
