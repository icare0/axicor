use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::window::{CursorGrabMode, PrimaryWindow};

#[derive(Component)]
pub struct IdeCamera {
    pub is_active: bool,
    pub speed: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for IdeCamera {
    fn default() -> Self {
        Self {
            is_active: false,
            speed: 500.0, // Базовая скорость (мкм/сек)
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // Размещаем камеру над слоем L1, смотрим вниз (Z - вверх)
        Transform::from_xyz(500.0, -500.0, 1500.0).looking_at(Vec3::ZERO, Vec3::Z),
        IdeCamera::default(),
    ));
}

/// Zero-Cost контроллер камеры (Blender-like UX).
/// Выполняется каждый кадр, реагирует на сырые события ввода.
pub fn camera_controller(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut q_camera: Query<(&mut Transform, &mut IdeCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btns: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    time: Res<Time>,
) {
    let mut window = q_windows.single_mut();
    let (mut transform, mut cam) = q_camera.single_mut();

    // 1. Context Switch: Активация/деактивация режима камеры (Alt / Esc)
    if keys.just_pressed(KeyCode::AltLeft) || keys.just_pressed(KeyCode::AltRight) {
        cam.is_active = !cam.is_active;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cam.is_active = false;
    }

    if cam.is_active {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    } else {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
        // Early Exit: если не в режиме камеры, мышь нужна для UI/Raycasting
        return; 
    }

    // 2. Управление скоростью (Колесико мыши)
    for ev in mouse_wheel.read() {
        // Логарифмический скейл скорости (10 мкм/с -> 10,000 мкм/с)
        let multiplier = if ev.y > 0.0 { 1.2 } else { 0.8 };
        cam.speed = (cam.speed * multiplier).clamp(10.0, 20_000.0);
    }

    // 3. Вращение (Mouse Look)
    let mut mouse_delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        mouse_delta += ev.delta;
    }
    
    if mouse_delta.length_squared() > 0.0 {
        cam.yaw -= mouse_delta.x * 0.002;
        cam.pitch -= mouse_delta.y * 0.002;
        // Защита от переворота (Gimbal Lock)
        cam.pitch = cam.pitch.clamp(-1.54, 1.54); 
        
        transform.rotation = Quat::from_euler(EulerRot::ZYX, cam.yaw, cam.pitch, 0.0);
    }

    // 4. Перемещение (С учетом ориентации)
    let mut velocity = Vec3::ZERO;
    let forward = transform.forward();
    let right = transform.right();

    if keys.pressed(KeyCode::KeyW) { velocity += *forward; }
    if keys.pressed(KeyCode::KeyS) { velocity -= *forward; }
    if keys.pressed(KeyCode::KeyA) { velocity -= *right; }
    if keys.pressed(KeyCode::KeyD) { velocity += *right; }
    
    // Глобальное изменение высоты (Strict Z)
    if keys.pressed(KeyCode::Space) { velocity += Vec3::Z; }
    if keys.pressed(KeyCode::ShiftLeft) { velocity -= Vec3::Z; }

    // 5. Pan-режим (ПКМ)
    if mouse_btns.pressed(MouseButton::Right) && mouse_delta.length_squared() > 0.0 {
        let up = transform.up();
        // Движение строго перпендикулярно направлению взгляда
        velocity -= *right * mouse_delta.x * 0.5;
        velocity += *up * mouse_delta.y * 0.5; 
    }

    if velocity.length_squared() > 0.0 {
        let delta = velocity.normalize() * cam.speed * time.delta_secs();
        transform.translation += delta;
    }
}
