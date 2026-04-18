use crate::domain::{
    BakeProjectEvent, BrainTopologyGraph, CompileGraphEvent, LoadGraphEvent, SaveProjectEvent,
    TopologyMutation,
};
use bevy::prelude::*;

pub mod domain;
pub mod systems;
pub mod ui;

pub struct NodeEditorPlugin;

impl Plugin for NodeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrainTopologyGraph>()
            .init_resource::<layout_api::ActiveBundle>()
            .add_event::<TopologyMutation>()
            .add_event::<SaveProjectEvent>()
            .add_event::<CompileGraphEvent>()
            .add_event::<BakeProjectEvent>()
            .add_event::<LoadGraphEvent>()
            .add_event::<layout_api::TopologyChangedEvent>()
            .add_event::<layout_api::OpenContextMenuEvent>()
            .add_event::<layout_api::ContextMenuActionTriggeredEvent>()
            .add_systems(
                Update,
                (
                    systems::interaction::init_node_editor_windows_system,
                    systems::interaction::handle_node_editor_menu_triggers_system,
                    systems::io::save::save_project_system,
                    systems::io::save::persist_blueprints_system,
                    systems::io::save::persist_io_system,
                    systems::io::compile::compile_project_system,
                    systems::io::bake::bake_project_system,
                    systems::io::layout::autosave_layout_system,
                    systems::mutations::apply_topology_mutations_system,
                    systems::mutations::evict_deleted_entities_system,
                    systems::mutations::hot_reload_io_system,
                    systems::loader::spawn_load_task_system,
                    systems::loader::apply_loaded_graph_system,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    systems::render::render_node_editor_system,
                    systems::modals::clear_graph_modal_system,
                )
                    .chain(),
            );
    }
}
