use crate::domain::AnatomySlicerState;
use bevy::prelude::*;
use connectome_viewer::domain::ZoneSelectedEvent;
use layout_api::{base_domain, PluginWindow, DOMAIN_ANATOMY_SLICER};

pub fn init_slicer_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Without<AnatomySlicerState>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_ANATOMY_SLICER {
            commands
                .entity(entity)
                .insert(AnatomySlicerState::default());
        }
    }
}

pub fn sync_active_zone_system(
    mut events: EventReader<ZoneSelectedEvent>,
    mut query: Query<&mut AnatomySlicerState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            state.active_zone = Some(ev.shard_name.clone());
        }
    }
}
