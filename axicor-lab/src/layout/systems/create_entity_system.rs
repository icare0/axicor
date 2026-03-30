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
                        create_department(target_path, name, pos, &mut graph, &mut ui_states);
                    } else if path_str.ends_with(".toml") && !path_str.contains("shard.toml") {
                        create_shard(target_path, name, pos, &mut graph, &mut ui_states);
                    }
                }
                CreateTarget::EnvRx { name, pos } => create_env_rx(target_path, name, pos, &mut graph, &mut ui_states),
                CreateTarget::EnvTx { name, pos } => create_env_tx(target_path, name, pos, &mut graph, &mut ui_states),
                CreateTarget::Connection { from, from_port, to, to_port } => create_connection(target_path, from, from_port, to, to_port, &mut graph),
                CreateTarget::IoMatrix { zone, is_input, name } => create_io_matrix(target_path, zone, *is_input, name, &mut graph),
            }
        }
    }
}

fn create_department(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>) {
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
    let brain_toml = format!("[depart_id_v1]\nid = \"{}\"\n\n[simulation]\nconfig = \"simulation.toml\"\n", structured_dept_id);
    let _ = fs::write(&brain_path, brain_toml);

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

fn create_shard(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>) {
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
    let _ = fs::create_dir_all(&shard_dir);
    let _ = fs::write(shard_dir.join("anatomy.toml"), "[[layer]]\nname = \"Main\"\nheight_pct = 1.0\ndensity = 0.1\ncomposition = {}\n");
    let _ = fs::write(shard_dir.join("blueprints.toml"), "[[neuron_type]]\nname = \"Default\"\nthreshold = 20000\nrest_potential = 0\nleak_rate = 100\nhomeostasis_penalty = 1000\nhomeostasis_decay = 10\nrefractory_period = 10\nsynapse_refractory_period = 10\nsignal_propagation_length = 5\n");
    let _ = fs::write(shard_dir.join("io.toml"), format!("[shard_id_v1]\nid = \"{}-IO\"\n\n[[input]]\nname = \"in\"\nwidth = 32\nheight = 32\n\n[[output]]\nname = \"out\"\nwidth = 32\nheight = 32\n", structured_shard_id));

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
ghost_capacity = 20000
"#, model_id, dept_id, structured_shard_id, structured_shard_id);
    let _ = fs::write(shard_dir.join("shard.toml"), shard_toml);

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

fn create_env_rx(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>) {
    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.env_rx_nodes.push(name.to_string());
        session.node_outputs.insert(name.to_string(), vec!["out".to_string()]);
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_env_tx(active_path: &Path, name: &str, pos: &bevy_egui::egui::Pos2, graph: &mut BrainTopologyGraph, ui_states: &mut Query<&mut node_editor::domain::NodeGraphUiState>) {
    if let Some(session) = graph.sessions.get_mut(active_path) {
        session.env_tx_nodes.push(name.to_string());
        session.node_inputs.insert(name.to_string(), vec!["in".to_string()]);
        session.is_dirty = true;
    }
    for mut ui in ui_states.iter_mut() { ui.node_positions.insert(name.to_string(), *pos); }
}

fn create_connection(active_path: &Path, from: &str, from_port: &str, to: &str, to_port: &str, graph: &mut BrainTopologyGraph) {
    let is_from_rx;
    let is_to_tx;
    let from_id;
    let to_id;

    {
        let session = if let Some(s) = graph.sessions.get(active_path) { s } else { return };
        is_from_rx = session.env_rx_nodes.contains(&from.to_string());
        is_to_tx = session.env_tx_nodes.contains(&to.to_string());
        from_id = session.zone_ids.get(from).cloned().unwrap_or_else(|| from.to_string());
        to_id = session.zone_ids.get(to).cloned().unwrap_or_else(|| to.to_string());
    }

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
        if is_sim { project_dir.join(node_name).join("io.toml") }
        else { project_dir.join(&dept_name).join(node_name).join("io.toml") }
    };

    if is_from_rx {
        let io_path = resolve_io_path(to);
        if let Ok(mut doc) = load_document(&io_path) {
            add_io_record(&mut doc, "input", from_port, &io_id, 32, 32);
            let _ = save_document(&io_path, &doc);
        }
    } else if is_to_tx {
        let io_path = resolve_io_path(from);
        if let Ok(mut doc) = load_document(&io_path) {
            add_io_record(&mut doc, "output", from_port, &io_id, 32, 32);
            let _ = save_document(&io_path, &doc);
        }
    } else {
        let mut doc = match load_document(active_path) { Ok(d) => d, Err(_) => return };
        let mut conn_table = Table::new();
        let mut inline_id = InlineTable::new();
        inline_id.insert("id", io_id.clone().into());
        conn_table.insert("conn_id_v1", Item::Value(toml_edit::Value::InlineTable(inline_id)));
        conn_table.insert("from", value(from));
        conn_table.insert("to", value(to));
        conn_table.insert("output_matrix", value(from_port));

        if !doc.contains_key("connection") { doc.insert("connection", Item::ArrayOfTables(ArrayOfTables::new())); }
        if let Some(arr) = doc.get_mut("connection").and_then(|i| i.as_array_of_tables_mut()) { arr.push(conn_table); }
        let _ = save_document(active_path, &doc);
    }

    if let Some(session_mut) = graph.sessions.get_mut(active_path) {
        let connection = (from.to_string(), from_port.to_string(), to.to_string(), to_port.to_string());
        if !session_mut.connections.contains(&connection) {
            session_mut.connections.push(connection);
            session_mut.is_dirty = true;
        }
    }
}

fn create_io_matrix(active_path: &Path, zone: &str, is_input: bool, name: &str, graph: &mut BrainTopologyGraph) {
    let path_str = active_path.to_string_lossy();
    let is_sim = path_str.contains("simulation.toml");
    let dept_name = active_path.file_name().unwrap().to_string_lossy().replace(".toml", "");
    let project_dir = active_path.parent().unwrap_or(Path::new("."));

    let io_path = if is_sim { project_dir.join(zone).join("io.toml") }
    else { project_dir.join(&dept_name).join(zone).join("io.toml") };

    if let Ok(mut doc) = load_document(&io_path) {
        let section = if is_input { "input" } else { "output" };
        let io_id = format!("{}_{}", zone, name);
        add_io_record(&mut doc, section, name, &io_id, 32, 32);
        let _ = save_document(&io_path, &doc);
    }

    if let Some(session) = graph.sessions.get_mut(active_path) {
        if is_input {
            let inputs = session.node_inputs.entry(zone.to_string()).or_default();
            if !inputs.contains(&name.to_string()) { inputs.push(name.to_string()); }
        } else {
            let outputs = session.node_outputs.entry(zone.to_string()).or_default();
            if !outputs.contains(&name.to_string()) { outputs.push(name.to_string()); }
        }
        session.is_dirty = true;
    }
}
