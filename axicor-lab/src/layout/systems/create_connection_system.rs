use bevy::prelude::*;
use node_editor::domain::TopologyMutation;
use std::path::Path;
use genesis_core::config::sys::SystemMeta;
use crate::layout::systems::wm_file_ops::{load_document, save_document, add_io_record};
use toml_edit::{value, ArrayOfTables, InlineTable, Item, Table};

pub fn create_connection_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };

    for ev in events.read() {
        if let TopologyMutation::AddConnection { from, from_port, to, to_port } = ev {
            let is_from_rx;
            let is_to_tx;
            let from_id;
            let to_id;

            {
                let session = if let Some(s) = graph.sessions.get(&active_path) { s } else { continue };
                is_from_rx = session.env_rx_nodes.contains(from);
                is_to_tx = session.env_tx_nodes.contains(to);
                from_id = session.zone_ids.get(from).cloned().unwrap_or_else(|| from.clone());
                to_id = session.zone_ids.get(to).cloned().unwrap_or_else(|| to.clone());
            }

            // Генерация Lineage ID
            let from_pfx = if from_id.len() >= 4 { &from_id[from_id.len()-4..] } else { &from_id };
            let to_sfx = if to_id.len() >= 4 { &to_id[to_id.len()-4..] } else { &to_id };
            let uuid_full = SystemMeta::generate().id.replace("-", "");
            let core_id = if uuid_full.len() >= 6 { &uuid_full[..6] } else { &uuid_full };
            let io_id = format!("{}_{}_{}", from_pfx, core_id, to_sfx);

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
                // External I/O: Sensor -> Shard
                let io_path = resolve_io_path(to);
                if let Ok(mut doc) = load_document(&io_path) {
                    add_io_record(&mut doc, "input", from_port, &io_id, 32, 32);
                    let _ = save_document(&io_path, &doc);
                    info!("[Orchestrator] GXI Edge (RX) bound to {:?}. Lineage: {}", io_path, io_id);
                }
            } else if is_to_tx {
                // External I/O: Shard -> Motor
                let io_path = resolve_io_path(from);
                if let Ok(mut doc) = load_document(&io_path) {
                    add_io_record(&mut doc, "output", from_port, &io_id, 32, 32);
                    let _ = save_document(&io_path, &doc);
                    info!("[Orchestrator] GXO Edge (TX) bound from {:?}. Lineage: {}", io_path, io_id);
                }
            } else {
                // Internal I/O: Ghost Axons Shard -> Shard
                let mut doc = match load_document(&active_path) {
                    Ok(d) => d,
                    Err(e) => {
                        error!("[Orchestrator] Failed to load AST for connection: {}", e);
                        continue;
                    }
                };

                let mut conn_table = Table::new();
                let mut inline_id = InlineTable::new();
                inline_id.insert("id", io_id.clone().into());
                conn_table.insert("conn_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
                conn_table.insert("from", value(from.clone()));
                conn_table.insert("to", value(to.clone()));
                conn_table.insert("output_matrix", value(from_port.clone()));

                if !doc.contains_key("connection") {
                    doc.insert("connection", Item::ArrayOfTables(ArrayOfTables::new()));
                }
                if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) {
                    arr.push(conn_table);
                }

                if let Err(e) = save_document(&active_path, &doc) {
                    error!("[Orchestrator] Failed to write AST connection: {}", e);
                } else {
                    info!("[Orchestrator] Ghost Axon connection {} -> {} established. Lineage: {}", from, to, io_id);
                }
            }

            // Синхронное обновление RAM (Single Source of Truth)
            if let Some(session_mut) = graph.sessions.get_mut(&active_path) {
                session_mut.connections.push((from.clone(), from_port.clone(), to.clone(), to_port.clone()));
                session_mut.is_dirty = true;
            }
        }
    }
}
