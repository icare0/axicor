use crate::domain::{BrainTopologyGraph, NodeGraphUiState, SaveProjectEvent};
use crate::systems::io::utils::flush_session_to_disk;
use bevy::prelude::*;

///         (.tmp).
///     SaveProjectEvent.
pub fn save_project_system(
    mut events: EventReader<SaveProjectEvent>,
    graph: Res<BrainTopologyGraph>,
    ui_state_query: Query<&NodeGraphUiState>,
) {
    for _ev in events.read() {
        let Some(active_path) = &graph.active_path else {
            continue;
        };
        let Some(session) = graph.sessions.get(active_path) else {
            continue;
        };

        //  UI-    (  )
        //  DOD      UI state,
        //  -          .
        //     .
        let ui_state = ui_state_query.iter().next();

        info!("[IO] Saving visual layout for {:?}", active_path);

        if let Err(e) = flush_session_to_disk(active_path, session, ui_state, true) {
            error!("[ERROR] [IO] Failed to save layout: {}", e);
        } else {
            info!("[OK] [IO] Visual layout saved to .layout.tmp.toml");
        }
    }
}

pub fn persist_blueprints_system(
    mut events: EventReader<crate::domain::TopologyMutation>,
    graph: Res<crate::domain::BrainTopologyGraph>,
) {
    for ev in events.read() {
        if let crate::domain::TopologyMutation::UpdateBlueprint { zone, context_path } = ev {
            let active_path = context_path
                .as_ref()
                .unwrap_or_else(|| graph.active_path.as_ref().unwrap());
            let Some(session) = graph.sessions.get(active_path) else {
                continue;
            };
            let Some(blueprint) = session.shard_blueprints.get(zone) else {
                continue;
            };

            let project_dir = active_path.parent().unwrap_or(std::path::Path::new("."));
            let path_str = active_path.to_string_lossy();
            let is_sim = path_str.ends_with("simulation.toml");
            let is_zone_level = path_str.ends_with("shard.toml")
                || path_str.ends_with("io.toml")
                || path_str.ends_with("blueprints.toml")
                || path_str.ends_with("anatomy.toml");
            let dept_name = active_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(".toml", "");

            let cold_path = if is_sim {
                project_dir.join(zone).join("blueprints.toml")
            } else if is_zone_level {
                project_dir.join("blueprints.toml")
            } else {
                project_dir
                    .join(&dept_name)
                    .join(zone)
                    .join("blueprints.toml")
            };

            let sandbox_path = layout_api::resolve_sandbox_path(&cold_path);
            if let Some(p) = sandbox_path.parent() {
                let _ = std::fs::create_dir_all(p);
            }

            // [DOD FIX]    toml_edit   [shard_id_v1]
            if let Ok(content) = layout_api::overlay_read_to_string(&cold_path) {
                if let Ok(mut doc) = content.parse::<toml_edit::DocumentMut>() {
                    // 1.     RAM
                    if let Ok(new_toml_str) = toml::to_string_pretty(blueprint) {
                        // 2.
                        if let Ok(new_doc) = new_toml_str.parse::<toml_edit::DocumentMut>() {
                            // 3.   [[neuron_type]]
                            if let Some(new_array) = new_doc.get("neuron_type") {
                                doc["neuron_type"] = new_array.clone();

                                // 4.    (   !)
                                if let Err(e) = std::fs::write(&sandbox_path, doc.to_string()) {
                                    error!(
                                        "[ERROR] [IO] Failed to save blueprints to sandbox: {}",
                                        e
                                    );
                                } else {
                                    info!(
                                        " [IO] Blueprints for {} safely auto-saved to {:?}",
                                        zone, sandbox_path
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn persist_io_system(
    mut events: EventReader<crate::domain::TopologyMutation>,
    graph: Res<crate::domain::BrainTopologyGraph>,
) {
    for ev in events.read() {
        if let crate::domain::TopologyMutation::UpdateIo { zone, context_path } = ev {
            let active_path = context_path
                .as_ref()
                .unwrap_or_else(|| graph.active_path.as_ref().unwrap());
            let Some(session) = graph.sessions.get(active_path) else {
                continue;
            };
            let Some(io_data) = session.shard_io.get(zone) else {
                continue;
            };

            let project_dir = active_path.parent().unwrap_or(std::path::Path::new("."));
            let path_str = active_path.to_string_lossy();
            let is_sim = path_str.ends_with("simulation.toml");
            let is_zone_level = path_str.ends_with("shard.toml")
                || path_str.ends_with("io.toml")
                || path_str.ends_with("blueprints.toml")
                || path_str.ends_with("anatomy.toml");
            let dept_name = active_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .replace(".toml", "");

            let cold_path = if is_sim {
                project_dir.join(zone).join("io.toml")
            } else if is_zone_level {
                project_dir.join("io.toml")
            } else {
                project_dir.join(&dept_name).join(zone).join("io.toml")
            };

            let sandbox_path = layout_api::resolve_sandbox_path(&cold_path);
            if let Some(p) = sandbox_path.parent() {
                let _ = std::fs::create_dir_all(p);
            }

            //   TOML ( io.toml   )
            if let Ok(toml_str) = toml::to_string_pretty(io_data) {
                if let Err(e) = std::fs::write(&sandbox_path, toml_str) {
                    error!("[ERROR] [IO] Failed to save io.toml to sandbox: {}", e);
                } else {
                    info!(
                        " [IO] IO Map for {} safely auto-saved to {:?}",
                        zone, sandbox_path
                    );
                }
            }
        }
    }
}
