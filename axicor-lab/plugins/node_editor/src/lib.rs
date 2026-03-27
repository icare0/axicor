use bevy::prelude::*;

pub mod domain;
pub mod ui;
pub mod systems;

use domain::*;

pub struct NodeEditorPlugin;

impl Plugin for NodeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrainTopologyGraph>()
           .init_resource::<NodeGraphUiState>()
           // DOD FIX: Явная аллокация VFS-кэша
           .init_resource::<layout_api::ActiveBundle>()
           .add_event::<TopologyMutation>()
           .add_event::<SaveProjectEvent>()
           .add_event::<BakeProjectEvent>()
           .add_event::<LoadGraphEvent>() 
           .add_systems(Update, (
               systems::sync_graph::sync_topology_graph_system,
               systems::mutations::apply_topology_mutations_system,
               systems::pipeline::project_pipeline_system,
               systems::render::render_node_editor_system,
           ));
    }
}
