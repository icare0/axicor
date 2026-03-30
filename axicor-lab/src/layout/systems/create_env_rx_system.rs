use bevy::prelude::*;
use node_editor::domain::TopologyMutation;

pub fn create_env_rx_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
    mut ui_states: Query<&mut node_editor::domain::NodeGraphUiState>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };

    for ev in events.read() {
        if let TopologyMutation::AddEnvRx { name, pos } = ev {
            info!("[Orchestrator] Spawning World Node (Env RX): {}", name);

            if let Some(session) = graph.sessions.get_mut(&active_path) {
                session.env_rx_nodes.push(name.clone());
                // Env RX генерирует данные наружу, имеет только выход
                session.node_outputs.insert(name.clone(), vec!["out".to_string()]);
                session.is_dirty = true;
            }

            for mut ui in ui_states.iter_mut() {
                ui.node_positions.insert(name.clone(), *pos);
            }
        }
    }
}
