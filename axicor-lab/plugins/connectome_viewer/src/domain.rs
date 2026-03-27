use bevy::prelude::*;

#[derive(Component)]
pub struct ShardGeometry {
    pub viewport: Entity,
}

#[derive(Component)]
pub struct ViewportCamera {
    pub target: Vec3,
    pub radius: f32,
    pub alpha: f32, // Rotation around Y
    pub beta: f32,  // Rotation up/down
}

impl Default for ViewportCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            radius: 40.0,
            alpha: std::f32::consts::PI / 4.0,
            beta: 0.5,
        }
    }
}

#[derive(Event, Clone, Debug)]
pub struct ZoneSelectedEvent {
    pub project_name: String,
    pub shard_name: String,
}
