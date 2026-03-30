use bevy::prelude::*;
use node_editor::domain::TopologyMutation;
use std::path::Path;
use crate::layout::systems::wm_file_ops::{load_document, save_document, remove_io_record_by_name, remove_connection_record};

pub fn delete_connection_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };

    for ev in events.read() {
        if let TopologyMutation::RemoveConnection { from, from_port, to, to_port } = ev {
            let is_from_rx;
            let is_to_tx;

            {
                let session = if let Some(s) = graph.sessions.get(&active_path) { s } else { continue };
                is_from_rx = session.env_rx_nodes.contains(from);
                is_to_tx = session.env_tx_nodes.contains(to);
            }

            let path_str = active_path.to_string_lossy();
            let is_sim = path_str.contains("simulation.toml");
            let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
            let project_dir = active_path.parent().unwrap_or(Path::new("."));

            let resolve_io_path = |node_name: &str| {
                if is_sim {
                    project_dir.join(node_name).join("io.toml")
                } else {
                    project_dir.join(&dept_name).join(node_name).join("io.toml")
                }
            };

            if is_from_rx {
                let io_path = resolve_io_path(to);
                if let Ok(mut doc) = load_document(&io_path) {
                    if remove_io_record_by_name(&mut doc, "input", from_port) {
                        let _ = save_document(&io_path, &doc);
                        info!("[Orchestrator] GXI Edge (RX) excised from {:?}", io_path);
                    }
                }
            } else if is_to_tx {
                let io_path = resolve_io_path(from);
                if let Ok(mut doc) = load_document(&io_path) {
                    if remove_io_record_by_name(&mut doc, "output", from_port) {
                        let _ = save_document(&io_path, &doc);
                        info!("[Orchestrator] GXO Edge (TX) excised from {:?}", io_path);
                    }
                }
            } else {
                if let Ok(mut doc) = load_document(&active_path) {
                    if remove_connection_record(&mut doc, from, to, from_port) {
                        if let Err(e) = save_document(&active_path, &doc) {
                            error!("[Orchestrator] Failed to save AST after connection deletion: {}", e);
                        } else {
                            info!("[Orchestrator] Ghost Axon connection {} -> {} severed.", from, to);
                        }
                    }
                }
            }

            // Синхронное обновление RAM
            if let Some(session_mut) = graph.sessions.get_mut(&active_path) {
                session_mut.connections.retain(|(f, fp, t, tp)| {
                    !(f == from && fp == from_port && t == to && tp == to_port)
                });
                session_mut.is_dirty = true;
            }
        }
    }
}
