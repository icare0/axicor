use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};
use genesis_core::config::brain::{ZoneEntry, ConnectionEntry};

pub fn apply_topology_mutations_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_state: ResMut<NodeGraphUiState>,
) {
    let Some(config) = &mut graph.config else { return; };

    for ev in events.read() {
        match ev {
            TopologyMutation::AddZone { name, pos } => {
                if config.zones.iter().any(|z| z.name == *name) { continue; }

                config.zones.push(ZoneEntry {
                    name: name.clone(),
                    blueprints: std::path::PathBuf::from(format!("{}/blueprints.toml", name)),
                    anatomy: std::path::PathBuf::from(format!("{}/anatomy.toml", name)),
                    io: std::path::PathBuf::from(format!("{}/io.toml", name)),
                    baked_dir: std::path::PathBuf::from(format!("baked/{}/", name)),
                });

                ui_state.node_positions.insert(name.clone(), *pos);
                info!("[NodeMutator] Zone added: {} at {:?}", name, pos);
            }
            TopologyMutation::AddConnection { from, to, .. } => {
                config.connections.push(ConnectionEntry {
                    from: from.clone(),
                    to: to.clone(),
                    axon_ids: vec![],
                    width: Some(16),
                    height: Some(16),
                });
                info!("[NodeMutator] Connection added: {} -> {}", from, to);
            }
        }
    }
}
