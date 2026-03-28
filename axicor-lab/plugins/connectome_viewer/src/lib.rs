use bevy::prelude::*;

pub mod domain;
pub mod systems;

pub struct ConnectomeViewerPlugin;

impl Plugin for ConnectomeViewerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<domain::NeuronInstances>()
           .init_resource::<domain::TopologyGraph>()
           .add_plugins(MaterialPlugin::<systems::material::NeuronInstanceMaterial>::default())
           .add_event::<domain::ZoneSelectedEvent>()
           .add_systems(Update, (
               systems::camera::attach_camera_to_viewport_system,
               systems::viewport_camera_control_system,
               systems::geometry::load_zone_geometry_system,
               systems::render::render_connectome_viewer_system,
               systems::interaction::soma_picking_system,
               systems::interaction::trace_active_connections_system,
           ));
    }
}
