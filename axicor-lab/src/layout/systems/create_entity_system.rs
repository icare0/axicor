use bevy::prelude::*;
use std::fs;
use std::path::Path;
use node_editor::domain::{BrainTopologyGraph, TopologyMutation, CreateTarget};
use genesis_core::config::sys::SystemMeta;
use crate::layout::systems::wm_file_ops::{load_document, save_document, add_io_record};
use toml_edit::{value, ArrayOfTables, InlineTable, Item, Table};

pub fn create_entity_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_states: Query<&mut node_editor::domain::NodeGraphUiState>,
    mut commands: Commands,
    cad_entities: Query<Entity, With<node_editor::domain::ShardCadEntity>>,
    fs_cache: Res<project_explorer::domain::ProjectFsCache>,
) {
    let active_path_res = graph.active_path.clone();
    let Some(active_path) = active_path_res else { return };

    for ev in events.read() {
        if let TopologyMutation::Create(target, context_path) = ev {
            let target_path = context_path.as_ref().unwrap_or(&active_path);

            match target {
                CreateTarget::Zone { name, pos } => {
                    let path_str = target_path.to_string_lossy();
                    if path_str.contains("simulation.toml") {
                        create_department(target_path, name, pos, &mut graph, &mut ui_states, &fs_cache);
                    } else if path_str.ends_with(".toml") && !path_str.contains("shard.toml") {
                        create_shard(target_path, name, pos, &mut graph, &mut ui_states, &fs_cache);
                    }
                }
                CreateTarget::EnvRx { name, pos } => create_env_rx(target_path, name, pos, &mut graph, &mut ui_states, &fs_cache),
                CreateTarget::EnvTx { name, pos } => create_env_tx(target_path, name, pos, &mut graph, &mut ui_states, &fs_cache),
                CreateTarget::Connection { from, from_port, to, to_port, voxel_z } => {
                    create_connection(target_path, from, from_port, to, to_port, voxel_z.clone(), &mut graph, &fs_cache);
                    // [DOD FIX] Вызываем немедленную пересборку 3D сцены
                    for ent in cad_entities.iter() {
                        commands.entity(ent).despawn_recursive();
                    }
                }
                CreateTarget::Layer { zone, name, height_pct } => {
                    create_anatomy_layer(target_path, zone, name, *height_pct, &mut graph, &fs_cache);
                    // [DOD FIX] Вызываем немедленную пересборку 3D сцены
                    for ent in cad_entities.iter() {
                        commands.entity(ent).despawn_recursive();
                    }
                }
                CreateTarget::IoPin { zone, is_input, name } => {
                    let path_str = target_path.to_string_lossy();
                    let is_sim = path_str.contains("simulation.toml");
                    let dept_name = target_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");
                    let project_dir = target_path.parent().unwrap_or(Path::new("."));

                    let section = if *is_input { "input" } else { "output" };
                    let io_path = if is_sim {
                        project_dir.join(zone).join("io.toml")
                    } else {
                        project_dir.join(&dept_name).join(zone).join("io.toml")
                    };

                    let mut doc = match crate::layout::systems::wm_file_ops::load_document(&io_path) {
                        Ok(d) => d,
                        Err(_) => toml_edit::DocumentMut::new()
                    };

                    let io_id = format!("{}-{}", name, uuid::Uuid::new_v4().simple());
                    crate::layout::systems::wm_file_ops::add_io_record(&mut doc, section, name, &io_id, zone, 32, 32, None);

                    let _ = crate::layout::systems::wm_file_ops::save_document(&io_path, &doc);
                    info!("✅ [IO] Created pin {} in {:?}", name, io_path);

                    if let Some(session) = graph.sessions.get_mut(target_path) {
                        if *is_input {
                            let inputs = session.node_inputs.entry(zone.to_string()).or_default();
                            if !inputs.contains(&name.to_string()) { inputs.push(name.to_string()); }
                        } else {
                            let outputs = session.node_outputs.entry(zone.to_string()).or_default();
                            if !outputs.contains(&name.to_string()) { outputs.push(name.to_string()); }
                        }
                        session.is_dirty = true;
                    }
                }
            }
        }
    }
}

