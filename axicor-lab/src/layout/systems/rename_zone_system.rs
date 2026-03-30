use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::{BrainTopologyGraph, TopologyMutation};

/// Система-хирург: отвечает за физическое переименование сущностей на диске.
pub fn rename_zone_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<BrainTopologyGraph>,
) {
    for ev in events.read() {
        if let TopologyMutation::RenameZone { old_name, new_name, id, context_path } = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            info!("📝 [Orchestrator] Renaming entity: {} -> {} (ID: {})", old_name, new_name, id);

            // 1. Обновление родительского конфига (simulation.toml или Zone_N.toml)
            let Ok(content) = fs::read_to_string(active_path) else {
                error!("❌ [Orchestrator] Could not read config for renaming");
                continue;
            };

            let updated_content = content.replace(
                &format!("name = \"{}\"", old_name),
                &format!("name = \"{}\"", new_name)
            );

            if updated_content != content {
                let _ = fs::write(active_path, updated_content);
                info!("✅ [Orchestrator] Config updated with new name.");
            }

            // 2. Физическое переименование папок и файлов
            let project_dir = active_path.parent().unwrap_or(Path::new("."));
            
            // Если мы в simulation.toml, значит переименовываем Департамент
            if active_path.to_string_lossy().contains("simulation.toml") {
                let old_file = project_dir.join(format!("{}.toml", old_name));
                let new_file = project_dir.join(format!("{}.toml", new_name));
                let old_dir = project_dir.join(old_name);
                let new_dir = project_dir.join(new_name);

                if old_file.exists() { let _ = fs::rename(old_file, new_file); }
                if old_dir.exists() && old_dir.is_dir() { let _ = fs::rename(old_dir, new_dir); }
                
                info!("✅ [Orchestrator] Department files and directories renamed.");
            } else {
                // Иначе мы внутри департамента и переименовываем Шард
                let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
                let old_shard_dir = project_dir.join(&dept_name).join(old_name);
                let new_shard_dir = project_dir.join(&dept_name).join(new_name);

                if old_shard_dir.exists() && old_shard_dir.is_dir() {
                    let _ = fs::rename(old_shard_dir, new_shard_dir);
                    info!("✅ [Orchestrator] Shard directory renamed.");
                }
            }
        }
    }
}
