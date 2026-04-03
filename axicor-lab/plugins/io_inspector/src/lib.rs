pub mod domain;
pub mod systems;

use bevy::prelude::*;

pub struct IoInspectorPlugin;

impl Plugin for IoInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            systems::interaction::init_io_windows_system,
            systems::interaction::sync_active_zone_system,
            systems::render::render_io_inspector_system,
        ).chain());
    }
}
