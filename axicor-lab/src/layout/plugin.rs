use bevy::prelude::*;
use bevy_egui::EguiPlugin;

// DOD FIX: Внутренняя память оконного менеджера (Изолировано)
use crate::layout::domain::{
    SaveDefaultLayoutEvent, WindowDragState,
    TreeCommands, OsWindowCommand
};

// DOD FIX: Междоменная шина (Доступна плагинам)
use layout_api::{
    AllocatedPanes, WindowDragRequest, TopologyCache, CreateNewModelEvent,
    OpenContextMenuEvent, ContextMenuActionTriggeredEvent
};

use crate::layout::systems;

pub struct WindowManagerPlugin;

impl Plugin for WindowManagerPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin);
        }

        app.insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.1)))
           .insert_resource(Msaa::Off)
           .init_resource::<WindowDragState>()
           .init_resource::<WindowDragRequest>()
           .init_resource::<TopologyCache>()
           .init_resource::<AllocatedPanes>()
           .init_resource::<crate::layout::ui::context_menu::ContextMenuState>()
           .init_resource::<TreeCommands>();

        app.add_event::<SaveDefaultLayoutEvent>()
           .add_event::<OsWindowCommand>()
           .add_event::<CreateNewModelEvent>()
           .add_event::<OpenContextMenuEvent>()
           .add_event::<ContextMenuActionTriggeredEvent>()
           .add_event::<layout_api::EntityDeletedEvent>()
           .add_systems(Startup, systems::boot::boot_layout_system)
           .add_systems(Update, (
               systems::input::window_input_system,
               systems::input::route_plugin_inputs_system,
               systems::window::evaluate_drag_intents_system,
               systems::window::execute_window_commands_system,
               systems::window::window_garbage_collector_system,
               systems::window::window_drag_execution_system,
               systems::input::sync_plugin_geometry_system,
               systems::render::render_workspace_system,
               systems::render::sync_plugin_visibility_system,
               systems::save::save_layout_system,
           ).chain().after(bevy_egui::EguiSet::InitContexts))
           .add_systems(Update, (
               systems::create_model_system::create_model_system,
               systems::create_entity_system::create_entity_system,
               systems::delete_entity_system::delete_entity_system,
               systems::rename_zone_system::rename_zone_system,

               systems::wm_file_ops::wm_file_ops_context_menu_system,
               crate::layout::ui::context_menu::context_menu_ui_system,
           ).chain().after(bevy_egui::EguiSet::InitContexts));

    }
}