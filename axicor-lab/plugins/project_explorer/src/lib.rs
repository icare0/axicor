pub mod domain;
pub mod systems;

use bevy::prelude::*;
use domain::*;

pub struct ProjectExplorerPlugin;

impl Plugin for ProjectExplorerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectFsCache>()
           // Сканер файловой системы крутится в Update параллельно со всем остальным
           .add_systems(Update, (
               systems::scanner::fs_scanner_system,
               systems::render::render_project_explorer_system,
               systems::interaction::create_new_model_system,
               systems::interaction::sync_smart_focus_system,
           ));
    }
}
