use bevy::prelude::*;
use layout_api::{PluginWindow, base_domain, DOMAIN_IO_INSPECTOR};
use connectome_viewer::domain::ZoneSelectedEvent;
use crate::domain::IoInspectorState;

pub fn init_io_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Without<IoInspectorState>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_IO_INSPECTOR {
            commands.entity(entity).insert(IoInspectorState::default());
        }
    }
}

pub fn sync_active_zone_system(
    mut events: EventReader<ZoneSelectedEvent>,
    mut query: Query<&mut IoInspectorState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            state.active_zone = Some(ev.shard_name.clone());
        }
    }
}
