use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::TopologyMutation;
use crate::layout::systems::wm_file_ops::{load_document, save_document, remove_array_of_tables_item};

pub fn delete_shard_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<node_editor::domain::BrainTopologyGraph>,
    mut deleted_ev: EventWriter<layout_api::EntityDeletedEvent>,
) {
    for ev in events.read() {
        if let TopologyMutation::RemoveZone { name, id, context_path } = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            let path_str = active_path.to_string_lossy();
            if path_str.contains("simulation.toml") || path_str.contains("shard.toml") { continue; }
            if !path_str.ends_with(".toml") { continue; }

            info!("[Orchestrator] Starting physical deletion of Shard: {} (ID: {})", name, id);

            let mut doc = match load_document(active_path) {
                Ok(d) => d,
                Err(e) => {
                    error!("[Orchestrator] Could not read Department config for deletion: {}", e);
                    continue;
                }
            };

            if remove_array_of_tables_item(&mut doc, "zone", "shard_id_v1", id) {
                if let Err(e) = save_document(active_path, &doc) {
                    error!("[Orchestrator] Failed to save AST after deletion: {}", e);
                } else {
                    info!("[Orchestrator] Shard {} removed from registry.", name);
                }
            }

            let project_dir = active_path.parent().unwrap_or(Path::new("."));
            let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
            let shard_dir = project_dir.join(&dept_name).join(name);

            if shard_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&shard_dir) {
                    error!("[Orchestrator] Failed to delete shard directory: {}", e);
                } else {
                    info!("[Orchestrator] Shard directory deleted.");
                    deleted_ev.send(layout_api::EntityDeletedEvent { path: shard_dir });
                }
            }
        }
    }
}
