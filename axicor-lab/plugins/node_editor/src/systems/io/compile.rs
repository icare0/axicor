use bevy::prelude::*;
use std::fs;
use crate::domain::{BrainTopologyGraph, CompileGraphEvent, NodeGraphUiState};
use crate::systems::io::utils::flush_session_to_disk;

/// Система компиляции проекта (Tmp -> Cold).
/// Переносит изменения из временных файлов в основные конфиги и сбрасывает dirty-флаг.
pub fn compile_project_system(
    mut events: EventReader<CompileGraphEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
    ui_state_query: Query<&NodeGraphUiState>,
) {
    for _ev in events.read() {
        info!("[IO] Compiling changes (Tmp -> Cold) for all dirty sessions");

        // Клонируем пути, чтобы избежать проблем с заимствованием при мутации сессий
        let dirty_paths: Vec<_> = graph.sessions.iter()
            .filter(|(_, s)| s.is_dirty)
            .map(|(p, _)| p.clone())
            .collect();

        if dirty_paths.is_empty() {
            info!("[IO] No dirty sessions found, nothing to compile.");
            continue;
        }

        for path in dirty_paths {
            let Some(session) = graph.sessions.get(&path) else { continue };
            let ui_state = ui_state_query.iter().next(); // Аналогично save, берем первый доступный

            // 1. Фиксация в основной TOML (is_tmp = false)
            if let Err(e) = flush_session_to_disk(&path, session, ui_state, false) {
                error!("❌ [IO] Failed to compile session {:?}: {}", path, e);
                continue;
            }

            // 2. Очистка временных файлов
            let toml_fname = path.file_name().unwrap_or_default().to_string_lossy();
            let parent_dir = path.parent().unwrap_or(std::path::Path::new("."));
            let tmp_path = parent_dir.join(format!("{}.tmp.toml", toml_fname.replace(".toml", "")));
            
            if tmp_path.exists() {
                let _ = fs::remove_file(tmp_path);
            }

            // 3. Сброс флага RAM-синхронизации
            if let Some(mut_session) = graph.sessions.get_mut(&path) {
                mut_session.is_dirty = false;
            }
            
            info!("✅ [IO] Session {:?} compiled and synchronized.", path);
        }
    }
}
