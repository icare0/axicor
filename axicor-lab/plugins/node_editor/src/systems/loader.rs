use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, EditorLevel};

#[derive(Component)]
pub struct LoadGraphTask(Task<LoadedGraph>);

pub struct LoadedGraph {
    pub project_name: String,
    pub zones:        Vec<String>,
    pub connections:  Vec<(String, String, String, String)>,
    pub node_inputs:  HashMap<String, Vec<String>>,
    pub node_outputs: HashMap<String, Vec<String>>,
    pub level:        EditorLevel, // DOD FIX: Контекст уровня
}

pub fn spawn_load_task_system(
    mut events: EventReader<layout_api::OpenFileEvent>,
    mut commands: Commands,
) {
    for ev in events.read() {
        let path = ev.path.clone();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else { continue };

        // DOD FIX: Динамическое определение уровня
        let level = if file_name == "simulation.toml" || file_name.ends_with(".axic") {
            EditorLevel::Model
        } else if file_name.ends_with(".toml") && file_name != "manifest.toml" {
            EditorLevel::Department
        } else {
            continue;
        };

        info!("[NodeEditor] Spawning load task for: {:?} at level {:?}", path, level);
        let task = AsyncComputeTaskPool::get().spawn(async move {
            load_graph_from_disk(path, level)
        });
        commands.spawn(LoadGraphTask(task));
    }
}

pub fn apply_loaded_graph_system(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadGraphTask)>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_states: Query<&mut NodeGraphUiState>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        let Some(result) = future::block_on(future::poll_once(&mut task.0)) else { continue };

        graph.project_name = Some(result.project_name);
        graph.zones         = result.zones;
        graph.connections   = result.connections;
        graph.node_inputs   = result.node_inputs;
        graph.node_outputs  = result.node_outputs;
        graph.is_dirty      = false;

        for mut ui in ui_states.iter_mut() {
            ui.level = result.level.clone(); // DOD FIX: Используем уровень из результата таски
        }

        commands.entity(entity).despawn();
        info!("[NodeEditor] Graph loaded: {} zones, {} connections at level {:?}", 
            graph.zones.len(), graph.connections.len(), result.level);
    }
}

fn load_graph_from_disk(path: PathBuf, level: EditorLevel) -> LoadedGraph {
    let proj_dir = if path.is_file() { path.parent().unwrap().to_path_buf() } else { path.clone() };
    let proj_name = proj_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();

    let mut zones = Vec::new();
    let mut connections = Vec::new();
    let mut node_inputs = HashMap::new();
    let mut node_outputs = HashMap::new();

    if level == EditorLevel::Model {
        // --- LEVEL 0: СКАНИРУЕМ ДЕПАРТАМЕНТЫ (*.toml) ---
        if let Ok(entries) = std::fs::read_dir(&proj_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(".toml") && name != "simulation.toml" && name != "manifest.toml" {
                            let dept_name = name.replace(".toml", "");
                            zones.push(dept_name.clone());
                            // Пытаемся прочитать I/O из папки с таким же именем, если она есть
                            let (ins, outs) = load_zone(&proj_dir, &dept_name);
                            node_inputs.insert(dept_name.clone(), ins);
                            node_outputs.insert(dept_name, outs);
                        }
                    }
                }
            }
        }
    } else {
        // --- LEVEL 1: СКАНИРУЕМ ШАРДЫ (Папки) И ЧИТАЕМ СВЯЗИ ---
        if let Ok(entries) = std::fs::read_dir(&proj_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        if name == "baked" || name == "logs" || name == ".git" || name == "BrainDNA" { continue; }
                        zones.push(name.to_string());
                        let (ins, outs) = load_zone(&proj_dir, name);
                        node_inputs.insert(name.to_string(), ins);
                        node_outputs.insert(name.to_string(), outs);
                    }
                }
            }
        }

        // Читаем связи из открытого файла (например, brain.toml)
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(toml_val) = content.parse::<toml::Value>() {
                if let Some(conn_array) = toml_val.get("connection").and_then(|v| v.as_array()) {
                    for c in conn_array {
                        let from = c.get("from").and_then(|n| n.as_str()).unwrap_or("");
                        let to   = c.get("to").and_then(|n| n.as_str()).unwrap_or("");
                        let out_port = c.get("output_matrix").and_then(|n| n.as_str()).unwrap_or("out");
                        let in_port = "in";

                        if !from.is_empty() && !to.is_empty() {
                            connections.push((from.to_string(), out_port.to_string(), to.to_string(), in_port.to_string()));
                        }
                    }
                }
            }
        }
    }

    zones.sort();

    LoadedGraph {
        project_name: proj_name,
        zones,
        connections,
        node_inputs,
        node_outputs,
        level,
    }
}

fn load_zone(proj_dir: &Path, name: &str) -> (Vec<String>, Vec<String>) {
    let io_path = proj_dir.join(name).join("io.toml");
    let Ok(content) = std::fs::read_to_string(&io_path) else {
        return (vec!["in".to_string()], vec!["out".to_string()]);
    };
    let Ok(io_cfg) = genesis_core::config::io::IoConfig::parse(&content) else {
        return (vec!["in".to_string()], vec!["out".to_string()]);
    };
    
    let mut ins: Vec<String> = io_cfg.inputs.into_iter().map(|i| i.name).collect();
    let mut outs: Vec<String> = io_cfg.outputs.into_iter().map(|o| o.name).collect();

    // DOD FIX: Обязательный фолбэк! Без него ноды останутся без розеток, а связи не отрендерятся.
    if ins.is_empty() { ins.push("in".to_string()); }
    if outs.is_empty() { outs.push("out".to_string()); }

    (ins, outs)
}
