pub mod domain;
pub mod systems;

use bevy::prelude::*;

pub struct MatrixEditorPlugin;

impl Plugin for MatrixEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            systems::interaction::init_matrix_editor_windows_system,
            systems::interaction::sync_active_zone_system,
            systems::render::render_matrix_editor_system,
        ).chain());
    }
}
