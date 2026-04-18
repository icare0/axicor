use bevy::prelude::*;
use crate::domain::{AnatomySlicerState, ShardCadEntity};

pub fn cleanup_cad_scene_system(
    mut commands: Commands,
    mut query: Query<&mut AnatomySlicerState>,
    entities: Query<Entity, With<ShardCadEntity>>,
    mut last_zone: Local<Option<String>>,
) {
    for mut state in query.iter_mut() {
        let current_zone = state.active_zone.clone();

        if *last_zone != current_zone {
            //         3D 
            if last_zone.is_some() {
                for entity in entities.iter() {
                    commands.entity(entity).despawn_recursive();
                }
                info!("[Shard Slicer] 3D scene cleaned up due to zone transition");
                
                if current_zone.is_none() {
                    state.shard_rtt = None;
                }
            }
            
            *last_zone = current_zone;
        }
    }
}
