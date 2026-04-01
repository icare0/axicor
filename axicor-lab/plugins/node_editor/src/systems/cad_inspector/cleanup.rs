use bevy::prelude::*;
use crate::domain::ShardCadEntity;

pub fn cleanup_cad_scene_system(
    mut commands: Commands,
    mut query: Query<&mut crate::domain::NodeGraphUiState>,
    entities: Query<Entity, With<ShardCadEntity>>,
) {
    for mut state in query.iter_mut() {
        let is_zone = matches!(state.level, crate::domain::EditorLevel::Zone(_));
        if !is_zone && state.shard_rtt.is_some() {
            state.shard_rtt = None;
            for entity in entities.iter() {
                commands.entity(entity).despawn_recursive();
            }
            info!("[CAD Inspector] Scene cleaned up");
        }
    }
}
