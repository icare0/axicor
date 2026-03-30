use bevy::prelude::*;
use crate::domain::{BrainTopologyGraph, NodeGraphUiState, TopologyMutation};

/// Централизованная система применения мутаций топологии в RAM.
/// Обрабатывает добавление и удаление узлов, синхронизируя кэш сессий.
pub fn apply_topology_mutations_system(
    mut events: EventReader<TopologyMutation>,
    mut graph: ResMut<BrainTopologyGraph>,
    mut ui_states: Query<&mut NodeGraphUiState>,
) {
    for ev in events.read() {
        match ev {
            TopologyMutation::AddZone { .. } => {}
            TopologyMutation::AddEnvRx { .. } => {}
            TopologyMutation::AddEnvTx { .. } => {}
            TopologyMutation::RemoveConnection { .. } => {}

            TopologyMutation::AddIoMatrix { zone, is_input, name } => {
                let Some(active_path) = graph.active_path.clone() else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };
                
                if *is_input {
                    let inputs = session.node_inputs.entry(zone.clone()).or_default();
                    if !inputs.contains(name) { inputs.push(name.clone()); }
                } else {
                    let outputs = session.node_outputs.entry(zone.clone()).or_default();
                    if !outputs.contains(name) { outputs.push(name.clone()); }
                }
                session.is_dirty = true;
                info!("[RAM Sync] Added I/O Matrix '{}' to zone '{}'", name, zone);
            }

            TopologyMutation::AddConnection { from, from_port, to, to_port } => {
                let Some(active_path) = graph.active_path.clone() else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };

                let connection = (from.clone(), from_port.clone(), to.clone(), to_port.clone());
                if session.connections.contains(&connection) { continue; }

                session.connections.push(connection.clone());
                session.is_dirty = true;
                info!("[RAM Sync] Connection added: {}[{}] -> {}[{}]", from, from_port, to, to_port);
            }

            TopologyMutation::RemoveZone { name, id, context_path } => {
                // 1. Определяем целевой путь (из ивента или активный)
                let active_path = context_path.clone().or_else(|| graph.active_path.clone());
                let Some(active_path) = active_path else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };

                // 2. Cache Eviction (RAM Cleanup)
                session.zones.retain(|z| z != name);
                session.connections.retain(|(f, _, t, _)| f != name && t != name);
                session.node_inputs.remove(name);
                session.node_outputs.remove(name);
                session.zone_ids.remove(name);
                session.layout_cache.remove(id);
                session.is_dirty = true;

                // 3. Синхронизация UI-состояний (удаление визуальной ноды)
                for mut ui in ui_states.iter_mut() {
                    ui.node_positions.remove(name);
                    if ui.selected_node_id.as_ref() == Some(id) {
                        ui.selected_node_id = None;
                    }
                }

                info!("[RAM Sync] Evicted node {} (ID: {}) from session {:?}", name, id, active_path);
            }

            TopologyMutation::RenameZone { old_name, new_name, id, context_path } => {
                let active_path = context_path.clone().or_else(|| graph.active_path.clone());
                let Some(active_path) = active_path else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };

                if session.zones.contains(new_name) {
                    warn!("[RAM Sync] Cannot rename {} to {}: name already exists", old_name, new_name);
                    continue;
                }

                // 1. Обновляем основные структуры данных
                if let Some(pos) = session.zones.iter().position(|z| z == old_name) {
                    session.zones[pos] = new_name.clone();
                }

                if let Some(id_val) = session.zone_ids.remove(old_name) {
                    session.zone_ids.insert(new_name.clone(), id_val);
                }

                if let Some(inputs) = session.node_inputs.remove(old_name) {
                    session.node_inputs.insert(new_name.clone(), inputs);
                }

                if let Some(outputs) = session.node_outputs.remove(old_name) {
                    session.node_outputs.insert(new_name.clone(), outputs);
                }

                // 2. Каскадное обновление связей
                for conn in session.connections.iter_mut() {
                    if &conn.0 == old_name { conn.0 = new_name.clone(); }
                    if &conn.2 == old_name { conn.2 = new_name.clone(); }
                }

                // 3. Обновляем UI-позиции во всех окнах
                for mut ui in ui_states.iter_mut() {
                    if let Some(pos) = ui.node_positions.remove(old_name) {
                        ui.node_positions.insert(new_name.clone(), pos);
                    }
                }

                session.is_dirty = true;
                info!("[RAM Sync] Renamed node {} to {} (ID: {})", old_name, new_name, id);
            }
        }
    }
}

pub fn evict_deleted_entities_system(
    mut events: EventReader<layout_api::EntityDeletedEvent>,
    mut graph: ResMut<crate::domain::BrainTopologyGraph>,
    mut ui_states: Query<&mut crate::domain::NodeGraphUiState>,
) {
    for ev in events.read() {
        // 1. O(N) вычищение всех зомби-сессий дочерних файлов
        graph.sessions.retain(|path, _| !path.starts_with(&ev.path));

        // 2. Сброс активного пути, если он был удален
        if let Some(active) = &graph.active_path {
            if active.starts_with(&ev.path) {
                graph.active_path = None;
                graph.active_project = None;
            }
        }

        // 3. Зачистка UI-состояний
        if graph.active_path.is_none() {
            for mut ui in ui_states.iter_mut() {
                ui.node_positions.clear();
                ui.selected_node_id = None;
            }
        }
    }
}
