pub mod domain;
pub mod ui;
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
           ));
    }
}
