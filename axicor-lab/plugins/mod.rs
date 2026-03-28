use bevy::prelude::*;
use bevy::app::PluginGroupBuilder;

// DOD FIX: Строгий фасад. AllPlugins является единственной точкой входа.
// Прямая зависимость от крейтов плагинов теперь только здесь.

pub struct AllPlugins;

impl PluginGroup for AllPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(project_explorer::ProjectExplorerPlugin)
            .add(node_editor::NodeEditorPlugin)
            .add(connectome_viewer::ConnectomeViewerPlugin)
            .add(code_editor::CodeEditorPlugin)
    }
}
