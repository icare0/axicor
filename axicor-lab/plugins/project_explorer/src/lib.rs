pub mod domain;
pub mod systems;

use bevy::prelude::*;
use domain::*;

pub struct ProjectExplorerPlugin;

impl Plugin for ProjectExplorerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectFsCache>()
            .add_event::<layout_api::OpenContextMenuEvent>()
            .add_event::<layout_api::ContextMenuActionTriggeredEvent>()
            .add_event::<node_editor::domain::TopologyMutation>()
            //      Update
            .add_systems(
                Update,
                (
                    systems::scanner::fs_scanner_system,
                    systems::render::render_project_explorer_system,
                    systems::interaction::sync_smart_focus_system,
                    systems::interaction::handle_explorer_menu_triggers_system,
                    systems::interaction::evict_deleted_focus_system,
                ),
            );
    }
}
