use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::{BrainTopologyGraph, TopologyMutation, RenameTarget};
use crate::layout::systems::wm_file_ops::{load_document, save_document};

/// Система-роутер: делегирует физическое переименование сущностей изолированным функциям.
pub fn rename_zone_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<BrainTopologyGraph>,
) {
    for ev in events.read() {
        if let TopologyMutation::Rename(target, context_path) = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            match target {
                RenameTarget::Shard { old_name, new_name, id } => rename_shard(active_path, old_name, new_name, id),
                RenameTarget::IoPin { zone, is_input, old_name, new_name } => rename_io_pin(active_path, zone, is_input, old_name, new_name),
            }
        }
    }
}

fn rename_shard(active_path: &Path, old_name: &str, new_name: &str, id: &str) {
    info!("📝 [Orchestrator] Renaming entity: {} -> {} (ID: {})", old_name, new_name, id);
    let Ok(content) = fs::read_to_string(active_path) else {
        error!("❌ [Orchestrator] Could not read config for renaming");
        return;
    };
    let updated_content = content.replace(&format!("name = \"{}\"", old_name), &format!("name = \"{}\"", new_name));
    if updated_content != content {
        let _ = fs::write(active_path, updated_content);
        info!("✅ [Orchestrator] Config updated with new name.");
    }
    let project_dir = active_path.parent().unwrap_or(Path::new("."));
    if active_path.to_string_lossy().contains("simulation.toml") {
        let old_file = project_dir.join(format!("{}.toml", old_name));
        let new_file = project_dir.join(format!("{}.toml", new_name));
        let old_dir = project_dir.join(old_name);
        let new_dir = project_dir.join(new_name);
        if old_file.exists() { let _ = fs::rename(old_file, new_file); }
        if old_dir.exists() && old_dir.is_dir() { let _ = fs::rename(old_dir, new_dir); }
        info!("✅ [Orchestrator] Department files and directories renamed.");
    } else {
        let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
        let old_shard_dir = project_dir.join(&dept_name).join(old_name);
        let new_shard_dir = project_dir.join(&dept_name).join(new_name);
        if old_shard_dir.exists() && old_shard_dir.is_dir() {
            let _ = fs::rename(old_shard_dir, new_shard_dir);
            info!("✅ [Orchestrator] Shard directory renamed.");
        }
    }
}

fn rename_io_pin(active_path: &Path, zone: &str, is_input: &bool, old_name: &str, new_name: &str) {
    let path_str = active_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
    let project_dir = active_path.parent().unwrap_or(Path::new("."));

    let io_path = if is_sim {
        project_dir.join(zone).join("io.toml")
    } else {
        project_dir.join(&dept_name).join(zone).join("io.toml")
    };

    if let Ok(mut doc) = load_document(&io_path) {
        let section = if *is_input { "input" } else { "output" };
        if let Some(arr) = doc.get_mut(section).and_then(|i| i.as_array_of_tables_mut()) {
            for table in arr.iter_mut() {
                if let Some(name_val) = table.get("name").and_then(|v| v.as_str()) {
                    if name_val == old_name {
                        table.insert("name", toml_edit::value(new_name));
                        break;
                    }
                }
            }
        }
        if let Err(e) = save_document(&io_path, &doc) {
            error!("❌ [Orchestrator] Failed to rename IO pin in {:?}: {}", io_path, e);
        } else {
            info!("✅ [Orchestrator] Renamed IO pin {} -> {} in {:?}", old_name, new_name, io_path);
        }
    }
}
