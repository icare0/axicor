use bevy::prelude::*;
use std::path::Path;
use node_editor::domain::{BrainTopologyGraph, TopologyMutation, DeleteTarget};
use crate::layout::systems::wm_file_ops::{load_document, save_document, remove_array_of_tables_item, remove_io_record_by_name, remove_connection_record};

/// Система-роутер: делегирует физическое удаление сущностей изолированным функциям.
pub fn delete_entity_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<BrainTopologyGraph>,
    mut deleted_ev: EventWriter<layout_api::EntityDeletedEvent>,
) {
    for ev in events.read() {
        if let TopologyMutation::Delete(target, context_path) = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            match target {
                DeleteTarget::Zone { name, id } => {
                    let path_str = active_path.to_string_lossy();
                    if path_str.contains("simulation.toml") {
                        delete_department(active_path, &name, &id, &mut deleted_ev);
                    } else if path_str.ends_with(".toml") {
                        delete_shard(active_path, &name, &id, &mut deleted_ev);
                    }
                }
                DeleteTarget::Connection { from, from_port, to, to_port } => {
                    delete_connection(active_path, &from, &from_port, &to, &to_port, &graph);
                }
                DeleteTarget::Layer { zone, name } => {
                    delete_anatomy_layer(active_path, &zone, &name, &graph);
                }
                DeleteTarget::IoPin { zone, is_input, name } => {
                    let section = if *is_input { "input" } else { "output" };
                    let path_str = active_path.to_string_lossy();
                    let is_sim = path_str.contains("simulation.toml");
                    let dept_name = active_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");
                    let project_dir = active_path.parent().unwrap_or(Path::new("."));

                    let io_path = if is_sim { project_dir.join(zone).join("io.toml") } else { project_dir.join(&dept_name).join(zone).join("io.toml") };

                    if let Ok(mut doc) = crate::layout::systems::wm_file_ops::load_document(&io_path) {
                        if crate::layout::systems::wm_file_ops::remove_array_of_tables_item(&mut doc, section, "name", name) {
                            let _ = crate::layout::systems::wm_file_ops::save_document(&io_path, &doc);
                            info!("✅ [IO] Deleted pin {} from {:?}", name, io_path);
                        }
                    }
                }
            }
        }
    }
}

fn delete_shard(active_path: &Path, name: &str, id: &str, _deleted_ev: &mut EventWriter<layout_api::EntityDeletedEvent>) {
    info!("🗑 [Orchestrator] Starting physical deletion of Shard: {} (ID: {})", name, id);
    if let Ok(mut doc) = load_document(active_path) {
        if remove_array_of_tables_item(&mut doc, "zone", "shard_id_v1", id) {
            if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) {
                let mut to_remove = Vec::new();
                for (i, table) in arr.iter().enumerate() {
                    let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
                    let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
                    if f == name || t == name { to_remove.push(i); }
                }
                for i in to_remove.into_iter().rev() { arr.remove(i); }
            }
            let _ = save_document(active_path, &doc);
        }
    }
    info!("🗑 [Sandbox] Shard {} removed from AST. Physical deletion deferred to Compile phase.", name);
}

fn delete_department(active_path: &Path, name: &str, id: &str, _deleted_ev: &mut EventWriter<layout_api::EntityDeletedEvent>) {
    info!("🗑 [Orchestrator] Deleting Department: {} (ID: {})", name, id);
    if let Ok(mut doc) = load_document(active_path) {
        if remove_array_of_tables_item(&mut doc, "department", "depart_id_v1", id) {
            if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) {
                let mut to_remove = Vec::new();
                for (i, table) in arr.iter().enumerate() {
                    let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
                    let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
                    if f == name || t == name { to_remove.push(i); }
                }
                for i in to_remove.into_iter().rev() { arr.remove(i); }
            }
            let _ = save_document(active_path, &doc);
        }
    }
    info!("🗑 [Sandbox] Department {} removed from AST. Physical deletion deferred to Compile phase.", name);
}

