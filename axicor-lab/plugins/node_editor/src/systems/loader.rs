use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, EditorLevel, ProjectSession};
use layout_api::{ProjectFsCache, ProjectNode};

fn find_path_by_id(cache: &ProjectFsCache, target_id: &str) -> Option<PathBuf> {
    fn search(nodes: &[ProjectNode], target_id: &str) -> Option<PathBuf> {
        for node in nodes {
            if node.id == target_id { return Some(node.path.clone()); }
            if let Some(p) = search(&node.children, target_id) { return Some(p); }
        }
        None
    }
    for proj in &cache.projects {
        if let Some(p) = search(&proj.root_nodes, target_id) { return Some(p); }
    }
    None
}

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
    pub shard_anatomies: HashMap<String, crate::domain::ShardAnatomy>,
    pub voxel_size_um: f32,
    pub level:        EditorLevel, 
}

pub fn spawn_load_task_system(
    mut open_events: EventReader<layout_api::OpenFileEvent>,
    mut load_events: EventReader<crate::domain::LoadGraphEvent>,
    mut topo_events: EventReader<layout_api::TopologyChangedEvent>,
    mut commands: Commands,
    graph: Res<BrainTopologyGraph>,
    fs_cache: Res<ProjectFsCache>,
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
    } else if file_name == "shard.toml" || file_name == "blueprints.toml" || file_name == "io.toml" || file_name == "anatomy.toml" {
        // [DOD FIX] Микро-уровень полностью делегирован выделенным плагинам.
        // Node Editor игнорирует эти файлы, оставаясь на макро-уровне.
        return;
    } else if file_name.ends_with(".toml") && !file_name.starts_with('.') && file_name != "manifest.toml" {
        EditorLevel::Department
    } else {
        return;
    };

    info!("[NodeEditor] Spawning load task for: {:?} at level {:?}", path, level);
    let cache_clone = ProjectFsCache { projects: fs_cache.projects.clone() };
    let task = AsyncComputeTaskPool::get().spawn(async move {
        load_graph_from_disk(path, level, cache_clone)
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
            session.shard_anatomies = result.shard_anatomies;
            session.voxel_size_um = result.voxel_size_um;

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

fn load_graph_from_disk(path: PathBuf, level: EditorLevel, fs_cache: ProjectFsCache) -> LoadedGraph {
    let mut zones = Vec::new();
    let mut zone_ids = HashMap::new();
    let mut connections = Vec::new();
    let mut node_inputs = HashMap::new();
    let mut node_outputs = HashMap::new();
    let mut layout_cache = HashMap::new();
    let mut shard_anatomies = HashMap::new();
    let mut voxel_size_um = 25.0; // Значение по умолчанию
    let mut father_id = String::new();

    if let Ok(content) = layout_api::overlay_read_to_string(&path) {
        if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
             // Парсинг модели
             if let Some(id_table) = doc.get("model_id_v1") {
                father_id = id_table.get("id").and_then(|v| v.as_str()).unwrap_or("0000").to_string();
             }
             // Парсинг департамента (или зоны в симуляции)
             if let Some(id_table) = doc.get("depart_id_v1") {
                father_id = id_table.get("id").and_then(|v| v.as_str()).unwrap_or("0000").to_string();
             }

             if let Some(dept_arr) = doc.get("department").and_then(|v| v.as_array_of_tables()) {
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

             if let Some(zone_arr) = doc.get("zone").and_then(|v| v.as_array_of_tables()) {
                for table in zone_arr.iter() {
                    if let Some(name) = table.get("name").and_then(|v| v.as_str()) {
                        zones.push(name.to_string());
                        let id = table.get("shard_id_v1").and_then(|t| t.get("id")).and_then(|v| v.as_str()).unwrap_or("0000").to_string();
                        zone_ids.insert(name.to_string(), id);
                    }
                }
             }

             if let Some(conn_arr) = doc.get("connection").and_then(|v| v.as_array_of_tables()) {
                for table in conn_arr.iter() {
                    let from = table.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let to = table.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let m = table.get("output_matrix").and_then(|v| v.as_str()).unwrap_or("out").to_string();
                    let im = table.get("input_matrix").and_then(|v| v.as_str()).unwrap_or("in").to_string();
                    connections.push((from, m, to, im));
                }
             }

             // [DOD FIX] Читаем геометрию шардов в RAM
             let project_dir = path.parent().unwrap_or(std::path::Path::new("."));
             if let Ok(content) = layout_api::overlay_read_to_string(&project_dir.join("simulation.toml")) {
                 if let Ok(doc_sim) = content.parse::<toml_edit::DocumentMut>() {
                     if let Some(sim) = doc_sim.get("simulation").and_then(|i| i.as_table()) {
                         voxel_size_um = sim.get("voxel_size_um").and_then(|v| v.as_float()).unwrap_or(25.0) as f32;
                     }
                 }
             }
             let path_str = path.to_string_lossy();
             let is_sim = path_str.ends_with("simulation.toml");
             let is_zone_level = path_str.ends_with("shard.toml") || path_str.ends_with("io.toml") || path_str.ends_with("blueprints.toml") || path_str.ends_with("anatomy.toml");
             let dept_name = path.file_name().unwrap_or_default().to_string_lossy().replace(".toml", "");

             for zone in &zones {
                 let mut shard_path = None;
                 let mut anatomy_path = None;

                 if let Some(shard_id) = zone_ids.get(zone) {
                     shard_path = find_path_by_id(&fs_cache, shard_id);
                     anatomy_path = find_path_by_id(&fs_cache, &format!("{}-ANATOMY", shard_id));
                 }

                 // DOD FIX: Безопасный фолбэк с выходом на уровень департамента, если ID еще не просканирован
                 let shard_path = shard_path.unwrap_or_else(|| {
                     let base = if is_sim {
                         project_dir.join(zone)
                     } else if is_zone_level {
                         project_dir.parent().unwrap_or(Path::new(".")).join(zone)
                     } else {
                         project_dir.join(&dept_name).join(zone)
                     };
                     base.join("shard.toml")
                 });

                 let anatomy_path = anatomy_path.unwrap_or_else(|| {
                     shard_path.parent().unwrap_or(Path::new(".")).join("anatomy.toml")
                 });

                 let mut w = 32.0; let mut d = 32.0; let mut h = 32.0;
                 if let Ok(content) = layout_api::overlay_read_to_string(&shard_path) {
                     if let Ok(doc_shard) = content.parse::<toml_edit::DocumentMut>() {
                         if let Some(dim) = doc_shard.get("dimensions").and_then(|i| i.as_table()) {
                             w = dim.get("w").and_then(|v| v.as_integer()).unwrap_or(32) as f32;
                             d = dim.get("d").and_then(|v| v.as_integer()).unwrap_or(32) as f32;
                             h = dim.get("h").and_then(|v| v.as_integer()).unwrap_or(32) as f32;
                         }
                     }
                 }

                 let mut layers = Vec::new();
                 if let Ok(content) = layout_api::overlay_read_to_string(&anatomy_path) {
                     match content.parse::<toml_edit::DocumentMut>() {
                         Ok(doc_anatomy) => {
                             if let Some(arr) = doc_anatomy.get("layer").and_then(|i| i.as_array_of_tables()) {
                                 for table in arr.iter() {
                                     let l_name = table.get("name").and_then(|v| v.as_str()).unwrap_or("Layer").to_string();
                                     let pct = table.get("height_pct").and_then(|v| Some(v.as_float().unwrap_or_else(|| v.as_integer().unwrap_or(1) as f64) as f32)).unwrap_or(1.0);
                                     layers.push(crate::domain::ShardLayer { name: l_name, height_pct: pct });
                                 }
                             } else if let Some(arr) = doc_anatomy.get("layer").and_then(|i| i.as_array()) {
                                 for val in arr.iter() {
                                     if let Some(table) = val.as_inline_table() {
                                         let l_name = table.get("name").and_then(|v| v.as_str()).unwrap_or("Layer").to_string();
                                         let pct = table.get("height_pct").and_then(|v| Some(v.as_float().unwrap_or_else(|| v.as_integer().unwrap_or(1) as f64) as f32)).unwrap_or(1.0);
                                         layers.push(crate::domain::ShardLayer { name: l_name, height_pct: pct });
                                     }
                                 }
                             }
                         }
                         Err(e) => {
                             error!("[Loader] anatomy.toml parse error in zone {}: {}", zone, e);
                         }
                     }
                 }
                 if layers.is_empty() { layers.push(crate::domain::ShardLayer { name: "Main".to_string(), height_pct: 1.0 }); }

                 shard_anatomies.insert(zone.clone(), crate::domain::ShardAnatomy { w, d, h, layers });
             }

             // Пытаемся загрузить лэйаут (через Overlay FS)
             let layout_path = path.parent().unwrap().join(format!("{}.layout.toml", path.file_name().unwrap().to_string_lossy().replace(".toml", "")));
             if let Ok(l_content) = layout_api::overlay_read_to_string(&layout_path) {
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
        project_name: path.components().nth(1).map(|c| c.as_os_str().to_string_lossy().into_owned()).unwrap_or_else(|| "Unknown".to_string()),
        file_path: path,
        father_id,
        zones,
        zone_ids,
        connections,
        node_inputs,
        node_outputs,
        layout_cache,
        shard_anatomies,
        voxel_size_um,
        level,
    }
}

fn clean_proj_name(name: &str) -> String {
    name.replace("Genesis-Models/", "").replace("/", "")
}
