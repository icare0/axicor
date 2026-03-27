use bevy::prelude::*;
use std::path::PathBuf;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, LoadGraphEvent};
use bevy::utils::HashMap;
use genesis_core::config::brain::BrainConfig;

pub fn sync_topology_graph_system(
    mut events: EventReader<LoadGraphEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_state: ResMut<NodeGraphUiState>,
    mut bundle: ResMut<layout_api::ActiveBundle>,
) {
    for ev in events.read() {
        let axic_path = PathBuf::from("Genesis-Models").join(format!("{}.axic", ev.project_name));

        if bundle.load(&axic_path, &ev.project_name).is_none() { 
            error!("[NodeEditor] Failed to mmap archive: {:?}", axic_path);
            continue; 
        }

        if let Some(brain_data) = bundle.get_file("brain.toml") {
            let brain_str = std::str::from_utf8(brain_data).unwrap();
            
            if let Ok(config) = toml::from_str::<BrainConfig>(brain_str) {
                info!("[NodeEditor] Loaded Zero-Copy VFS topology for: {}", ev.project_name);
                
                let mut x = 100.0;
                let mut io_configs = HashMap::new();
                let mut anatomy_configs = HashMap::new();

                for zone in &config.zones {
                    let io_file = zone.io.file_name().unwrap_or_default().to_string_lossy();
                    let io_path = format!("{}/{}", zone.name, io_file);

                    if let Some(io_data) = bundle.get_file(&io_path) {
                        if let Ok(io_cfg) = toml::from_str(std::str::from_utf8(io_data).unwrap()) {
                            io_configs.insert(zone.name.clone(), io_cfg);
                        }
                    }

                    let anatomy_file = zone.anatomy.file_name().unwrap_or_default().to_string_lossy();
                    let anatomy_path = format!("{}/{}", zone.name, anatomy_file);

                    if let Some(anatomy_data) = bundle.get_file(&anatomy_path) {
                        if let Ok(anatomy_cfg) = toml::from_str(std::str::from_utf8(anatomy_data).unwrap()) {
                            anatomy_configs.insert(zone.name.clone(), anatomy_cfg);
                        }
                    }

                    ui_state.node_positions.entry(zone.name.clone()).or_insert_with(|| {
                        let pos = bevy_egui::egui::Pos2::new(x, 150.0);
                        x += 300.0;
                        pos
                    });
                }

                graph.active_project = Some(ev.project_name.clone());
                graph.active_path = Some(axic_path);
                graph.config = Some(config);
                graph.io_configs = io_configs;
                graph.anatomy_configs = anatomy_configs;
            }
        }
    }
}
