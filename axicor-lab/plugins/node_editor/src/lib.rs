use bevy::prelude::*;
use crate::domain::{
    BrainTopologyGraph, TopologyMutation,
    SaveProjectEvent, BakeProjectEvent, LoadGraphEvent,
};


pub mod domain;
pub mod ui;
pub mod systems;

pub struct NodeEditorPlugin;

impl Plugin for NodeEditorPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<BrainTopologyGraph>()
            .init_resource::<layout_api::ActiveBundle>()
            .add_event::<TopologyMutation>()
            .add_event::<SaveProjectEvent>()
            .add_event::<BakeProjectEvent>()
            .add_event::<LoadGraphEvent>()
            .add_event::<layout_api::TopologyChangedEvent>()
            .add_systems(Update, (
                systems::interaction::init_node_editor_windows_system,
                systems::interaction::sync_topology_graph_system,
                systems::mutations::apply_topology_mutations_system,
                systems::pipeline::project_pipeline_system,
                systems::render::render_node_editor_system,
            ).chain());
    }
}