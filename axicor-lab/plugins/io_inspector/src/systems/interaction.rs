use crate::domain::IoInspectorState;
use bevy::prelude::*;
use connectome_viewer::domain::ZoneSelectedEvent;
use layout_api::{base_domain, PluginWindow, DOMAIN_IO_INSPECTOR};

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
