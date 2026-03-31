use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, SaveProjectEvent};
use crate::systems::io::utils::flush_session_to_disk;

/// Система сохранения текущего состояния проекта во временные файлы (.tmp).
/// Выполняется немедленно при получении SaveProjectEvent.
pub fn save_project_system(
    mut events: EventReader<SaveProjectEvent>,
    graph: Res<BrainTopologyGraph>,
    ui_state_query: Query<&NodeGraphUiState>,
) {
    for _ev in events.read() {
        let Some(active_path) = &graph.active_path else { continue };
        let Some(session) = graph.sessions.get(active_path) else { continue };

        // Находим UI-состояние для этого окна (через активный путь)
        // В DOD мы просто берем первый попавшийся UI state, так как 
        // в мульти-оконном режиме у нас может быть несколько окон с одним путем.
        // Для простоты берем первый доступный.
        let ui_state = ui_state_query.iter().next();

        info!("[IO] Saving visual layout for {:?}", active_path);

        if let Err(e) = flush_session_to_disk(active_path, session, ui_state, true) {
            error!("❌ [IO] Failed to save layout: {}", e);
        } else {
            info!("✅ [IO] Visual layout saved to .layout.tmp.toml");
        }
    }
}
