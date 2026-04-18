use bevy::prelude::*;
use crate::domain::NodeGraphUiState;
use layout_api::{PluginWindow, ContextMenuActionTriggeredEvent, OpenFileEvent, base_domain, DOMAIN_NODE_ED};

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

/// Context menu triggers for the Node Editor
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
            if parts.len() == 2 {
                let name = parts[1].to_string();
                let Some(active_path) = graph.active_path.clone() else { continue };
                let Some(session) = graph.sessions.get(&active_path) else { continue };
                let id = session.zone_ids.get(&name).cloned().unwrap_or_default();
                topo_events.send(crate::domain::TopologyMutation::Delete(
                    crate::domain::DeleteTarget::Zone { name, id },
                    Some(active_path),
                ));
            }
        } else if ev.action_id.starts_with("node_editor.start_rename|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            let name = parts[1].to_string();
            state.renaming_zone = Some(name.clone());
            state.rename_buffer = name;
            info!("Node Editor: Started renaming node {}", parts[1]);
        } else if ev.action_id.starts_with("node_editor.start_rename_port|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            if parts.len() == 4 {
                state.renaming_port = Some((parts[1].to_string(), parts[2] == "1", parts[3].to_string()));
                state.rename_buffer = parts[3].to_string();
            }
        } else if ev.action_id.starts_with("node_editor.delete_port|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            if parts.len() == 4 {
                topo_events.send(crate::domain::TopologyMutation::Delete(
                    crate::domain::DeleteTarget::IoPin {
                        zone: parts[1].to_string(),
                        is_input: parts[2] == "1",
                        name: parts[3].to_string(),
                    },
                    None,
                ));
            }
        } else if ev.action_id.starts_with("node_editor.add_") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            let action_type = parts[0];
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
                    let (prefix, existing_names) = match action_type {
                        "node_editor.add_env_rx" => ("Sensor_", &session.env_rx_nodes),
                        "node_editor.add_env_tx" => ("Motor_", &session.env_tx_nodes),
                        _ => if path_str.contains("simulation.toml") { ("Zone_", &session.zones) } else { ("Shard_", &session.zones) }
                    };

                    let mut next_idx = existing_names.len();
                    let mut name = format!("{}{}", prefix, next_idx);
                    while existing_names.contains(&name) {
                        next_idx += 1;
                        name = format!("{}{}", prefix, next_idx);
                    }

                    if !has_pos {
                        let offset = existing_names.len() as f32 * 25.0;
                        spawn_pos = bevy_egui::egui::pos2(offset, offset);
                    }

                    match action_type {
                        "node_editor.add_env_rx" => topo_events.send(crate::domain::TopologyMutation::Create(crate::domain::CreateTarget::EnvRx { name: name.clone(), pos: spawn_pos }, None)),
                        "node_editor.add_env_tx" => topo_events.send(crate::domain::TopologyMutation::Create(crate::domain::CreateTarget::EnvTx { name: name.clone(), pos: spawn_pos }, None)),
                        _ => topo_events.send(crate::domain::TopologyMutation::Create(crate::domain::CreateTarget::Zone { name: name.clone(), pos: spawn_pos }, None)),
                    };

                    info!("Node Editor: Intent '{}' -> Spawned {} at {:?}", action_type, name, spawn_pos);
                }
            }
        } else if ev.action_id.starts_with("node_editor.connect_matrix|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            // Format: node_editor.connect_matrix | src_zone | src_port | target_zone | target_port | voxel_z
            if parts.len() == 6 {
                let from = parts[1].to_string();
                let from_port = parts[2].to_string();
                let to = parts[3].to_string();
                let to_port = parts[4].to_string();
                let voxel_z = parts[5].parse::<u32>().ok();

                topo_events.send(crate::domain::TopologyMutation::Create(
                    crate::domain::CreateTarget::Connection {
                        from: from.clone(),
                        from_port: from_port.clone(),
                        to: to.clone(),
                        to_port: to_port.clone(),
                        voxel_z,
                    },
                    None,
                ));
                
                info!("Node Editor: DND matrix connected: {}.{} -> {}.{} (Z-Voxel: {:?})", from, from_port, to, to_port, voxel_z);
                //       AST  io.toml
            }
        } else if ev.action_id.starts_with("node_editor.connect_global|") {
            let parts: Vec<&str> = ev.action_id.split('|').collect();
            // Format: node_editor.connect_global|src_zone|src_port|target_zone
            if parts.len() == 4 {
                let from = parts[1].to_string();
                let from_port = parts[2].to_string();
                let to = parts[3].to_string();
                let to_port = "in".to_string(); //      

                topo_events.send(crate::domain::TopologyMutation::Create(
                    crate::domain::CreateTarget::Connection {
                        from: from.clone(),
                        from_port: from_port.clone(),
                        to: to.clone(),
                        to_port,
                        voxel_z: None,
                    },
                    None,
                ));
                
                info!("Node Editor: Global Atlas connection created: {}.{} -> {}", from, from_port, to);
            }
        } else if ev.action_id == "node_editor.clear_graph" {
            //  state,        
            state.show_clear_modal = true;
            info!("Node Editor: Opening Clear Graph modal");
        } else if !ev.action_id.starts_with("node_editor.delete_node|") && !ev.action_id.starts_with("node_editor.start_rename|") {
            warn!("Unknown node_editor action: {}", ev.action_id);
        }
    }
}

pub fn sync_smart_focus_system(
    mut open_events: EventReader<OpenFileEvent>,
    mut query: Query<&mut NodeGraphUiState>,
    graph: Res<crate::domain::BrainTopologyGraph>,
) {
    for ev in open_events.read() {
        let Some(active_path) = &graph.active_path else { continue };
        let Some(session) = graph.sessions.get(active_path) else { continue };

        let file_name = ev.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let node_name = if file_name == "shard.toml" || file_name == "io.toml" || file_name == "anatomy.toml" || file_name == "blueprints.toml" {
            ev.path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
        } else {
            ev.path.file_stem().and_then(|s| s.to_str()).unwrap_or("")
        };

        if let Some(id) = session.zone_ids.get(node_name) {
            for mut state in query.iter_mut() {
                state.selected_node_id = Some(id.clone());
            }
            info!("Smart Focus: Selected node {} ({}) based on file {:?}", node_name, id, ev.path);
        }
    }
}
