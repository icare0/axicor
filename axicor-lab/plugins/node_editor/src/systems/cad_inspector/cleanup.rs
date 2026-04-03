use bevy::prelude::*;
use crate::domain::{NodeGraphUiState, ShardCadEntity, EditorLevel};

pub fn cleanup_cad_scene_system(
    mut commands: Commands,
    mut query: Query<&mut NodeGraphUiState>,
    entities: Query<Entity, With<ShardCadEntity>>,
    mut last_level: Local<Option<EditorLevel>>,
) {
    for mut state in query.iter_mut() {
        let current_level = state.level.clone();

        // Отлавливаем любой переход состояний
        if last_level.as_ref() != Some(&current_level) {
            let was_zone = matches!(last_level.as_ref(), Some(EditorLevel::Zone(_)));
            let is_zone = matches!(current_level, EditorLevel::Zone(_));

            // Если мы БЫЛИ в зоне, нам в любом случае нужно снести старую 3D сцену
            if was_zone {
                for entity in entities.iter() {
                    commands.entity(entity).despawn_recursive();
                }
                info!("[CAD Inspector] Scene cleaned up due to level transition");
                
                // Освобождаем VRAM текстуру ТОЛЬКО если выходим на макро-уровни
                if !is_zone {
                    state.shard_rtt = None;
                }
            }
            
            // Сохраняем новое состояние
            *last_level = Some(current_level);
        }
    }
}
