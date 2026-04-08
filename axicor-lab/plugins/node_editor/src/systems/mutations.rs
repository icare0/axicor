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
            TopologyMutation::Create(_, _) => {
                // [DOD FIX] RAM мутации для Create выполняются строго внутри `create_entity_system`,
                // так как для них требуется генерация уникальных Lineage ID (UUID), недоступных здесь.
            }

            TopologyMutation::Delete(target, context_path) => {
                let active_path = context_path.clone().or_else(|| graph.active_path.clone());
                let Some(active_path) = active_path else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };

                match target {
                    crate::domain::DeleteTarget::Zone { name, id } => {
                        session.zones.retain(|z| z != name);
                        session.env_rx_nodes.retain(|z| z != name);
                        session.env_tx_nodes.retain(|z| z != name);
                        session.connections.retain(|(f, _, t, _)| f != name && t != name);
                        session.node_inputs.remove(name.as_str());
                        session.node_outputs.remove(name.as_str());
                        session.zone_ids.remove(name.as_str());
                        session.layout_cache.remove(id.as_str());
                        for mut ui in ui_states.iter_mut() {
                            ui.node_positions.remove(name.as_str());
                            if ui.selected_node_id.as_ref() == Some(&id) { ui.selected_node_id = None; }
                        }
                        info!("[RAM Sync] Evicted node {} from session {:?}", name, active_path);
                    }
                    crate::domain::DeleteTarget::Connection { from, from_port, to, to_port } => {
                        session.connections.retain(|(f, fp, t, tp)| !(f == from && fp == from_port && t == to && tp == to_port));
                        info!("[RAM Sync] Severed connection {} -> {}", from, to);
                    }
                    crate::domain::DeleteTarget::IoPin { zone, is_input, name } => {
                        if *is_input {
                            if let Some(inputs) = session.node_inputs.get_mut(zone.as_str()) { inputs.retain(|p| p != name); }
                        } else {
                            if let Some(outputs) = session.node_outputs.get_mut(zone.as_str()) { outputs.retain(|p| p != name); }
                        }
                        session.connections.retain(|(f, fp, t, tp)| !((f == zone && fp == name && !*is_input) || (t == zone && tp == name && *is_input)));
                        info!("[RAM Sync] Removed I/O Matrix '{}' from zone '{}'", name, zone);
                    }
                    crate::domain::DeleteTarget::Layer { zone, name } => {
                        if let Some(anatomy) = session.shard_anatomies.get_mut(zone) {
                            let mut removed_pct = 0.0;
                            let mut found = false;
                            
                            if let Some(pos) = anatomy.layers.iter().position(|l| &l.name == name) {
                                removed_pct = anatomy.layers[pos].height_pct;
                                anatomy.layers.remove(pos);
                                found = true;
                            }

                            if found && removed_pct < 1.0 {
                                let divisor = 1.0 - removed_pct;
                                for layer in anatomy.layers.iter_mut() {
                                    layer.height_pct /= divisor;
                                }
                            }
                        }
                        info!("[RAM Sync] Removed Layer '{}' from zone '{}'", name, zone);
                    }
                }
                session.is_dirty = true;
            }

            TopologyMutation::Rename(target, context_path) => {
                let active_path = context_path.clone().or_else(|| graph.active_path.clone());
                let Some(active_path) = active_path else { continue };
                let Some(session) = graph.sessions.get_mut(&active_path) else { continue };

                match target {
                    crate::domain::RenameTarget::Shard { old_name, new_name, id: _id } => {
                        let mut found = false;
                        if let Some(pos) = session.zones.iter().position(|z| z == old_name) { session.zones[pos] = new_name.clone(); found = true; }
                        if let Some(pos) = session.env_rx_nodes.iter().position(|z| z == old_name) { session.env_rx_nodes[pos] = new_name.clone(); found = true; }
                        if let Some(pos) = session.env_tx_nodes.iter().position(|z| z == old_name) { session.env_tx_nodes[pos] = new_name.clone(); found = true; }

                        if !found { continue; }

                        if let Some(id_val) = session.zone_ids.remove(old_name) { session.zone_ids.insert(new_name.clone(), id_val); }
                        if let Some(inputs) = session.node_inputs.remove(old_name) { session.node_inputs.insert(new_name.clone(), inputs); }
                        if let Some(outputs) = session.node_outputs.remove(old_name) { session.node_outputs.insert(new_name.clone(), outputs); }

                        for conn in session.connections.iter_mut() {
                            if conn.0 == *old_name { conn.0 = new_name.clone(); }
                            if conn.2 == *old_name { conn.2 = new_name.clone(); }
                        }

                        for mut ui in ui_states.iter_mut() {
                            if let Some(pos) = ui.node_positions.remove(old_name) { ui.node_positions.insert(new_name.clone(), pos); }
                        }
                        session.is_dirty = true;
                        info!("[RAM Sync] Renamed Node {} to {}", old_name, new_name);
                    }
                    crate::domain::RenameTarget::IoPin { zone, is_input, old_name, new_name } => {
                        if *is_input {
                            if let Some(inputs) = session.node_inputs.get_mut(zone) {
                                if let Some(pos) = inputs.iter().position(|p| p == old_name) { inputs[pos] = new_name.clone(); }
                            }
                        } else {
                            if let Some(outputs) = session.node_outputs.get_mut(zone) {
                                if let Some(pos) = outputs.iter().position(|p| p == old_name) { outputs[pos] = new_name.clone(); }
                            }
                        }
                        for conn in session.connections.iter_mut() {
                            if conn.0 == *zone && conn.1 == *old_name && !*is_input { conn.1 = new_name.clone(); }
                            if conn.2 == *zone && conn.3 == *old_name && *is_input { conn.3 = new_name.clone(); }
                        }
                        session.is_dirty = true;
                        info!("[RAM Sync] Renamed IO Pin {} to {} in {}", old_name, new_name, zone);
                    }
                }
            }

            TopologyMutation::UpdateBlueprint { .. } | TopologyMutation::UpdateIo { .. } => {
                // RAM мутации для UpdateBlueprint и UpdateIo не требуются, так как редактирование идет In-Place в `ResMut<BrainTopologyGraph>`
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

pub fn hot_reload_io_system(
    mut events: EventReader<layout_api::TopologyChangedEvent>,
    mut graph: ResMut<crate::domain::BrainTopologyGraph>,
) {
    for ev in events.read() {
        // Извлекаем ключи заранее, чтобы обойти ограничения borrow checker'а
        let paths: Vec<std::path::PathBuf> = graph.sessions.keys().cloned().collect();
        
        for path in paths {
            if path.to_string_lossy().contains(&ev.project_name) {
                if let Some(session) = graph.sessions.get_mut(&path) {
                    // [DOD FIX] Сквозное обновление портов из .Sandbox в RAM
                    crate::systems::io::utils::sync_io_ports_from_disk(&path, session);
                    info!("[NodeEditor] Hot-reloaded I/O ports for session {:?}", path);
                }
            }
        }
    }
}
