use bevy::prelude::*;
pub mod domain;
pub mod systems;

pub struct AiCopilotPlugin;

impl Plugin for AiCopilotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::render::init_copilot_windows_system,
                systems::render::render_copilot_system,
            ),
        );
    }
}
