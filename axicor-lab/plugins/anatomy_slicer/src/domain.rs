use bevy::prelude::*;

#[derive(Component, Default)]
pub struct AnatomySlicerState {
    pub active_zone: Option<String>,
    pub shard_rtt: Option<Handle<Image>>,
    pub cad_viewport_size: bevy_egui::egui::Vec2,
    pub cad_viewport_rect: Option<bevy_egui::egui::Rect>,
    pub active_3d_hover: Option<(bevy_egui::egui::Pos2, u32)>, 
}

#[derive(Component)]
pub struct ShardCadEntity;

#[derive(Component)]
pub struct CadCameraState {
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, //   Y
    pub beta: f32,  //  /
}

impl Default for CadCameraState {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 60.0,
            alpha: std::f32::consts::PI / 4.0,
            beta: 0.5,
        }
    }
}
