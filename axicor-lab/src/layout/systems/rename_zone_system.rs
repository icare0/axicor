use crate::layout::systems::wm_file_ops::{load_document, save_document};
use bevy::prelude::*;
use node_editor::domain::{BrainTopologyGraph, RenameTarget, TopologyMutation};
use std::fs;
use std::path::Path;

/// -:      .
pub fn rename_zone_system(
    mut events: EventReader<TopologyMutation>,
    graph: Res<BrainTopologyGraph>,
) {
    for ev in events.read() {
        if let TopologyMutation::Rename(target, context_path) = ev {
            let target_path = context_path.as_ref().or(graph.active_path.as_ref());
            let Some(active_path) = target_path else {
                continue;
            };

            match target {
                RenameTarget::Shard {
                    old_name,
                    new_name,
                    id,
                } => rename_shard(active_path, old_name, new_name, id),
                RenameTarget::IoPin {
                    zone,
                    is_input,
                    old_name,
                    new_name,
                } => {
                    let path_str = active_path.to_string_lossy();
                    let is_sim = path_str.contains("simulation.toml");
                    let dept_name = active_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .replace(".toml", "");
                    let project_dir = active_path.parent().unwrap_or(Path::new("."));

                    let section = if *is_input { "input" } else { "output" };
                    let io_path = if is_sim {
                        project_dir.join(zone).join("io.toml")
                    } else {
                        project_dir.join(&dept_name).join(zone).join("io.toml")
                    };

                    if let Ok(mut doc) = load_document(&io_path) {
                        let mut renamed = false;
                        if let Some(arr) = doc
                            .get_mut(section)
                            .and_then(|i| i.as_array_of_tables_mut())
                        {
                            for table in arr.iter_mut() {
                                if table.get("name").and_then(|v| v.as_str())
                                    == Some(old_name.as_str())
                                {
                                    table.insert("name", toml_edit::value(new_name.clone()));
                                    renamed = true;
                                    break;
                                }
                            }
                        }
                        if renamed {
                            let _ = save_document(&io_path, &doc);
                            info!(
                                "[OK] [IO] Renamed pin {} -> {} in {:?}",
                                old_name, new_name, io_path
                            );
                        }
                    }
                }
            }
        }
    }
}

fn rename_shard(active_path: &Path, old_name: &str, new_name: &str, id: &str) {
    info!(
        " [Orchestrator] Renaming entity: {} -> {} (ID: {})",
        old_name, new_name, id
    );

    let is_sim = active_path.to_string_lossy().contains("simulation.toml");
    let table_name = if is_sim { "department" } else { "zone" };

    let mut doc = match load_document(active_path) {
        Ok(d) => d,
        Err(_) => {
            error!("[ERROR] [Orchestrator] Could not read config for renaming");
            return;
        }
    };

    let mut renamed = false;

    // 1.
    if let Some(arr) = doc
        .get_mut(table_name)
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for table in arr.iter_mut() {
            if table.get("name").and_then(|v| v.as_str()) == Some(old_name) {
                table.insert("name", toml_edit::value(new_name));
                renamed = true;
            }
        }
    }

    // 2. [DOD FIX]     (Connections)
    if let Some(arr) = doc
        .get_mut("connection")
        .and_then(|i| i.as_array_of_tables_mut())
    {
        for table in arr.iter_mut() {
            if table.get("from").and_then(|v| v.as_str()) == Some(old_name) {
                table.insert("from", toml_edit::value(new_name));
                renamed = true;
            }
            if table.get("to").and_then(|v| v.as_str()) == Some(old_name) {
                table.insert("to", toml_edit::value(new_name));
                renamed = true;
            }
        }
    }

    //
    if renamed {
        let _ = save_document(active_path, &doc);
        info!("[OK] [Orchestrator] Config updated with new name and connections.");
    }

    // 3.    (Cold Files)
    let project_dir = active_path.parent().unwrap_or(Path::new("."));
    if is_sim {
        let old_file = project_dir.join(format!("{}.toml", old_name));
        let new_file = project_dir.join(format!("{}.toml", new_name));
        let old_dir = project_dir.join(old_name);
        let new_dir = project_dir.join(new_name);
        if old_file.exists() {
            let _ = fs::rename(old_file, new_file);
        }
        if old_dir.exists() && old_dir.is_dir() {
            let _ = fs::rename(old_dir, new_dir);
        }
        info!("[OK] [Orchestrator] Department files and directories renamed.");
    } else {
        let dept_name = active_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace(".toml", "");
        let old_shard_dir = project_dir.join(&dept_name).join(old_name);
        let new_shard_dir = project_dir.join(&dept_name).join(new_name);
        if old_shard_dir.exists() && old_shard_dir.is_dir() {
            let _ = fs::rename(old_shard_dir, new_shard_dir);
            info!("[OK] [Orchestrator] Shard directory renamed.");
        }
    }
}
