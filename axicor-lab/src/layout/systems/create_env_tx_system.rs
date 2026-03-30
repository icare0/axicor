use bevy::prelude::*;
use node_editor::domain::TopologyMutation;

pub fn create_env_tx_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<node_editor::domain::BrainTopologyGraph>,
    mut ui_states: Query<&mut node_editor::domain::NodeGraphUiState>,
) {
    let active_path = graph.active_path.clone();
    let Some(active_path) = active_path else { return };

    for ev in events.read() {
        if let TopologyMutation::AddEnvTx { name, pos } = ev {
            info!("[Orchestrator] Spawning World Node (Env TX): {}", name);

            if let Some(session) = graph.sessions.get_mut(&active_path) {
                session.env_tx_nodes.push(name.clone());
                // Env TX принимает данные из шарда, имеет только вход
                session.node_inputs.insert(name.clone(), vec!["in".to_string()]);
                session.is_dirty = true;
            }

            for mut ui in ui_states.iter_mut() {
                ui.node_positions.insert(name.clone(), *pos);
            }
        }
    }
}
