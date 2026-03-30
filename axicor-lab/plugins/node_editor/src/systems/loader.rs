use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, EditorLevel, ProjectSession};

#[derive(Component)]
pub struct LoadGraphTask(Task<LoadedGraph>);

pub struct LoadedGraph {
    pub project_name: String,
    pub file_path:    PathBuf, 
    pub father_id:    String, 
    pub zones:        Vec<String>,
    pub zone_ids:     HashMap<String, String>, 
    pub connections:  Vec<(String, String, String, String)>,
    pub node_inputs:  HashMap<String, Vec<String>>,
    pub node_outputs: HashMap<String, Vec<String>>,
    pub layout_cache: HashMap<String, (f32, f32)>, 
    pub level:        EditorLevel, 
}

pub fn spawn_load_task_system(
    mut open_events: EventReader<layout_api::OpenFileEvent>,
    mut load_events: EventReader<crate::domain::LoadGraphEvent>,
    mut topo_events: EventReader<layout_api::TopologyChangedEvent>,
    mut commands: Commands,
    graph: Res<BrainTopologyGraph>,
) {
    let mut target_path = None;

    for ev in load_events.read() {
        let name = clean_proj_name(&ev.project_name);
        let path = PathBuf::from("Genesis-Models").join(&name).join("simulation.toml");
        if graph.sessions.get(&path).map_or(false, |s| s.is_dirty) {
            info!("[NodeEditor] Session '{:?}' is dirty. Skipping reload.", path);
            continue;
        }
        target_path = Some(path);
    }
    for ev in topo_events.read() {
        target_path = Some(PathBuf::from("Genesis-Models").join(clean_proj_name(&ev.project_name)).join("brain.toml"));
    }
    for ev in open_events.read() {
        target_path = Some(ev.path.clone());
    }

    let Some(path) = target_path else { return };
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else { return };

    let level = if file_name == "simulation.toml" || file_name.ends_with(".axic") {
        EditorLevel::Model
    } else if file_name == "blueprints.toml" || file_name == "io.toml" || file_name == "shard.toml" {
        EditorLevel::Zone(path.parent().and_then(|p| p.file_name()).map_or("Zone".to_string(), |n| n.to_string_lossy().into_owned()))
    } else if file_name.ends_with(".toml") && !file_name.starts_with('.') && file_name != "manifest.toml" {
        EditorLevel::Department
    } else {
        return;
    };

    info!("[NodeEditor] Spawning load task for: {:?} at level {:?}", path, level);
    let task = AsyncComputeTaskPool::get().spawn(async move {
        load_graph_from_disk(path, level)
    });
    commands.spawn(LoadGraphTask(task));
}

pub fn apply_loaded_graph_system(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadGraphTask)>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_states: Query<&mut NodeGraphUiState>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        let Some(result) = future::block_on(future::poll_once(&mut task.0)) else { continue };

        let proj_name = clean_proj_name(&result.project_name);
        graph.active_project = Some(proj_name);
        graph.active_path = Some(result.file_path.clone());

        let session = graph.sessions.entry(result.file_path.clone()).or_insert_with(ProjectSession::default);
        
        if !session.is_dirty {
            session.father_id = result.father_id;
            session.zones = result.zones;
            session.zone_ids = result.zone_ids;
            session.connections = result.connections;
            session.node_inputs = result.node_inputs;
            session.node_outputs = result.node_outputs;
            session.layout_cache = result.layout_cache;

            // Синхронизируем I/O порты (пины) с реальными io.toml на диске (Cold Boot Fix)
            crate::systems::io::utils::sync_io_ports_from_disk(&result.file_path, session);
        }

        for mut ui in ui_states.iter_mut() {
            ui.level = result.level.clone(); 
            ui.node_positions.clear(); 
        }

        commands.entity(entity).despawn();
        info!("[NodeEditor] Switched to session: {:?}", result.file_path);
    }
}

fn load_graph_from_disk(path: PathBuf, level: EditorLevel) -> LoadedGraph {
    let mut zones = Vec::new();
    let mut zone_ids = HashMap::new();
    let mut connections = Vec::new();
    let mut node_inputs = HashMap::new();
    let mut node_outputs = HashMap::new();
    let mut layout_cache = HashMap::new();
    let mut father_id = String::new();

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(toml) = content.parse::<toml_edit::DocumentMut>() {
             // Парсинг модели
             if let Some(id_table) = toml.get("model_id_v1") {
                father_id = id_table.get("id").and_then(|v| v.as_str()).unwrap_or("0000").to_string();
             }
             // Парсинг департамента (или зоны в симуляции)
             if let Some(id_table) = toml.get("depart_id_v1") {
                father_id = id_table.get("id").and_then(|v| v.as_str()).unwrap_or("0000").to_string();
             }

             if let Some(dept_arr) = toml.get("department").and_then(|v| v.as_array_of_tables()) {
                for table in dept_arr.iter() {
                    if let Some(name) = table.get("name").and_then(|v| v.as_str()) {
                        zones.push(name.to_string());
                        let id = table.get("depart_id_v1").and_then(|t| t.get("id")).and_then(|v| v.as_str()).unwrap_or("0000").to_string();
                        zone_ids.insert(name.to_string(), id);
                        node_inputs.insert(name.to_string(), vec!["in".to_string()]);
                        node_outputs.insert(name.to_string(), vec!["out".to_string()]);
                    }
                }
             }

             if let Some(zone_arr) = toml.get("zone").and_then(|v| v.as_array_of_tables()) {
                for table in zone_arr.iter() {
                    if let Some(name) = table.get("name").and_then(|v| v.as_str()) {
                        zones.push(name.to_string());
                        let id = table.get("shard_id_v1").and_then(|t| t.get("id")).and_then(|v| v.as_str()).unwrap_or("0000").to_string();
                        zone_ids.insert(name.to_string(), id);
                    }
                }
             }

             if let Some(conn_arr) = toml.get("connection").and_then(|v| v.as_array_of_tables()) {
                for table in conn_arr.iter() {
                    let from = table.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let to = table.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let m = table.get("output_matrix").and_then(|v| v.as_str()).unwrap_or("out").to_string();
                    connections.push((from, m, to, "in".to_string()));
                }
             }

             // Пытаемся загрузить лэйаут
             let layout_path = path.parent().unwrap().join(format!(".{}.layout.tmp.toml", path.file_name().unwrap().to_string_lossy()));
             if let Ok(l_content) = std::fs::read_to_string(layout_path) {
                if let Ok(l_toml) = l_content.parse::<toml_edit::DocumentMut>() {
                    if let Some(nodes) = l_toml.get("nodes").and_then(|v| v.as_table()) {
                        for (id, val) in nodes.iter() {
                            let x = val.get("x").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
                            let y = val.get("y").and_then(|v| v.as_float()).unwrap_or(0.0) as f32;
                            layout_cache.insert(id.to_string(), (x, y));
                        }
                    }
                }
             }
        }
    }

    LoadedGraph {
        project_name: path.parent().and_then(|p| p.file_name()).map_or("Unknown".to_string(), |n| n.to_string_lossy().into_owned()),
        file_path: path,
        father_id,
        zones,
        zone_ids,
        connections,
        node_inputs,
        node_outputs,
        layout_cache,
        level,
    }
}

fn clean_proj_name(name: &str) -> String {
    name.replace("Genesis-Models/", "").replace("/", "")
}
