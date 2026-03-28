pub mod domain;
pub mod systems;

use bevy::prelude::*;

pub struct CodeEditorPlugin;

impl Plugin for CodeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            systems::interaction::init_editor_windows_system,
            systems::interaction::handle_open_file_system,
            systems::render::render_code_editor_system,
        ));
    }
}