fn delete_connection(active_path: &Path, from: &str, from_port: &str, to: &str, to_port: &str, graph: &Res<BrainTopologyGraph>) {
    let is_from_rx = graph.sessions.get(active_path).map_or(false, |s| s.env_rx_nodes.contains(&from.to_string()));
    let is_to_tx = graph.sessions.get(active_path).map_or(false, |s| s.env_tx_nodes.contains(&to.to_string()));

    let path_str = active_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
    let project_dir = active_path.parent().unwrap_or(Path::new("."));

    let resolve_io_path = |node_name: &str| {
        if is_sim { project_dir.join(node_name).join("io.toml") }
        else { project_dir.join(&dept_name).join(node_name).join("io.toml") }
    };

    if is_from_rx {
        let io_path = resolve_io_path(to);
        if let Ok(mut doc) = load_document(&io_path) {
            if remove_io_record_by_name(&mut doc, "input", from_port) { let _ = save_document(&io_path, &doc); }
        }
    } else if is_to_tx {
        let io_path = resolve_io_path(from);
        if let Ok(mut doc) = load_document(&io_path) {
            if remove_io_record_by_name(&mut doc, "output", from_port) { let _ = save_document(&io_path, &doc); }
        }
    } else {
        if let Ok(mut doc) = load_document(active_path) {
            // [DCR] 1. Извлекаем габариты удаляемой связи
            let mut proj_w: i64 = 0;
            let mut proj_h: i64 = 0;
            if let Some(arr) = doc.get("connection").and_then(|i| i.as_array_of_tables()) {
                for table in arr.iter() {
                    let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
                    let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
                    let m = table.get("output_matrix").and_then(|v| v.as_str()).unwrap_or("");
                    if f == from && t == to && m == from_port {
                        proj_w = table.get("width").and_then(|v| v.as_integer()).unwrap_or(32);
                        proj_h = table.get("height").and_then(|v| v.as_integer()).unwrap_or(32);
                        break;
                    }
                }
            }

            // 2. Физическое удаление
            if remove_connection_record(&mut doc, from, to, from_port, to_port) { 
                let _ = save_document(active_path, &doc); 
                
                // [DCR] 3. Освобождение VRAM на целевом шарде
                if proj_w > 0 && proj_h > 0 {
                    let dst_shard_path = if is_sim { project_dir.join(to).join("shard.toml") } else { project_dir.join(&dept_name).join(to).join("shard.toml") };
                    if let Ok(mut dst_doc) = load_document(&dst_shard_path) {
                        let capacity_sub = proj_w * proj_h * 2;
                        let current = dst_doc.get("settings").and_then(|s| s.get("ghost_capacity")).and_then(|v| v.as_integer()).unwrap_or(0);
                        let new_cap = (current - capacity_sub).max(0);
                        if let Some(settings) = dst_doc.get_mut("settings").and_then(|s| s.as_table_mut()) {
                            // Обязательный каст через toml_edit::value
                            settings.insert("ghost_capacity", toml_edit::value(new_cap));
                        }
                        let _ = save_document(&dst_shard_path, &dst_doc);
                        info!("✅ [DCR] Freed ghost_capacity for {}. New capacity: {}", to, new_cap);
                    }
                }
            }
        }
    }
}

fn delete_anatomy_layer(active_path: &Path, zone: &str, name: &str, _graph: &Res<BrainTopologyGraph>) {
    let path_str = active_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = active_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");
    let project_dir = active_path.parent().unwrap_or(Path::new("."));

    let anatomy_path = if is_sim {
        project_dir.join(zone).join("anatomy.toml")
    } else {
        project_dir.join(&dept_name).join(zone).join("anatomy.toml")
    };

    if let Ok(mut doc) = crate::layout::systems::wm_file_ops::load_document(&anatomy_path) {
        if crate::layout::systems::wm_file_ops::remove_anatomy_layer_record(&mut doc, name) {
            let _ = crate::layout::systems::wm_file_ops::save_document(&anatomy_path, &doc);
            info!("✅ [Anatomy] Deleted layer {} from {:?}", name, anatomy_path);
        }
    }
}
