use bevy::prelude::*;
use bevy::input::mouse::MouseButton;
use bevy::window::PrimaryWindow;
use crate::components::{Voxel, ClickText};

pub fn voxel_click_detection(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    voxel_query: Query<&GlobalTransform, With<Voxel>>,
    mut text_query: Query<(&mut Text, &mut ClickText)>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        let window = windows.single();

        if let Some(cursor_position) = window.cursor_position() {
            for (camera, camera_transform) in camera_query.iter() {
                // Cast a ray from the camera through the cursor position
                if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
                    // Simple ray-box intersection test for the voxel
                    for voxel_transform in voxel_query.iter() {
                        let voxel_pos = voxel_transform.translation();

                        // Simple distance-based check (for single voxel at origin)
                        if let Some(_) = ray_box_intersection(ray.origin, *ray.direction, voxel_pos, 0.5) {
                            // Voxel was clicked!
                            for (mut text, mut click_text) in text_query.iter_mut() {
                                **text = "Voxel was clicked!".to_string();
                                click_text.timer.reset();
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ray_box_intersection(ray_origin: Vec3, ray_dir: Vec3, box_center: Vec3, box_half_size: f32) -> Option<f32> {
    let box_min = box_center - Vec3::splat(box_half_size);
    let box_max = box_center + Vec3::splat(box_half_size);

    let mut tmin = (box_min.x - ray_origin.x) / ray_dir.x;
    let mut tmax = (box_max.x - ray_origin.x) / ray_dir.x;

    if tmin > tmax {
        std::mem::swap(&mut tmin, &mut tmax);
    }

    let mut tymin = (box_min.y - ray_origin.y) / ray_dir.y;
    let mut tymax = (box_max.y - ray_origin.y) / ray_dir.y;

    if tymin > tymax {
        std::mem::swap(&mut tymin, &mut tymax);
    }

    if tmin > tymax || tymin > tmax {
        return None;
    }

    if tymin > tmin {
        tmin = tymin;
    }

    if tymax < tmax {
        tmax = tymax;
    }

    let mut tzmin = (box_min.z - ray_origin.z) / ray_dir.z;
    let mut tzmax = (box_max.z - ray_origin.z) / ray_dir.z;

    if tzmin > tzmax {
        std::mem::swap(&mut tzmin, &mut tzmax);
    }

    if tmin > tzmax || tzmin > tmax {
        return None;
    }

    if tzmin > tmin {
        tmin = tzmin;
    }

    if tmin < 0.0 {
        return None;
    }

    Some(tmin)
}
