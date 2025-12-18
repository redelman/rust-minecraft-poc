use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use noise::{NoiseFn, Perlin};

pub fn create_noise_texture(images: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    let size = 256;
    let perlin = Perlin::new(42);

    let mut data = Vec::with_capacity(size * size * 4);

    for y in 0..size {
        for x in 0..size {
            let nx = x as f64 / size as f64;
            let ny = y as f64 / size as f64;

            // Create layered noise for more interesting texture
            let noise_value =
                perlin.get([nx * 10.0, ny * 10.0]) * 0.5 +
                perlin.get([nx * 20.0, ny * 20.0]) * 0.25 +
                perlin.get([nx * 40.0, ny * 40.0]) * 0.125 +
                perlin.get([nx * 80.0, ny * 80.0]) * 0.0625;

            // Map noise to color range (vibrant stone/grass-like colors)
            let value = ((noise_value + 1.0) / 2.0).clamp(0.0, 1.0);

            // Create gradient from grass green to stone gray
            let r = (60.0 + value * 140.0) as u8;
            let g = (100.0 + value * 120.0) as u8;
            let b = (50.0 + value * 80.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255);
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

/// Create a simple daytime skybox texture (gradient blue sky)
pub fn create_skybox_texture(images: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    let width = 512;
    let height = 512;
    let mut data = Vec::with_capacity(width * height * 4);

    for y in 0..height {
        for _x in 0..width {
            // Create vertical gradient from horizon to sky
            let v = y as f32 / height as f32;

            // Simple blue gradient - actual sky color comes from clear_color
            // This texture is mostly for consistency, real color is dynamic
            let horizon_color = Vec3::new(0.75, 0.85, 0.95);
            let zenith_color = Vec3::new(0.3, 0.55, 0.95);

            let t = v.powf(0.7);
            let color = horizon_color * (1.0 - t) + zenith_color * t;

            let r = (color.x * 255.0).clamp(0.0, 255.0) as u8;
            let g = (color.y * 255.0).clamp(0.0, 255.0) as u8;
            let b = (color.z * 255.0).clamp(0.0, 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255);
        }
    }

    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    images.add(image)
}

/// Create a night sky texture with stars and a nebula
pub fn create_night_sky_texture(images: &mut ResMut<Assets<Image>>) -> Handle<Image> {
    use rand::{Rng, SeedableRng};
    use rand::rngs::StdRng;

    let width = 1024;
    let height = 512;
    let mut data = Vec::with_capacity(width * height * 4);

    let perlin = Perlin::new(12345);
    let mut rng = StdRng::seed_from_u64(42);

    // Pre-generate star positions
    let num_stars = 800;
    let mut stars: Vec<(usize, usize, f32)> = Vec::with_capacity(num_stars);
    for _ in 0..num_stars {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let brightness = rng.gen_range(0.3..1.0_f32);
        stars.push((x, y, brightness));
    }

    for y in 0..height {
        for x in 0..width {
            let u = x as f64 / width as f64;
            let v = y as f64 / height as f64;

            // Base dark sky color
            let mut r = 0.02_f32;
            let mut g = 0.02_f32;
            let mut b = 0.05_f32;

            // Add subtle nebula coloring using perlin noise
            let nebula1 = perlin.get([u * 3.0, v * 3.0]) as f32;
            let nebula2 = perlin.get([u * 5.0 + 10.0, v * 5.0 + 10.0]) as f32;
            let nebula3 = perlin.get([u * 8.0 + 20.0, v * 8.0 + 20.0]) as f32;

            // Purple/blue nebula tint
            let nebula_intensity = ((nebula1 * 0.5 + nebula2 * 0.3 + nebula3 * 0.2 + 0.5) * 0.15).max(0.0);
            r += nebula_intensity * 0.3;  // slight red/purple
            g += nebula_intensity * 0.1;
            b += nebula_intensity * 0.5;  // more blue

            // Check if this pixel is near a star
            for &(sx, sy, brightness) in &stars {
                let dx = (x as i32 - sx as i32).abs();
                let dy = (y as i32 - sy as i32).abs();

                if dx <= 1 && dy <= 1 {
                    // Star core
                    if dx == 0 && dy == 0 {
                        r += brightness;
                        g += brightness;
                        b += brightness;
                    } else {
                        // Star glow (dimmer for adjacent pixels)
                        let glow = brightness * 0.3;
                        r += glow;
                        g += glow;
                        b += glow * 1.1; // slight blue tint
                    }
                }
            }

            let r = (r * 255.0).clamp(0.0, 255.0) as u8;
            let g = (g * 255.0).clamp(0.0, 255.0) as u8;
            let b = (b * 255.0).clamp(0.0, 255.0) as u8;

            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255);
        }
    }

    let image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    images.add(image)
}
