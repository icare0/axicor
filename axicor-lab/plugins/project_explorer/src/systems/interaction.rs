use bevy::prelude::*;
use layout_api::ContextMenuActionTriggeredEvent;
use node_editor::domain::TopologyMutation;
use crate::domain::ProjectFsCache;
use std::fs;
use std::path::PathBuf;

pub fn sync_smart_focus_system(
    mut events: EventReader<layout_api::OpenFileEvent>,
    mut query: Query<&mut crate::domain::ProjectExplorerState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            state.active_file = Some(ev.path.clone());
        }
    }
}

pub fn handle_explorer_menu_triggers_system(
    mut events: EventReader<ContextMenuActionTriggeredEvent>,
    mut topo_events: EventWriter<TopologyMutation>,
    mut deleted_ev: EventWriter<layout_api::EntityDeletedEvent>,
    cache: Res<ProjectFsCache>,
) {
    for ev in events.read() {
        if !ev.action_id.starts_with("explorer.") {
            continue;
        }

        let parts: Vec<&str> = ev.action_id.split('|').collect();
        if parts.len() < 2 { continue; }

        match parts[0] {
            "explorer.delete_model" => {
                let model_name = parts[1];
                //    
                if let Some(project) = cache.projects.iter().find(|p| p.name == model_name) {
                    //    simulation.toml  root_nodes
                    let sim_path = project.root_nodes.iter()
                        .find(|n| n.node_type == crate::domain::ProjectNodeType::Simulation)
                        .map(|n| n.path.clone());
                    
                    if let Some(path) = sim_path {
                        if let Some(parent) = path.parent() {
                            info!("Project Explorer: Deleting model directory {:?}", parent);
                            if let Err(e) = fs::remove_dir_all(parent) {
                                error!("Project Explorer: Failed to delete model: {}", e);
                            } else {
                                deleted_ev.send(layout_api::EntityDeletedEvent { path: parent.to_path_buf() });
                            }

                        }
                    }
                }
            }
            "explorer.delete_dept" | "explorer.delete_shard" => {
                if parts.len() < 4 { continue; }
                let name = parts[1].to_string();
                let id = parts[2].to_string();
                let context_path = PathBuf::from(parts[3]);

                topo_events.send(TopologyMutation::Delete(
                    node_editor::domain::DeleteTarget::Zone { name, id: id.clone() },
                    Some(context_path),
                ));
                info!("Project Explorer: Triggered Delete cascade for {}", id);
            }
            _ => warn!("Unknown explorer action: {}", parts[0]),
        }
    }
}

pub fn evict_deleted_focus_system(
    mut events: EventReader<layout_api::EntityDeletedEvent>,
    mut query: Query<&mut crate::domain::ProjectExplorerState>,
) {
    for ev in events.read() {
        for mut state in query.iter_mut() {
            if let Some(active) = &state.active_file {
                if active.starts_with(&ev.path) {
                    state.active_file = None;
                }
            }
        }
    }
}
