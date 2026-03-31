use bevy::prelude::*;
use std::fs;
use std::path::Path;
use crate::domain::{BrainTopologyGraph, CompileGraphEvent, NodeGraphUiState};

pub fn compile_project_system(
    mut events: EventReader<CompileGraphEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
    _ui_state_query: Query<&NodeGraphUiState>,
) {
    for _ev in events.read() {
        let Some(active_proj) = graph.active_project.clone() else { continue };

        let base_dir = Path::new("Genesis-Models").join(&active_proj);
        let sandbox_dir = base_dir.join(".Sandbox");
        let autosave_dir = sandbox_dir.join(".tmp.autosave");
        let last_backup_dir = sandbox_dir.join(".tmp.last_backup");
        let old_backup_dir = sandbox_dir.join(".tmp.old_backup");

        if !autosave_dir.exists() {
            info!("✅ [Compile] Sandbox is empty. Nothing to commit.");
            for (_, session) in graph.sessions.iter_mut() { session.is_dirty = false; }
            continue;
        }

        info!("⚙️ [Compile] Starting transactional commit for '{}'...", active_proj);

        // 3.0 Удаляем старый бэкап (если есть - удалится, если нет - игнор)
        let _ = fs::remove_dir_all(&old_backup_dir);

        // 3.1 Ротация прошлого бэкапа
        if last_backup_dir.exists() {
            let _ = fs::rename(&last_backup_dir, &old_backup_dir);
        }

        // 3.2-3.3 Копируем измененные файлы поверх оригиналов
        if let Err(e) = copy_dir_recursive(&autosave_dir, &base_dir) {
            error!("❌ [Compile] Failed to apply sandbox to pure files: {}", e);
            continue;
        }

        // Переименование autosave в last_backup атомарно очищает песочницу
        // Это эквивалентно перемещению всех tmp файлов в last_backup
        if let Err(e) = fs::rename(&autosave_dir, &last_backup_dir) {
            error!("❌ [Compile] Failed to rotate autosave to last_backup: {}", e);
            continue;
        }

        // Сбрасываем флаги в RAM
        for (_, session) in graph.sessions.iter_mut() {
            session.is_dirty = false;
        }

        info!("✅ [Compile] Successfully committed changes to disk. Backups rotated.");
    }
}

/// Рекурсивное копирование директории (Overlay Apply)
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        
        if ft.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(&entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
