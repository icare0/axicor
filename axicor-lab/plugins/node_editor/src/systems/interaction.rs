use bevy::prelude::*;
use std::path::PathBuf;
use crate::domain::{LoadGraphEvent, BrainTopologyGraph, NodeGraphUiState, CompileGraphEvent, BakeProjectEvent, SaveProjectEvent};
use layout_api::{TopologyChangedEvent, PluginWindow, base_domain, DOMAIN_NODE_ED};

const MODELS_ROOT: &str = "Genesis-Models";

pub fn sync_topology_graph_system(
    mut load_ev: EventReader<LoadGraphEvent>,
    mut topo_ev: EventReader<TopologyChangedEvent>,
    mut open_file_ev: EventReader<layout_api::OpenFileEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
) {
    // Последнее событие побеждает — оба потока в одном fold
    let mut target = load_ev.read().map(|e| e.project_name.clone())
        .chain(topo_ev.read().map(|e| e.project_name.clone()))
        .last();

    // DOD FIX: Если пользователь открыл brain.toml в редакторе кода, 
    // автоматически подхватываем проект в Node Editor!
    for ev in open_file_ev.read() {
        if let Some(file_name) = ev.path.file_name().and_then(|n| n.to_str()) {
            if file_name == "brain.toml" {
                // Извлекаем имя проекта из пути: Genesis-Models/ProjectName/brain.toml
                let mut iter = ev.path.components();
                if let (Some(_), Some(proj_name)) = (iter.next(), iter.next()) {
                    target = Some(proj_name.as_os_str().to_string_lossy().into_owned());
                }
            }
        }
    }

    let Some(project) = target else { return };

    let brain_path = PathBuf::from(MODELS_ROOT).join(&project).join("brain.toml");

    graph.project_name = Some(project.clone());
    graph.zones.clear();
    graph.connections.clear();
    graph.node_inputs.clear();
    graph.node_outputs.clear();

    let content = match std::fs::read_to_string(&brain_path) {
        Ok(c)  => c,
        Err(e) => { warn!("[NodeEditor] Cannot read {:?}: {}", brain_path, e); return; }
    };

    let toml_val = match content.parse::<toml::Value>() {
        Ok(v)  => v,
        Err(e) => { warn!("[NodeEditor] TOML parse error in {:?}: {}", brain_path, e); return; }
    };

    parse_zones(&toml_val, &mut graph);
    parse_connections(&toml_val, &mut graph);

    info!("[NodeEditor] Synced '{}': {} zones, {} connections",
        project, graph.zones.len(), graph.connections.len());
}

pub fn init_node_editor_windows_system(
    mut commands: Commands,
    query: Query<(Entity, &PluginWindow), Added<PluginWindow>>,
) {
    for (entity, window) in query.iter() {
        if base_domain(&window.plugin_id) == DOMAIN_NODE_ED {
            commands.entity(entity).insert(NodeGraphUiState::default());
        }
    }
}

// ---------------------------------------------------------------------------

fn parse_zones(val: &toml::Value, graph: &mut BrainTopologyGraph) {
    let Some(zones) = val.get("zone").and_then(|v| v.as_array()) else { return };
    for z in zones {
        if let Some(name) = z.get("name").and_then(|n| n.as_str()) {
            let name_str = name.to_string();
            graph.zones.push(name_str.clone());

            // DOD FIX: Парсим реальные порты
            let mut ins = Vec::new();
            let mut outs = Vec::new();
            
            if let Some(project) = &graph.project_name {
                let io_path = PathBuf::from(MODELS_ROOT).join(project).join(&name_str).join("io.toml");
                if let Ok(content) = std::fs::read_to_string(&io_path) {
                    if let Ok(io_cfg) = genesis_core::config::io::IoConfig::parse(&content) {
                        ins = io_cfg.inputs.into_iter().map(|i| i.name).collect();
                        outs = io_cfg.outputs.into_iter().map(|o| o.name).collect();
                    }
                }
            }

            if ins.is_empty() { ins.push("in".to_string()); }
            if outs.is_empty() { outs.push("out".to_string()); }
            graph.node_inputs.insert(name_str.clone(), ins);
            graph.node_outputs.insert(name_str, outs);
        }
    }
}

fn parse_connections(val: &toml::Value, graph: &mut BrainTopologyGraph) {
    if let Some(connections) = val.get("connection").and_then(|v| v.as_array()) {
        for c in connections {
            let from = c.get("from").and_then(|n| n.as_str()).unwrap_or("");
            let to   = c.get("to").and_then(|n| n.as_str()).unwrap_or("");
            
            // DOD FIX: Читаем реальный выходной порт. Входной всегда "in", т.к. Ghost Axons просто входят в зону.
            let out_port = c.get("output_matrix").and_then(|n| n.as_str()).unwrap_or("out");
            let in_port = "in"; 
            
            if !from.is_empty() && !to.is_empty() {
                graph.connections.push((from.to_string(), out_port.to_string(), to.to_string(), in_port.to_string()));
            }
        }
    }
}

