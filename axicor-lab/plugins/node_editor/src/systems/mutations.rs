use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};
use genesis_core::config::brain::{ZoneEntry, ConnectionEntry};

pub fn apply_topology_mutations_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_states: Query<&mut NodeGraphUiState>,
) {
    let Some(config) = &mut graph.config else { return };
    if events.is_empty() { return; }

    // Собираем ui_states один раз до цикла
    let mut ui_states: Vec<_> = ui_states.iter_mut().collect();

    for ev in events.read() {
        match ev {
            TopologyMutation::AddZone { name, pos } => {
                if config.zones.iter().any(|z| z.name == *name) { continue; }

                config.zones.push(ZoneEntry {
                    name:      name.clone(),
                    blueprints: zone_path(name, "blueprints.toml"),
                    anatomy:    zone_path(name, "anatomy.toml"),
                    io:         zone_path(name, "io.toml"),
                    baked_dir:  zone_path(name, "baked/"),
                });

                for ui in ui_states.iter_mut() {
                    ui.node_positions.insert(name.clone(), *pos);
                }

                info!("[NodeMutator] Zone added: {} at {:?}", name, pos);
            }

            TopologyMutation::AddConnection { from, to, .. } => {
                let exists = config.connections.iter()
                    .any(|c| c.from == *from && c.to == *to);
                if exists { continue; }

                config.connections.push(ConnectionEntry {
                    from: from.clone(),
                    to:   to.clone(),
                    axon_ids: vec![],
                    width:  Some(16),
                    height: Some(16),
                });

                info!("[NodeMutator] Connection added: {} -> {}", from, to);
            }
        }
    }
}

#[inline]
fn zone_path(name: &str, file: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}/{}", name, file))
}