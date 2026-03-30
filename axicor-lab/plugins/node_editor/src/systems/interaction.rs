use bevy::prelude::*;
use crate::domain::NodeGraphUiState;
use layout_api::{PluginWindow, ContextMenuActionTriggeredEvent, base_domain, DOMAIN_NODE_ED};

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

/// Обработка триггеров меню
pub fn handle_node_editor_menu_triggers_system(
    mut events: EventReader<ContextMenuActionTriggeredEvent>,
    mut query: Query<(Entity, &mut NodeGraphUiState)>,
    mut topo_events: EventWriter<crate::domain::TopologyMutation>,
    graph: Res<crate::domain::BrainTopologyGraph>,
) {
    for ev in events.read() {
        if !ev.action_id.starts_with("node_editor.") {
            continue;
        }

        let Ok((_entity, mut state)) = query.get_mut(ev.target_window) else {
            continue;
        };

        if ev.action_id.starts_with("node_editor.delete_node|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            if parts.len() < 2 { continue; }
            let node_name = parts[1].to_string();

            // Ищем ID в активной сессии
            if let Some(active_path) = &graph.active_path {
                if let Some(session) = graph.sessions.get(active_path) {
                    if let Some(node_id) = session.zone_ids.get(&node_name) {
                        topo_events.send(crate::domain::TopologyMutation::RemoveZone {
                            name: node_name,
                            id: node_id.clone(),
                            context_path: Some(active_path.clone()),
                        });
                        info!("Node Editor: Triggered RemoveZone for {}", node_id);
                    }
                }
            }
        } else if ev.action_id.starts_with("node_editor.start_rename|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            if parts.len() < 2 { continue; }
            let name = parts[1].to_string();

            state.renaming_zone = Some(name.clone());
            state.rename_buffer = name;
            info!("Node Editor: Started renaming node {}", parts[1]);
        } else if ev.action_id.starts_with("node_editor.add_node") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            let mut spawn_pos = bevy_egui::egui::pos2(0.0, 0.0);
            let mut has_pos = false;

            if parts.len() == 3 {
                if let (Ok(x), Ok(y)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>()) {
                    spawn_pos = bevy_egui::egui::pos2(x, y);
                    has_pos = true;
                }
            }

            let Some(active_path) = graph.active_path.clone() else { continue };
            if let Some(session) = graph.sessions.get(&active_path) {
                let path_str = active_path.to_string_lossy();
                if path_str.contains("simulation.toml") || path_str.ends_with(".toml") {
                    let prefix = if path_str.contains("simulation.toml") { "Zone_" } else { "Shard_" };

                    let mut next_idx = session.zones.len();
                    let mut name = format!("{}{}", prefix, next_idx);
                    while session.zones.contains(&name) {
                        next_idx += 1;
                        name = format!("{}{}", prefix, next_idx);
                    }

                    // Если координат в Intent нет (фоллбэк), используем каскадное смещение
                    if !has_pos {
                        let offset = session.zones.len() as f32 * 25.0;
                        spawn_pos = bevy_egui::egui::pos2(offset, offset);
                    }

                    topo_events.send(crate::domain::TopologyMutation::AddZone {
                        name: name.clone(),
                        pos: spawn_pos,
                    });

                    info!("Node Editor: Intent 'Add Node' -> TopologyMutation::AddZone(name: {}, pos: {:?})", name, spawn_pos);
                }
            }
        } else if ev.action_id == "node_editor.clear_graph" {
            // Используем state, который уже был безопасно захвачен в начале цикла
            state.show_clear_modal = true;
            info!("Node Editor: Opening Clear Graph modal");
        } else if !ev.action_id.starts_with("node_editor.delete_node|") && !ev.action_id.starts_with("node_editor.start_rename|") {
            warn!("Unknown node_editor action: {}", ev.action_id);
        }
    }
}

pub fn sync_smart_focus_system() {
    // Заглушка для соответствия lib.rs
}
