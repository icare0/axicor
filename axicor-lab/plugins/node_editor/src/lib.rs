use bevy::prelude::*;
use crate::domain::{
    BrainTopologyGraph, TopologyMutation,
    SaveProjectEvent, CompileGraphEvent, BakeProjectEvent, LoadGraphEvent,
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
            .add_event::<CompileGraphEvent>()
            .add_event::<BakeProjectEvent>()
            .add_event::<LoadGraphEvent>()
            .add_event::<layout_api::TopologyChangedEvent>()
            .add_event::<layout_api::OpenContextMenuEvent>()
            .add_event::<layout_api::ContextMenuActionTriggeredEvent>()
            .add_systems(Update, (
                systems::interaction::init_node_editor_windows_system,
                systems::interaction::handle_node_editor_menu_triggers_system,
                systems::io::save::save_project_system,
                systems::io::compile::compile_project_system,
                systems::io::bake::bake_project_system,
                systems::io::layout::autosave_layout_system,
                systems::mutations::apply_topology_mutations_system,
                systems::mutations::evict_deleted_entities_system,
                systems::loader::spawn_load_task_system,
                systems::loader::apply_loaded_graph_system,
            ).chain())
            .add_systems(Update, (
                systems::cad_inspector::vram::allocate_vram_system,
                systems::cad_inspector::vram::sync_vram_system,
                systems::cad_inspector::camera::spawn_cad_camera_system,
                systems::cad_inspector::camera::sync_camera_aspect_system,
                systems::cad_inspector::camera::cad_camera_control_system,
                systems::cad_inspector::geometry::spawn_cad_geometry_system,
                systems::render::render_node_editor_system,                  // UI ПИШЕТ pending_3d_drop + dragging_over_3d
                systems::cad_inspector::geometry::sync_hover_plane_system,   // Читает active_3d_hover (из raycast прошлого кадра)
                systems::cad_inspector::raycast::dnd_raycast_system,         // ЧИТАЕТ pending_3d_drop → шлёт OpenContextMenuEvent
                systems::cad_inspector::cleanup::cleanup_cad_scene_system,
                systems::modals::clear_graph_modal_system,
            ).chain());
                }
                }