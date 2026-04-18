use bevy::prelude::*;
use layout_api::{PluginWindow, base_domain, DOMAIN_BLUEPRINT_EDITOR};
use connectome_viewer::domain::ZoneSelectedEvent;
use crate::domain::BlueprintEditorState;

pub fn init_blueprint_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Without<BlueprintEditorState>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_BLUEPRINT_EDITOR {
            commands.entity(entity).insert(BlueprintEditorState::default());
        }
    }
}

pub fn sync_active_zone_system(
    mut events: EventReader<ZoneSelectedEvent>,
    mut query: Query<&mut BlueprintEditorState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            state.active_zone = Some(ev.shard_name.clone());
        }
    }
}

pub fn debounce_save_blueprint_system(
    time: Res<Time>,
    mut query: Query<&mut BlueprintEditorState>,
    mut topo_events: EventWriter<node_editor::domain::TopologyMutation>,
    graph: Res<node_editor::domain::BrainTopologyGraph>,
) {
    for mut state in query.iter_mut() {
        if state.is_dirty {
            state.debounce_timer += time.delta_seconds();
            //  500     
            if state.debounce_timer > 0.5 {
                state.is_dirty = false;
                state.debounce_timer = 0.0;
                if let Some(zone) = &state.active_zone {
                    topo_events.send(node_editor::domain::TopologyMutation::UpdateBlueprint {
                        zone: zone.clone(),
                        context_path: graph.active_path.clone(),
                    });
                }
            }
        }
    }
}
