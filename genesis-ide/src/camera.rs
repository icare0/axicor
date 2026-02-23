use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

use crate::AppState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Running), spawn_camera)
            .add_systems(
                Update,
                (toggle_camera_mode, camera_movement, camera_look)
                    .chain()
                    .run_if(in_state(AppState::Running)),
            );
    }
}

/// Camera movement speed multiplier (changed by scroll wheel).
#[derive(Resource)]
pub struct CameraSpeed(pub f32);

impl Default for CameraSpeed {
    fn default() -> Self {
        CameraSpeed(5.0)
    }
}

/// Whether camera mode is active (Alt-toggled).
#[derive(Resource, Default)]
pub struct CameraMode {
    pub active: bool,
}

/// Camera yaw (horizontal) and pitch (vertical) state.
#[derive(Component, Default)]
pub struct FpsCamera {
    pub yaw: f32,
    pub pitch: f32,
}

fn spawn_camera(mut commands: Commands) {
    // Ambient light to see neurons clearly.
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 120.0,
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 20.0, 0.0).looking_at(Vec3::new(10.0, 10.0, 10.0), Vec3::Y),
        FpsCamera::default(),
    ));
}

/// Alt to toggle camera mode; Esc to exit.
fn toggle_camera_mode(
    keys: Res<ButtonInput<KeyCode>>,
    mut cam_mode: ResMut<CameraMode>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let alt_pressed = keys.just_pressed(KeyCode::AltLeft) || keys.just_pressed(KeyCode::AltRight);
    let esc_pressed = keys.just_pressed(KeyCode::Escape);

    if alt_pressed {
        cam_mode.active = !cam_mode.active;
    }
    if esc_pressed && cam_mode.active {
        cam_mode.active = false;
    }

    // Update cursor grab.
    if let Ok(mut window) = windows.get_single_mut() {
        if cam_mode.active {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

/// Mouse look — only when camera mode is active.
fn camera_look(
    cam_mode: Res<CameraMode>,
    mut motion: EventReader<MouseMotion>,
    mut query: Query<(&mut Transform, &mut FpsCamera)>,
) {
    if !cam_mode.active {
        motion.clear();
        return;
    }

    let sensitivity = 0.002_f32;

    for ev in motion.read() {
        for (mut transform, mut fps) in &mut query {
            fps.yaw -= ev.delta.x * sensitivity;
            fps.pitch -= ev.delta.y * sensitivity;
            fps.pitch = fps.pitch.clamp(-1.5, 1.5);

            transform.rotation = Quat::from_euler(EulerRot::YXZ, fps.yaw, fps.pitch, 0.0);
        }
    }
}

/// WASD movement + Shift/Space up/down + RMB pan.
fn camera_movement(
    cam_mode: Res<CameraMode>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut motion: EventReader<MouseMotion>,
    mut speed: ResMut<CameraSpeed>,
    mut query: Query<&mut Transform, With<FpsCamera>>,
) {
    // Scroll wheel adjusts speed (not zoom).
    for ev in scroll.read() {
        speed.0 = (speed.0 + ev.y * 0.5).clamp(0.5, 200.0);
    }

    if !cam_mode.active {
        motion.clear();
        return;
    }

    let Ok(mut transform) = query.get_single_mut() else {
        return;
    };

    let spd = speed.0 * time.delta_secs();
    let forward = transform.forward();
    let right = transform.right();
    let up = Vec3::Y; // Global Z in genesis = Y in Bevy

    // WASD — local movement.
    if keys.pressed(KeyCode::KeyW) {
        transform.translation += *forward * spd;
    }
    if keys.pressed(KeyCode::KeyS) {
        transform.translation -= *forward * spd;
    }
    if keys.pressed(KeyCode::KeyA) {
        transform.translation -= *right * spd;
    }
    if keys.pressed(KeyCode::KeyD) {
        transform.translation += *right * spd;
    }

    // Shift = up along global Y (= genesis Z).
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        transform.translation += up * spd;
    }
    // Space = down along global Y.
    if keys.pressed(KeyCode::Space) {
        transform.translation -= up * spd;
    }

    // RMB held = pan perpendicular to view (XZ plane relative to camera).
    if mouse_buttons.pressed(MouseButton::Right) {
        for ev in motion.read() {
            let pan_speed = spd * 0.3;
            transform.translation -= *right * ev.delta.x * pan_speed;
            transform.translation += up * ev.delta.y * pan_speed;
        }
    } else {
        motion.clear();
    }
}
