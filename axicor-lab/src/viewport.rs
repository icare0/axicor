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
    // Set order to 0. UI camera will be set to 1.
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
            base_color: Color::rgb(0.486, 0.565, 1.0), // #7c90ff
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
    let width = window.width();
    let height = window.height();
    let physical_width = window.physical_width();
    let physical_height = window.physical_height();

    if let Ok((gt, node)) = ui_query.get_single() {
        if let Ok(mut camera) = camera_query.get_single_mut() {
            let position = gt.translation();
            let size = node.size();

            if size.x <= 0.0 || size.y <= 0.0 {
                camera.is_active = false;
                return;
            }
            camera.is_active = true;

            // Calculate logical rect
            let logical_left = position.x - size.x / 2.0 + width / 2.0;
            let logical_top = height / 2.0 - (position.y + size.y / 2.0);
            
            // Convert to physical with rounding to be safe
            let mut x = (logical_left * scale_factor).round() as i32;
            let mut y = (logical_top * scale_factor).round() as i32;
            let mut w = (size.x * scale_factor).round() as i32;
            let mut h = (size.y * scale_factor).round() as i32;

            // Clamp to physical window boundaries to prevent wgpu validation errors
            x = x.clamp(0, physical_width as i32);
            y = y.clamp(0, physical_height as i32);
            
            // Ensure width and height are at least 1 and fit within the remaining space
            w = w.max(1).min(physical_width as i32 - x);
            h = h.max(1).min(physical_height as i32 - y);

            if w <= 0 || h <= 0 {
                camera.is_active = false;
                return;
            }

            camera.viewport = Some(Viewport {
                physical_position: UVec2::new(x as u32, y as u32),
                physical_size: UVec2::new(w as u32, h as u32),
                depth: 0.0..1.0,
            });
        }
    }
}
