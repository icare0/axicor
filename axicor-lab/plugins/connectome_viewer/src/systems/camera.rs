use bevy::prelude::*;
use crate::domain::ViewportCamera;
use layout_api::{PluginInput, PluginGeometry, PluginWindow};

pub fn viewport_camera_control_system(
    mut query: Query<(&mut Transform, &mut ViewportCamera, &PluginInput, &PluginGeometry, &mut Projection)>,
) {
    for (mut transform, mut cam, input, geom, mut projection) in query.iter_mut() {
        if geom.size.x > 0.0 && geom.size.y > 0.0 {
            if let Projection::Perspective(ref mut persp) = *projection {
                let current_aspect = geom.size.x / geom.size.y;
                if (persp.aspect_ratio - current_aspect).abs() > 0.001 {
                    persp.aspect_ratio = current_aspect;
                }
            }
        }

        // DOD FIX:   Ghost Mutations. 
        //       mut transform,    is_changed()  .
        let is_dragging = input.is_secondary_pressed || input.is_middle_pressed;
        let is_scrolling = input.scroll_delta.abs() > 0.0;

        if !is_dragging && !is_scrolling {
            continue;
        }

        if input.scroll_delta.abs() > 0.0 {
            let tick = input.scroll_delta.signum();
            cam.radius -= tick * 0.15 * cam.radius;
            cam.radius = cam.radius.clamp(0.1, 1000.0);
        }

        if input.is_secondary_pressed {
            cam.alpha -= input.cursor_delta.x * 0.005;
            cam.beta -= input.cursor_delta.y * 0.005;
            cam.beta = cam.beta.clamp(-std::f32::consts::PI / 2.0 + 0.01, std::f32::consts::PI / 2.0 - 0.01);
        }

        if input.is_middle_pressed {
            let right = transform.right();
            let up = transform.up();
            let pan_speed = cam.radius * 0.002;
            cam.target -= right * input.cursor_delta.x * pan_speed;
            cam.target += up * input.cursor_delta.y * pan_speed;
        }

        let rotation = Quat::from_euler(EulerRot::YXZ, cam.alpha, cam.beta, 0.0);
        let offset = rotation * Vec3::new(0.0, 0.0, cam.radius);
        
        transform.translation = cam.target + offset;
        transform.look_at(cam.target, Vec3::Y);
    }
}

pub fn attach_camera_to_viewport_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Without<Camera>>,
) {
    for (entity, window) in query.iter() {
        if window.plugin_id.starts_with(layout_api::DOMAIN_VIEWPORT) {
            // ,  WM (sync_plugin_geometry_system)  VRAM 
            if let Some(tex_handle) = &window.texture {
                commands.entity(entity).insert((
                    Camera3dBundle {
                        camera: Camera {
                            target: bevy::render::camera::RenderTarget::Image(tex_handle.clone()),
                            clear_color: ClearColorConfig::Custom(Color::rgb(0.117, 0.117, 0.125)),
                            ..default()
                        },
                        transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                        ..default()
                    },
                    ViewportCamera {
                        viewport: entity,
                        ..default()
                    },
                ));
            }
        }
    }
}

pub fn toggle_idle_cameras_system(
    window_query: Query<&layout_api::PluginWindow>,
    //  Ref<Transform>  NeuronInstances,    
    mut camera_query: Query<(Entity, &mut Camera, &crate::domain::ViewportCamera)>,
    mut commands: Commands,
) {
    for (cam_entity, mut camera, vp_cam) in camera_query.iter_mut() {
        // DOD FIX:   ID  (vp_cam.viewport),   cam_entity!
        if let Ok(_plugin_window) = window_query.get(vp_cam.viewport) {
            // :   ,    .  .
            if !camera.is_active {
                camera.is_active = true;
            }
        } else {
            //     .  -.
            commands.entity(cam_entity).despawn_recursive();
        }
    }
}
