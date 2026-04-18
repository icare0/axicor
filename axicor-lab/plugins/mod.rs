use bevy::prelude::*;
use bevy::app::PluginGroupBuilder;

// DOD FIX:  . AllPlugins    .
//        .

pub struct AllPlugins;

impl PluginGroup for AllPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(project_explorer::ProjectExplorerPlugin)
            .add(node_editor::NodeEditorPlugin)
            .add(connectome_viewer::ConnectomeViewerPlugin)
            .add(code_editor::CodeEditorPlugin)
            .add(ai_copilot::AiCopilotPlugin)
            .add(io_inspector::IoInspectorPlugin)
            .add(blueprint_editor::BlueprintEditorPlugin)
            .add(anatomy_slicer::AnatomySlicerPlugin)
            .add(matrix_editor::MatrixEditorPlugin)
    }
}
