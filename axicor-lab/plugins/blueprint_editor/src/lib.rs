pub mod domain;
pub mod ui;
mod systems;

use bevy::prelude::*;

pub struct BlueprintEditorPlugin;

impl Plugin for BlueprintEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            systems::interaction::init_blueprint_windows_system,
            systems::interaction::sync_active_zone_system,
            systems::interaction::debounce_save_blueprint_system,
            systems::render::render_blueprint_editor_system,
        ).chain());
    }
}
