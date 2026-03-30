use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::TopologyMutation;
use crate::layout::systems::wm_file_ops::{load_document, save_document, remove_array_of_tables_item};

pub fn delete_department_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<node_editor::domain::BrainTopologyGraph>,
    mut deleted_ev: EventWriter<layout_api::EntityDeletedEvent>,
) {
    for ev in events.read() {
        if let TopologyMutation::RemoveZone { name, id, context_path } = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            if !active_path.to_string_lossy().contains("simulation.toml") { continue; }

            info!("[Orchestrator] Deleting Department: {} (ID: {})", name, id);

            let mut doc = match load_document(active_path) {
                Ok(d) => d,
                Err(e) => {
                    error!("[Orchestrator] Could not read AST for deletion: {}", e);
                    continue;
                }
            };

            if remove_array_of_tables_item(&mut doc, "department", "depart_id_v1", id) {
                if let Err(e) = save_document(active_path, &doc) {
                    error!("[Orchestrator] Failed to save AST after deletion: {}", e);
                } else {
                    info!("[Orchestrator] Department {} excised from simulation.toml", name);
                }
            }

            let project_dir = active_path.parent().unwrap_or(Path::new("."));
            let brain_file = project_dir.join(format!("{}.toml", name));
            let zone_dir = project_dir.join(name);

            if brain_file.exists() { let _ = fs::remove_file(&brain_file); }
            if zone_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&zone_dir) {
                    error!("[Orchestrator] Failed to delete directory {:?}: {}", zone_dir, e);
                } else {
                    deleted_ev.send(layout_api::EntityDeletedEvent { path: zone_dir });
                }
            }
        }
    }
}