fn create_department(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
    info!("[Orchestrator] Starting clean birth of Department: {}", name);
    let project_dir = active_path.parent().unwrap_or(Path::new("."));
    let mut doc = match load_document(active_path) { Ok(d) => d, Err(_) => return };

    let sim_id = doc.get("model_id_v1").and_then(|i| i.get("id")).and_then(|v| v.as_str()).unwrap_or("0000").to_string();
    let sim_suffix = if sim_id.len() >= 4 { &sim_id[sim_id.len()-4..] } else { "0000" };
    let dept_uuid = SystemMeta::generate().id;
    let dept_suffix = if dept_uuid.len() >= 4 { &dept_uuid[dept_uuid.len()-4..] } else { "0000" };
    let structured_dept_id = format!("{}_{}", sim_suffix, dept_suffix);

    let mut dept_table = Table::new();
    dept_table.insert("name", value(name));
    dept_table.insert("config", value(format!("{}.toml", name)));
    let mut inline_id = InlineTable::new();
    inline_id.insert("id", structured_dept_id.clone().into());
    dept_table.insert("depart_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));

    if !doc.contains_key("department") { doc.insert("department", Item::ArrayOfTables(ArrayOfTables::new())); }
    if let Some(arr) = doc.get_mut("department").and_then(|i| i.as_array_of_tables_mut()) { arr.push(dept_table); }

    if save_document(active_path, &doc).is_err() { return; }

    let brain_path = project_dir.join(format!("{}.toml", name));
    let sandbox_brain_path = layout_api::resolve_sandbox_path(&brain_path);
    if let Some(p) = sandbox_brain_path.parent() { let _ = fs::create_dir_all(p); }
    
    let brain_toml = format!("[depart_id_v1]\nid = \"{}\"\n\n[simulation]\nconfig = \"simulation.toml\"\n", structured_dept_id);
    let _ = fs::write(&sandbox_brain_path, brain_toml);

    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.zones.push(name.to_string());
        session.zone_ids.insert(name.to_string(), structured_dept_id.clone());
        session.node_inputs.insert(name.to_string(), vec!["in".to_string()]);
        session.node_outputs.insert(name.to_string(), vec!["out".to_string()]);
        session.layout_cache.insert(structured_dept_id, (pos.x, pos.y));
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_shard(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
    let project_dir = active_path.parent().unwrap_or(Path::new("."));
    let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");

    let mut doc = match load_document(active_path) { Ok(d) => d, Err(_) => return };

    let dept_id = doc.get("depart_id_v1").and_then(|i| i.get("id")).and_then(|v| v.as_str()).unwrap_or("0000").to_string();
    let sim_path = project_dir.join("simulation.toml");
    let model_id = if let Ok(sim_doc) = load_document(&sim_path) {
        sim_doc.get("model_id_v1").and_then(|i| i.get("id")).and_then(|v| v.as_str()).unwrap_or("unknown").to_string()
    } else { "unknown".to_string() };

    let shard_uuid = SystemMeta::generate().id;
    let shard_suffix = if shard_uuid.len() >= 8 { &shard_uuid[shard_uuid.len()-8..] } else { "00000000" };
    let structured_shard_id = format!("{}_{}", dept_id, shard_suffix);

    let mut zone_table = Table::new();
    zone_table.insert("name", value(name));
    let mut inline_id = InlineTable::new();
    inline_id.insert("id", structured_shard_id.clone().into());
    zone_table.insert("shard_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
    zone_table.insert("blueprints", value(format!("{}/{}/blueprints.toml", dept_name, name)));
    zone_table.insert("anatomy", value(format!("{}/{}/anatomy.toml", dept_name, name)));
    zone_table.insert("io", value(format!("{}/{}/io.toml", dept_name, name)));
    zone_table.insert("baked_dir", value(format!("baked/{}/", name)));

    if !doc.contains_key("zone") { doc.insert("zone", Item::ArrayOfTables(ArrayOfTables::new())); }
    if let Some(arr) = doc.get_mut("zone").and_then(|i| i.as_array_of_tables_mut()) { arr.push(zone_table); }

    if save_document(active_path, &doc).is_err() { return; }

    let shard_dir = project_dir.join(&dept_name).join(name);
    let sandbox_shard_dir = layout_api::resolve_sandbox_path(&shard_dir);
    let _ = fs::create_dir_all(&sandbox_shard_dir);
    
    let _ = fs::write(sandbox_shard_dir.join("anatomy.toml"), "[[layer]]\nname = \"Main\"\nheight_pct = 1.0\ndensity = 0.1\ncomposition = {}\n");
    let _ = fs::write(sandbox_shard_dir.join("blueprints.toml"), "[[neuron_type]]\nname = \"Default\"\nthreshold = 20000\nrest_potential = 0\nleak_rate = 100\nhomeostasis_penalty = 1000\nhomeostasis_decay = 10\nrefractory_period = 10\nsynapse_refractory_period = 10\nsignal_propagation_length = 5\n");
    let io_toml = format!(
        "[shard_id_v1]\nid = \"{}-IO\"\n\n[[input]]\nname = \"in\"\nzone = \"{}\"\ntarget_type = \"All\"\nwidth = 32\nheight = 32\nstride = 1\nentry_z = \"top\"\n\n[[output]]\nname = \"out\"\nzone = \"{}\"\ntarget_type = \"All\"\nwidth = 32\nheight = 32\nstride = 1\n",
        structured_shard_id, name, name
    );
    let _ = std::fs::write(sandbox_shard_dir.join("io.toml"), io_toml);

    let shard_toml = format!(r#"model_id_v1 = {{ id = "{}" }}
depart_id_v1 = {{ id = "{}" }}
shard_id_v1 = {{ id = "{}" }}

zone_id = "{}"

[dimensions]
w = 32
d = 32
h = 32

[world_offset]
x = 0
y = 0
z = 0

[settings]
night_interval_ticks = 10000
prune_threshold = 15
max_sprouts = 16
ghost_capacity = 0
"#, model_id, dept_id, structured_shard_id, structured_shard_id);
    let _ = fs::write(sandbox_shard_dir.join("shard.toml"), shard_toml);

    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.zones.push(name.to_string());
        session.zone_ids.insert(name.to_string(), structured_shard_id.clone());
        session.node_inputs.insert(name.to_string(), vec!["in".to_string()]);
        session.node_outputs.insert(name.to_string(), vec!["out".to_string()]);
        session.layout_cache.insert(structured_shard_id, (pos.x, pos.y));
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_env_rx(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.env_rx_nodes.push(name.to_string());
        session.node_outputs.insert(name.to_string(), vec!["out".to_string()]);
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_env_tx(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>, _fs_cache: &project_explorer::domain::ProjectFsCache) {
    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.env_tx_nodes.push(name.to_string());
        session.node_inputs.insert(name.to_string(), vec!["in".to_string()]);
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_connection(active_path: &Path, from: &str, from_port: &str, to: &str, to_port: &str, voxel_z: Option<u32>, graph: &mut BrainTopologyGraph, fs_cache: &project_explorer::domain::ProjectFsCache) {
    let is_from_rx;
    let is_to_tx;
    let from_id;
    let to_id;

    {
        let session = if let Some(s) = graph.sessions.get(active_path) { s } else { return };
        is_from_rx = session.env_rx_nodes.contains(&from.to_string());
        is_to_tx = session.env_tx_nodes.contains(&to.to_string());
        from_id = session.zone_ids.get(from).cloned().unwrap_or_default();
        to_id = session.zone_ids.get(to).cloned().unwrap_or_default();
    }

    let from_pfx = if from_id.len() >= 4 { &from_id[from_id.len()-4..] } else { &from_id };
    let to_sfx = if to_id.len() >= 4 { &to_id[to_id.len()-4..] } else { &to_id };
    let uuid_full = SystemMeta::generate().id.replace("-", "");
    let core_id = if uuid_full.len() >= 6 { &uuid_full[..6] } else { &uuid_full };
    let io_id = format!("{}_{}_{}", from_pfx, core_id, to_sfx);

    let src_shard_path = crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, &from_id).unwrap_or_default();
    let dst_shard_path = crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, &to_id).unwrap_or_default();
    let src_io_path = src_shard_path.parent().unwrap_or(Path::new(".")).join("io.toml");
    let dst_io_path = dst_shard_path.parent().unwrap_or(Path::new(".")).join("io.toml");

    if is_from_rx {
        if let Ok(mut doc) = load_document(&dst_io_path) {
            add_io_record(&mut doc, "input", from_port, &io_id, to, 32, 32, voxel_z);
            let _ = save_document(&dst_io_path, &doc);
        }
        // [DOD FIX] Роутинг Z для входов от окружения
        if let Some(z) = voxel_z {
            if let Ok(mut io_doc) = load_document(&dst_io_path) {
                if crate::layout::systems::wm_file_ops::update_io_input_z(&mut io_doc, from_port, z) {
                    let _ = save_document(&dst_io_path, &io_doc);
                }
            }
        }
    } else if is_to_tx {
        if let Ok(mut doc) = load_document(&src_io_path) {
            add_io_record(&mut doc, "output", from_port, &io_id, from, 32, 32, None);
            let _ = save_document(&src_io_path, &doc);
        }
    } else {
        // [DCR] 1. Извлекаем реальные габариты матрицы-источника
        let mut proj_w: i64 = 32;
        let mut proj_h: i64 = 32;
        if let Ok(src_doc) = load_document(&src_io_path) {
            if let Some(outputs) = src_doc.get("output").and_then(|i| i.as_array_of_tables()) {
                for t in outputs.iter() {
                    if t.get("name").and_then(|v| v.as_str()) == Some(from_port) {
                        proj_w = t.get("width").and_then(|v| v.as_integer()).unwrap_or(32);
                        proj_h = t.get("height").and_then(|v| v.as_integer()).unwrap_or(32);
                        break;
                    }
                }
            }
        }

        let local_doc = match load_document(active_path) { Ok(d) => d, Err(_) => return };
        // Родитель всегда знает детей: берем ID родителя из локального файла
        let macro_path = if let Some(parent_id) = local_doc.get("depart_id_v1").and_then(|i| i.get("id")).and_then(|v| v.as_str()) {
            crate::layout::systems::wm_file_ops::find_path_by_id(fs_cache, parent_id).unwrap_or(active_path.to_path_buf())
        } else {
            active_path.to_path_buf()
        };

        let mut doc = match load_document(&macro_path) { Ok(d) => d, Err(_) => return };
        let mut conn_table = Table::new();
        let mut inline_id = InlineTable::new();
        inline_id.insert("id", io_id.clone().into());
        conn_table.insert("conn_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
        conn_table.insert("from", value(from));
        conn_table.insert("to", value(to));
        conn_table.insert("output_matrix", value(from_port));
        conn_table.insert("input_matrix", value(to_port));
        conn_table.insert("width", value(proj_w));
        conn_table.insert("height", value(proj_h));
        
        if let Some(z) = voxel_z {
            conn_table.insert("entry_z", value(z as i64));
        } else {
            conn_table.insert("entry_z", value("top"));
        }

        if !doc.contains_key("connection") { doc.insert("connection", Item::ArrayOfTables(ArrayOfTables::new())); }
        if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) { arr.push(conn_table); }
        let _ = save_document(&macro_path, &doc);

        // [DCR] 2. Динамическое резервирование VRAM на целевом шарде
        if let Ok(mut dst_doc) = load_document(&dst_shard_path) {
            let capacity_add = proj_w * proj_h * 2;
            let current = dst_doc.get("settings").and_then(|s| s.get("ghost_capacity")).and_then(|v| v.as_integer()).unwrap_or(0);
            if let Some(settings) = dst_doc.get_mut("settings").and_then(|s| s.as_table_mut()) {
                settings.insert("ghost_capacity", value(current + capacity_add));
            }
            let _ = save_document(&dst_shard_path, &dst_doc);
            info!("✅ [DCR] Reserved {} ghost_capacity for {}", capacity_add, to);
        }
    }

    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.connections.push((from.to_string(), from_port.to_string(), to.to_string(), to_port.to_string()));
        session.is_dirty = true;
    }
}

fn create_anatomy_layer(active_path: &Path, zone: &str, name: &str, height_pct: f32, graph: &mut BrainTopologyGraph, _fs_cache: &project_explorer::domain::ProjectFsCache) {
    let path_str = active_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = active_path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");
    let project_dir = active_path.parent().unwrap_or(Path::new("."));

    let anatomy_path = if is_sim {
        project_dir.join(zone).join("anatomy.toml")
    } else {
        project_dir.join(&dept_name).join(zone).join("anatomy.toml")
    };

    let mut doc = match crate::layout::systems::wm_file_ops::load_document(&anatomy_path) {
        Ok(d) => d,
        Err(_) => toml_edit::DocumentMut::new(),
    };

    crate::layout::systems::wm_file_ops::add_anatomy_layer_record(&mut doc, name, height_pct);
    let _ = crate::layout::systems::wm_file_ops::save_document(&anatomy_path, &doc);
    info!("✅ [Anatomy] Created layer {} in {:?}", name, anatomy_path);

    if let Some(session) = graph.sessions.get_mut(active_path) {
        if let Some(anatomy) = session.shard_anatomies.get_mut(zone) {
            for layer in anatomy.layers.iter_mut() {
                layer.height_pct *= 1.0 - height_pct;
            }
            anatomy.layers.push(node_editor::domain::ShardLayer {
                name: name.to_string(),
                height_pct,
            });
        }
        session.is_dirty = true;
    }
}
