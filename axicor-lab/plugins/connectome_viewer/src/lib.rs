use bevy::prelude::*;

pub mod domain;
pub mod systems;

pub use domain::{ShardGeometry, ViewportCamera};

pub struct ConnectomeViewerPlugin;

impl Plugin for ConnectomeViewerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<domain::ZoneSelectedEvent>()
           .add_systems(Update, (
               systems::camera::attach_camera_to_viewport_system, // Добавлено
               systems::viewport_camera_control_system,
               systems::load_zone_geometry_system,
               systems::render::render_connectome_viewer_system,
           ));
    }
}
