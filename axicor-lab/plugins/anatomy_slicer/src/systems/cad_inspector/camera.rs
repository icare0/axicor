use crate::domain::{AnatomySlicerState, CadCameraState, ShardCadEntity};
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::view::RenderLayers;
use layout_api::PluginInput;
use node_editor::domain::BrainTopologyGraph;

pub fn spawn_cad_camera_system(
    mut commands: Commands,
    query: Query<&AnatomySlicerState>,
    cameras: Query<Entity, With<CadCameraState>>,
    graph: Res<BrainTopologyGraph>,
) {
    if !cameras.is_empty() {
        return;
    }

    let Some(state) = query.iter().find(|s| s.active_zone.is_some()) else {
        return;
    };
    let Some(handle) = &state.shard_rtt else {
        return;
    };

    let mut w = 32.0;
    let mut d = 32.0;
    let mut h = 32.0;
    if let Some(ref shard_name) = state.active_zone {
        if let Some(active_path) = &graph.active_path {
            if let Some(session) = graph.sessions.get(active_path) {
                if let Some(anatomy) = session.shard_anatomies.get(shard_name) {
                    w = anatomy.w.clamp(1.0, 10000.0);
                    d = anatomy.d.clamp(1.0, 10000.0);
                    h = anatomy.h.clamp(1.0, 10000.0);
                }
            }
        }
    }

    let cam_dist = (w.max(h).max(d) * 1.8).clamp(20.0, 15000.0);
    let cam_state = CadCameraState {
        target: Vec3::ZERO,
        radius: cam_dist,
        alpha: std::f32::consts::PI / 4.0,
        beta: 0.5,
    };
    let rotation = Quat::from_euler(EulerRot::YXZ, cam_state.alpha, cam_state.beta, 0.0);
    let cam_offset = rotation * Vec3::new(0.0, 0.0, cam_state.radius);

    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                target: RenderTarget::Image(handle.clone()),
                clear_color: ClearColorConfig::Custom(Color::rgb(0.10, 0.11, 0.12)),
                ..default()
            },
            projection: Projection::Perspective(PerspectiveProjection {
                far: 50000.0,
                ..default()
            }),
            transform: Transform::from_translation(cam_offset).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        RenderLayers::layer(2),
        ShardCadEntity,
        cam_state,
    ));

    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 5000.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_translation(cam_offset * 1.5)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        RenderLayers::layer(2),
        ShardCadEntity,
    ));
    info!("[Optics] Orbital camera and lighting spawned");
}

pub fn sync_camera_aspect_system(
    query: Query<&AnatomySlicerState>,
    mut cameras: Query<&mut Projection, With<CadCameraState>>,
) {
    let Some(state) = query.iter().find(|s| s.active_zone.is_some()) else {
        return;
    };
    if state.shard_rtt.is_none()
        || state.cad_viewport_size.x <= 10.0
        || state.cad_viewport_size.y <= 10.0
    {
        return;
    }

    let target_aspect = state.cad_viewport_size.x / state.cad_viewport_size.y;
    for mut proj in cameras.iter_mut() {
        if let Projection::Perspective(ref mut p) = *proj {
            if (p.aspect_ratio - target_aspect).abs() > 0.001 {
                p.aspect_ratio = target_aspect;
            }
        }
    }
}

pub fn cad_camera_control_system(
    ui_states: Query<(&AnatomySlicerState, &PluginInput)>,
    mut cameras: Query<(&mut Transform, &mut CadCameraState), With<ShardCadEntity>>,
) {
    let Some((state, input)) = ui_states.iter().find(|(s, _)| s.active_zone.is_some()) else {
        return;
    };

    // DOD FIX: Slicer captures cursor in the center, ignore edge artifacts
    if input.local_cursor.x < 20.0 {
        return;
    }
    let right_limit = state.cad_viewport_size.x - 20.0;
    if input.local_cursor.x > right_limit {
        return;
    }

    for (mut transform, mut cam) in cameras.iter_mut() {
        let is_dragging = input.is_secondary_pressed;
        let is_scrolling = input.scroll_delta.abs() > 0.0;
        if !is_dragging && !is_scrolling {
            continue;
        }

        if is_scrolling {
            let tick = input.scroll_delta.signum();
            cam.radius -= tick * 0.15 * cam.radius;
            cam.radius = cam.radius.max(5.0);
        }

        if input.is_secondary_pressed {
            cam.alpha -= input.cursor_delta.x * 0.005;
            cam.beta -= input.cursor_delta.y * 0.005;
            cam.beta = cam.beta.clamp(
                -std::f32::consts::PI / 2.0 + 0.01,
                std::f32::consts::PI / 2.0 - 0.01,
            );
        }

        let rotation = Quat::from_euler(EulerRot::YXZ, cam.alpha, cam.beta, 0.0);
        let offset = rotation * Vec3::new(0.0, 0.0, cam.radius);
        transform.translation = cam.target + offset;
        transform.look_at(cam.target, Vec3::Y);
    }
}
