use bevy::prelude::*;
use std::path::Path;
use node_editor::domain::{BrainTopologyGraph, TopologyMutation, DeleteTarget};
use crate::layout::systems::wm_file_ops::{load_document, save_document, remove_array_of_tables_item, remove_io_record_by_name, remove_connection_record};

/// Система-роутер: делегирует физическое удаление сущностей изолированным функциям.
pub fn delete_entity_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<BrainTopologyGraph>,
    mut deleted_ev: EventWriter<layout_api::EntityDeletedEvent>,
    fs_cache: Res<project_explorer::domain::ProjectFsCache>,
) {
    for ev in events.read() {
        if let TopologyMutation::Delete(target, context_path) = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else { continue };

            match target {
                DeleteTarget::Zone { name, id } => {
                    let path_str = active_path.to_string_lossy();
                    if path_str.contains("simulation.toml") {
                        delete_department(active_path, &name, &id, &mut deleted_ev, &fs_cache);
                    } else if path_str.ends_with(".toml") {
                        delete_shard(active_path, &name, &id, &mut deleted_ev, &fs_cache);
                    }
                }
                DeleteTarget::Connection { from, from_port, to, to_port } => {
                    delete_connection(active_path, &from, &from_port, &to, &to_port, &graph, &fs_cache);
                }
                DeleteTarget::Layer { zone, name } => {
                    delete_anatomy_layer(active_path, &zone, &name, &graph, &fs_cache);
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

                            // [DCR] Cascade deletion of connections using this pin
                            if let Ok(mut macro_doc) = load_document(active_path) {
                                let mut connections_to_remove = Vec::new();
                                if let Some(arr) = macro_doc.get("connection").and_then(|i| i.as_array_of_tables()) {
                                    for (i, table) in arr.iter().enumerate() {
                                        let f = table.get("from").and_then(|v| v.as_str()).unwrap_or("");
                                        let t = table.get("to").and_then(|v| v.as_str()).unwrap_or("");
                                        let out_m = table.get("output_matrix").and_then(|v| v.as_str()).unwrap_or("");
                                        let in_m = table.get("input_matrix").and_then(|v| v.as_str()).unwrap_or("in");

                                        let matches = if *is_input {
                                            t == zone && in_m == name
                                        } else {
                                            f == zone && out_m == name
                                        };

                                        if matches {
                                            let w = table.get("width").and_then(|v| v.as_integer()).unwrap_or(32);
                                            let h = table.get("height").and_then(|v| v.as_integer()).unwrap_or(32);
                                            connections_to_remove.push((i, t.to_string(), w, h));
                                        }
                                    }
                                }

                                if !connections_to_remove.is_empty() {
                                    let mut macro_changed = false;
                                    // Удаляем с конца, чтобы индексы не плыли
                                    for (idx, target_zone, w, h) in connections_to_remove.into_iter().rev() {
                                        if let Some(arr) = macro_doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) {
                                            arr.remove(idx);
                                            macro_changed = true;
                                            
                                            // Освобождаем ghost_capacity на целевом шарде
                                            let dst_shard_path = if is_sim { project_dir.join(&target_zone).join("shard.toml") } else { project_dir.join(&dept_name).join(&target_zone).join("shard.toml") };
                                            if let Ok(mut dst_doc) = load_document(&dst_shard_path) {
                                                let capacity_sub = w * h * 2;
                                                let current = dst_doc.get("settings").and_then(|s| s.get("ghost_capacity")).and_then(|v| v.as_integer()).unwrap_or(0);
                                                let new_cap = (current - capacity_sub).max(0);
                                                if let Some(settings) = dst_doc.get_mut("settings").and_then(|s| s.as_table_mut()) {
                                                    settings.insert("ghost_capacity", toml_edit::value(new_cap));
                                                    let _ = save_document(&dst_shard_path, &dst_doc);
                                                    info!("✅ [DCR] Freed ghost_capacity for {}. New capacity: {}", target_zone, new_cap);
                                                }
                                            }
                                        }
                                    }
                                    if macro_changed {
                                        let _ = save_document(active_path, &macro_doc);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn delete_shard(active_path: &Path, name: &str, id: &str, _deleted_ev: &mut EventWriter<layout_api::EntityDeletedEvent>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
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

fn delete_department(active_path: &Path, name: &str, id: &str, _deleted_ev: &mut EventWriter<layout_api::EntityDeletedEvent>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
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

fn delete_connection(active_path: &Path, from: &str, from_port: &str, to: &str, to_port: &str, graph: &Res<BrainTopologyGraph>, fs_cache: &project_explorer::domain::ProjectFsCache) {
    let (is_from_rx, is_to_tx, from_id, to_id) = if let Some(session) = graph.sessions.get(active_path) {
        (
            session.env_rx_nodes.contains(&from.to_string()),
            session.env_tx_nodes.contains(&to.to_string()),
            session.zone_ids.get(from).cloned().unwrap_or_default(),
            session.zone_ids.get(to).cloned().unwrap_or_default(),
        )
    } else {
        return;
    };

    let src_shard_path = crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, &from_id).unwrap_or_default();
    let dst_shard_path = crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, &to_id).unwrap_or_default();
    let src_io_path = src_shard_path.parent().unwrap_or(Path::new(".")).join("io.toml");
    let dst_io_path = dst_shard_path.parent().unwrap_or(Path::new(".")).join("io.toml");

    if is_from_rx {
        if let Ok(mut doc) = load_document(&dst_io_path) {
            if remove_io_record_by_name(&mut doc, "input", from_port) { let _ = save_document(&dst_io_path, &doc); }
        }
    } else if is_to_tx {
        if let Ok(mut doc) = load_document(&src_io_path) {
            if remove_io_record_by_name(&mut doc, "output", from_port) { let _ = save_document(&src_io_path, &doc); }
        }
    } else {
        let local_doc = match load_document(active_path) { Ok(d) => d, Err(_) => return };
        let macro_path = if let Some(parent_id) = local_doc.get("depart_id_v1").and_then(|i| i.get("id")).and_then(|v| v.as_str()) {
            crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, parent_id).unwrap_or(active_path.to_path_buf())
        } else {
            active_path.to_path_buf()
        };

        if let Ok(mut doc) = load_document(&macro_path) {
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
                let _ = save_document(&macro_path, &doc); 
                
                // [DCR] 3. Освобождение VRAM на целевом шарде
                if proj_w > 0 && proj_h > 0 {
                    if let Ok(mut dst_doc) = load_document(&dst_shard_path) {
                        let capacity_sub = proj_w * proj_h * 2;
                        let current = dst_doc.get("settings").and_then(|s| s.get("ghost_capacity")).and_then(|v| v.as_integer()).unwrap_or(0);
                        let new_cap = (current - capacity_sub).max(0);
                        if let Some(settings) = dst_doc.get_mut("settings").and_then(|s| s.as_table_mut()) {
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

fn delete_anatomy_layer(active_path: &Path, zone: &str, name: &str, _graph: &Res<BrainTopologyGraph>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
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
