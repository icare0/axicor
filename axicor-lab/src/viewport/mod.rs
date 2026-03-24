pub mod mask;

use bevy::prelude::*;
use bevy::render::camera::Viewport;
use crate::layout::systems::ViewportContainer;

#[derive(Component)]
pub struct WorldCamera;

pub fn setup_3d_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                order: 0,
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        WorldCamera,
    ));

    // Cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::rgb(0.486, 0.565, 1.0),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

pub fn sync_camera_viewport(
    windows: Query<&Window>,
    ui_query: Query<(&GlobalTransform, &Node), With<ViewportContainer>>,
    mut camera_query: Query<&mut Camera, With<WorldCamera>>,
) {
    let window = windows.single();
    let scale_factor = window.resolution.scale_factor();
    let physical_width = window.physical_width();
    let physical_height = window.physical_height();

    if let Ok((gt, node)) = ui_query.get_single() {
        if let Ok(mut camera) = camera_query.get_single_mut() {
            let position = gt.translation();
            let size = node.size();

            // 1. Calculate logical corners in screen space (top-left is 0,0)
            let logical_left = position.x - size.x / 2.0 + window.width() / 2.0;
            let logical_top = window.height() / 2.0 - (position.y + size.y / 2.0);
            
            // 2. Convert to physical pixels with flooring to stay inside boundaries
            let x = (logical_left * scale_factor).floor() as u32;
            let y = (logical_top * scale_factor).floor() as u32;
            let mut w = (size.x * scale_factor).ceil() as u32;
            let mut h = (size.y * scale_factor).ceil() as u32;

            // 3. Strict clamping to physical window dimensions
            // Ensure x and y are within window
            let x = x.min(physical_width.saturating_sub(1));
            let y = y.min(physical_height.saturating_sub(1));
            
            // Ensure x + w and y + h do not exceed window dimensions
            w = w.min(physical_width.saturating_sub(x));
            h = h.min(physical_height.saturating_sub(y));

            // 4. Deactivate camera if viewport is too small to avoid wgpu errors
            if w == 0 || h == 0 {
                camera.is_active = false;
                return;
            }
            camera.is_active = true;

            camera.viewport = Some(Viewport {
                physical_position: UVec2::new(x, y),
                physical_size: UVec2::new(w, h),
                depth: 0.0..1.0,
            });
        }
    }
}
