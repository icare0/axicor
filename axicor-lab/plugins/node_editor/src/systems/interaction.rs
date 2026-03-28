use bevy::prelude::*;
use std::path::PathBuf;
use crate::domain::{LoadGraphEvent, BrainTopologyGraph, NodeGraphUiState};
use layout_api::{TopologyChangedEvent, PluginWindow, base_domain, DOMAIN_NODE_ED};

const MODELS_ROOT: &str = "Genesis-Models";

pub fn sync_topology_graph_system(
    mut load_ev: EventReader<LoadGraphEvent>,
    mut topo_ev: EventReader<TopologyChangedEvent>,
    mut graph: ResMut<BrainTopologyGraph>,
) {
    // Последнее событие побеждает — оба потока в одном fold
    let target = load_ev.read().map(|e| e.project_name.clone())
        .chain(topo_ev.read().map(|e| e.project_name.clone()))
        .last();

    let Some(project) = target else { return };

    let brain_path = PathBuf::from(MODELS_ROOT).join(&project).join("brain.toml");

    graph.project_name = Some(project.clone());
    graph.zones.clear();
    graph.connections.clear();

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
            graph.zones.push(name.to_string());
        }
    }
}

fn parse_connections(val: &toml::Value, graph: &mut BrainTopologyGraph) {
    let Some(conns) = val.get("connection").and_then(|v| v.as_array()) else { return };
    for c in conns {
        let from = c.get("from").and_then(|n| n.as_str()).unwrap_or("");
        let to   = c.get("to").and_then(|n| n.as_str()).unwrap_or("");
        if !from.is_empty() && !to.is_empty() {
            graph.connections.push((from.to_string(), to.to_string()));
        }
    }
}