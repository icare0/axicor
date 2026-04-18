use crate::domain::MatrixEditorState;
use bevy::prelude::*;
use connectome_viewer::domain::ZoneSelectedEvent;
use layout_api::{base_domain, PluginWindow, DOMAIN_MATRIX_EDITOR};

pub fn init_matrix_editor_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow)>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_MATRIX_EDITOR {
            commands.entity(entity).insert(MatrixEditorState::default());
        }
    }
}

pub fn sync_active_zone_system(
    mut events: EventReader<ZoneSelectedEvent>,
    mut query: Query<&mut MatrixEditorState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            state.active_zone = Some(ev.shard_name.clone());
        }
    }
}