pub fn project_io_system(
    mut save_ev: EventReader<SaveProjectEvent>,
    mut compile_ev: EventReader<CompileGraphEvent>,
    mut bake_ev: EventReader<BakeProjectEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
) {
    let do_save = save_ev.read().next().is_some();
    let do_compile = compile_ev.read().next().is_some();
    let do_bake = bake_ev.read().next().is_some();

    if !do_save && !do_compile && !do_bake { return; }

    if let Some(proj_name) = &graph.project_name {
        let clean_proj_name = proj_name.replace(" (Source)", "").replace(" *", "");
        let base_dir = std::path::PathBuf::from("Genesis-Models").join(&clean_proj_name);

        // 1. SAVE: Точечное сохранение только ТЕКУЩЕГО уровня (brain.toml)
        if do_save && !do_compile && graph.is_dirty {
            println!("💾 [Node Editor] Saving current graph level to brain.toml...");
            let brain_toml_path = base_dir.join("brain.toml");
            
            if let Ok(content) = std::fs::read_to_string(&brain_toml_path) {
                if let Ok(mut brain_cfg) = genesis_core::config::brain::parse_brain_config_from_str(&content) {
                    brain_cfg.zones.clear();
                    for zone in &graph.zones {
                        brain_cfg.zones.push(genesis_core::config::brain::ZoneEntry {
                            name: zone.clone(),
                            blueprints: std::path::PathBuf::from(format!("{}/blueprints.toml", zone)),
                            anatomy: std::path::PathBuf::from(format!("{}/anatomy.toml", zone)),
                            io: std::path::PathBuf::from(format!("{}/io.toml", zone)),
                            baked_dir: std::path::PathBuf::from(format!("baked/{}/", zone)),
                        });
                    }

                    brain_cfg.connections.clear();
                    for (from, _out_port, to, _in_port) in &graph.connections {
                        brain_cfg.connections.push(genesis_core::config::brain::ConnectionEntry {
                            from: from.clone(),
                            to: to.clone(),
                            axon_ids: vec![],
                            width: Some(32),
                            height: Some(32),
                        });
                    }
                    let _ = std::fs::write(&brain_toml_path, toml::to_string(&brain_cfg).unwrap_or_default());
                }
            }
            graph.is_dirty = false;
        }

        // 2. COMPILE: Сохраняет ВСЕ файлы (создает папки, пишет дефолты и обновляет brain.toml)
        if do_compile && graph.is_dirty {
            println!("⚙️ [Node Editor] Compiling graph changes to disk...");
            
            for zone in &graph.zones {
                let zone_dir = base_dir.join(zone);
                if !zone_dir.exists() {
                    let _ = std::fs::create_dir_all(&zone_dir);
                    let _ = std::fs::write(zone_dir.join("anatomy.toml"), "[[layer]]\nname = \"Main\"\nheight_pct = 1.0\ndensity = 0.1\ncomposition = {}\n");
                    let _ = std::fs::write(zone_dir.join("blueprints.toml"), "[[neuron_type]]\nname = \"Default\"\nthreshold = 20000\nrest_potential = 0\nleak_rate = 100\nhomeostasis_penalty = 1000\nhomeostasis_decay = 10\nrefractory_period = 10\nsynapse_refractory_period = 10\nsignal_propagation_length = 5\n");
                    let _ = std::fs::write(zone_dir.join("shard.toml"), format!("zone_id = \"{}\"\n[dimensions]\nw = 32\nd = 32\nh = 32\n[world_offset]\nx = 0\ny = 0\nz = 0\n[settings]\nnight_interval_ticks = 10000\nprune_threshold = 15\nmax_sprouts = 16\nghost_capacity = 20000\n", zone));
                    let _ = std::fs::write(zone_dir.join("io.toml"), "");
                }
            }

            let brain_toml_path = base_dir.join("brain.toml");
            if let Ok(content) = std::fs::read_to_string(&brain_toml_path) {
                if let Ok(mut brain_cfg) = genesis_core::config::brain::parse_brain_config_from_str(&content) {
                    brain_cfg.zones.clear();
                    for zone in &graph.zones {
                        brain_cfg.zones.push(genesis_core::config::brain::ZoneEntry {
                            name: zone.clone(),
                            blueprints: std::path::PathBuf::from(format!("{}/blueprints.toml", zone)),
                            anatomy: std::path::PathBuf::from(format!("{}/anatomy.toml", zone)),
                            io: std::path::PathBuf::from(format!("{}/io.toml", zone)),
                            baked_dir: std::path::PathBuf::from(format!("baked/{}/", zone)),
                        });
                    }

                    brain_cfg.connections.clear();
                    for (from, _out_port, to, _in_port) in &graph.connections {
                        brain_cfg.connections.push(genesis_core::config::brain::ConnectionEntry {
                            from: from.clone(),
                            to: to.clone(),
                            axon_ids: vec![],
                            width: Some(32),
                            height: Some(32),
                        });
                    }
                    let _ = std::fs::write(&brain_toml_path, toml::to_string(&brain_cfg).unwrap_or_default());
                }
            }
            graph.is_dirty = false;
        }

        // 3. BAKE: Вызов genesis-baker для ТЕКУЩЕЙ модели
        if do_bake {
            let brain_toml_path = base_dir.join("brain.toml");
            println!("🔥 [Node Editor] Baking project: {:?}", brain_toml_path);
            
            let path_str = brain_toml_path.to_str().unwrap().to_string();
            std::thread::spawn(move || {
                let status = std::process::Command::new("cargo")
                    .args(["run", "--release", "-p", "genesis-baker", "--bin", "baker", "--", "--brain", &path_str])
                    .spawn()
                    .expect("Failed to spawn genesis-baker")
                    .wait()
                    .unwrap();

                if status.success() {
                    println!("✅ [Node Editor] Baking finished successfully!");
                } else {
                    eprintln!("❌ [Node Editor] Baking failed.");
                }
            });
        }
    }
}